//! Intégration navigateur : prévient le chargeur (`web/loader.js`) quand le jeu est prêt.

use crate::world::Sprites;
use bevy::prelude::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(inline_js = "
export function dsi_ready() { window.dispatchEvent(new CustomEvent('dsi-ready')); }
export function dsi_progress(loaded, total) { window.dispatchEvent(new CustomEvent('dsi-progress', { detail: { loaded, total } })); }
")]
extern "C" {
    fn dsi_ready();
    fn dsi_progress(loaded: u32, total: u32);
}

pub struct WebPlugin;

impl Plugin for WebPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, signal_ready);
    }
}

#[derive(Default)]
struct ReadyState {
    done: bool,
    frames: u32,
    last: u32,
}

fn signal_ready(
    sprites: Option<Res<Sprites>>,
    server: Res<AssetServer>,
    mut st: Local<ReadyState>,
) {
    if st.done {
        return;
    }
    let Some(s) = sprites else { return };
    let ids = [
        s.sky.id(),
        s.building.id(),
        s.people.id(),
        s.fx.id(),
        s.portraits.id(),
        s.mentors.id(),
    ];
    let total = ids.len() as u32;
    let loaded = ids
        .iter()
        .filter(|id| server.is_loaded_with_dependencies(**id))
        .count() as u32;
    if loaded != st.last {
        st.last = loaded;
        dsi_progress(loaded, total);
    }
    if loaded == total {
        st.frames += 1;
        if st.frames >= 3 {
            st.done = true;
            dsi_ready();
        }
    }
}
