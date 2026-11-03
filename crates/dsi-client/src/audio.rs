//! Audio procédural : de courtes séquences générées par WebAudio (oscillateurs),
//! sans aucun fichier son. Sur desktop, les sons sont ignorés.

use crate::sim::Settings;
use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    Click,
    Notify,
    Incident,
    Mail,
    Phone,
    Success,
    Fail,
    GameOver,
    Month,
}

#[derive(Resource, Default)]
pub struct Sfx(pub Vec<Cue>);

impl Sfx {
    pub fn push(&mut self, c: Cue) {
        if !self.0.contains(&c) {
            self.0.push(c);
        }
    }
}

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Sfx>().add_systems(Last, play);
    }
}

#[cfg(target_arch = "wasm32")]
mod js {
    use wasm_bindgen::prelude::*;
    #[wasm_bindgen(inline_js = r#"
let ctx = null;
function ac() {
  if (!ctx) {
    const C = window.AudioContext || window.webkitAudioContext;
    if (!C) return null;
    ctx = new C();
  }
  if (ctx.state === "suspended") ctx.resume();
  return ctx;
}
function tone(c, t0, freq, dur, type, vol, slide) {
  const o = c.createOscillator();
  const g = c.createGain();
  o.type = type;
  o.frequency.setValueAtTime(freq, t0);
  if (slide) o.frequency.exponentialRampToValueAtTime(slide, t0 + dur);
  g.gain.setValueAtTime(0.0001, t0);
  g.gain.exponentialRampToValueAtTime(vol, t0 + 0.01);
  g.gain.exponentialRampToValueAtTime(0.0001, t0 + dur);
  o.connect(g).connect(c.destination);
  o.start(t0);
  o.stop(t0 + dur + 0.02);
}
export function dsi_sfx(kind) {
  const c = ac();
  if (!c) return;
  const t = c.currentTime + 0.01;
  switch (kind) {
    case 0: tone(c, t, 880, 0.04, "square", 0.03); break;                                   // clic
    case 1: tone(c, t, 660, 0.09, "sine", 0.12); tone(c, t + 0.1, 990, 0.14, "sine", 0.12); break; // notification
    case 2: for (let i = 0; i < 3; i++) { tone(c, t + i * 0.28, 520, 0.22, "sawtooth", 0.07, 820); } break; // incident
    case 3: tone(c, t, 1200, 0.05, "triangle", 0.1); tone(c, t + 0.06, 1500, 0.07, "triangle", 0.08); break; // e-mail
    case 4: for (let i = 0; i < 2; i++) { for (let k = 0; k < 6; k++) { tone(c, t + i * 0.45 + k * 0.05, k % 2 ? 1300 : 1050, 0.05, "square", 0.04); } } break; // téléphone
    case 5: [523, 659, 784, 1046].forEach((f, i) => tone(c, t + i * 0.08, f, 0.16, "triangle", 0.1)); break; // réussite
    case 6: tone(c, t, 330, 0.18, "square", 0.05, 220); break;                               // échec
    case 7: [392, 370, 349, 262].forEach((f, i) => tone(c, t + i * 0.32, f, i === 3 ? 0.9 : 0.3, "sawtooth", 0.06, i === 3 ? 180 : null)); break; // game over
    case 8: tone(c, t, 200, 0.12, "triangle", 0.08, 400); tone(c, t + 0.12, 300, 0.1, "triangle", 0.06); break; // nouveau mois
  }
}
"#)]
    extern "C" {
        pub fn dsi_sfx(kind: u32);
    }
}

fn play(mut sfx: ResMut<Sfx>, settings: Res<Settings>) {
    if sfx.0.is_empty() {
        return;
    }
    let cues: Vec<Cue> = std::mem::take(&mut sfx.0);
    if !settings.sound {
        return;
    }
    // Un seul son significatif par image : le plus important.
    let order = [
        Cue::GameOver,
        Cue::Incident,
        Cue::Phone,
        Cue::Success,
        Cue::Fail,
        Cue::Month,
        Cue::Notify,
        Cue::Mail,
        Cue::Click,
    ];
    let Some(c) = order.iter().find(|c| cues.contains(c)) else {
        return;
    };
    let _kind = match c {
        Cue::Click => 0,
        Cue::Notify => 1,
        Cue::Incident => 2,
        Cue::Mail => 3,
        Cue::Phone => 4,
        Cue::Success => 5,
        Cue::Fail => 6,
        Cue::GameOver => 7,
        Cue::Month => 8,
    };
    #[cfg(target_arch = "wasm32")]
    js::dsi_sfx(_kind);
}
