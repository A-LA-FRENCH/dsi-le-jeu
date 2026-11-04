//! WAR ROOM : gestion de crise cyber. Le joueur priorise isoler, investiguer,
//! patcher, restaurer, communiquer, appeler l'ANSSI, prévenir le COMEX.
//! Aucune méthode offensive : on joue la prévention, l'arbitrage et la réponse.

use crate::content::Content;
use crate::effects::{self, Effects};
use crate::state::{GameState, Metric, Resolution, Tone};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CrisisDef {
    pub id: String,
    pub title: String,
    pub intro: String,
    /// Progression de la menace par tour si rien n'est isolé.
    pub spread_rate: f32,
    #[serde(default)]
    pub initial_spread: f32,
    #[serde(default = "six")]
    pub hours: u32,
    /// Ce que l'investigation révèle.
    pub cause: String,
    /// Le patch existe-t-il ? (sinon : contournement)
    #[serde(default = "yes")]
    pub patch_available: bool,
}

fn six() -> u32 {
    6
}
fn yes() -> bool {
    true
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WarAction {
    Isolate,
    Investigate,
    Patch,
    Restore,
    Communicate,
    CallAnssi,
    InformComex,
}

impl WarAction {
    pub const ALL: [WarAction; 7] = [
        WarAction::Isolate,
        WarAction::Investigate,
        WarAction::Patch,
        WarAction::Restore,
        WarAction::Communicate,
        WarAction::CallAnssi,
        WarAction::InformComex,
    ];
    pub fn label(self) -> &'static str {
        match self {
            WarAction::Isolate => "Isoler",
            WarAction::Investigate => "Investiguer",
            WarAction::Patch => "Patcher",
            WarAction::Restore => "Restaurer",
            WarAction::Communicate => "Communiquer aux utilisateurs",
            WarAction::CallAnssi => "Appeler l'ANSSI / CERT",
            WarAction::InformComex => "Prévenir le COMEX",
        }
    }
    pub fn hint(self) -> &'static str {
        match self {
            WarAction::Isolate => "Coupe la propagation. Coupe aussi la production.",
            WarAction::Investigate => "Comprendre la cause avant d'agir. Rend le patch fiable.",
            WarAction::Patch => "Corrige la faille. Sans investigation, c'est un pari.",
            WarAction::Restore => "Remet en service. Si rien n'est isolé : réinfection.",
            WarAction::Communicate => "Les utilisateurs pardonnent mieux quand on leur parle.",
            WarAction::CallAnssi => "Expertise externe, propagation ralentie. Ça fait sérieux.",
            WarAction::InformComex => {
                "Ne pas le faire, c'est garantir qu'ils l'apprennent par LinkedIn."
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct WarRoom {
    pub scenario: String,
    pub hours_left: u32,
    pub spread: f32,
    pub contained: bool,
    pub investigated: bool,
    pub patched: bool,
    pub restored: bool,
    pub communicated: bool,
    pub anssi: bool,
    pub comex_informed: bool,
    pub uptime_hit: f32,
    pub log: Vec<String>,
}

pub fn start(content: &Content, s: &GameState, scenario: &str) -> Option<WarRoom> {
    let d = content.crisis(scenario)?;
    // Une bonne sécurité préalable ralentit tout.
    let initial = d.initial_spread + (60.0 - s.m(Metric::Security)).max(0.0) * 0.3;
    Some(WarRoom {
        scenario: scenario.into(),
        hours_left: d.hours,
        spread: initial.clamp(0.0, 90.0),
        contained: false,
        investigated: false,
        patched: false,
        restored: false,
        communicated: false,
        anssi: false,
        comex_informed: false,
        uptime_hit: 0.0,
        log: vec![d.intro.clone()],
    })
}

pub fn done(w: &WarRoom) -> bool {
    w.hours_left == 0 || w.spread >= 100.0 || (w.contained && w.patched && w.restored)
}

pub fn act(content: &Content, s: &mut GameState, w: &mut WarRoom, a: WarAction) {
    let Some(d) = content.crisis(&w.scenario).cloned() else {
        return;
    };
    if done(w) {
        return;
    }
    let line = match a {
        WarAction::Isolate => {
            if w.contained {
                "C'est déjà isolé. Quelqu'un débranche quand même un câble, par principe."
                    .to_string()
            } else {
                w.contained = true;
                w.uptime_hit += 8.0;
                "Segments isolés. La propagation s'arrête. Le standard explose : « Internet ne marche plus ».".to_string()
            }
        }
        WarAction::Investigate => {
            w.investigated = true;
            format!("Investigation : {}", d.cause)
        }
        WarAction::Patch => {
            let ok = w.investigated || s.rng.random::<f32>() < 0.4;
            if ok && d.patch_available {
                w.patched = true;
                "Correctif appliqué sur le bon périmètre. Le SOC respire.".to_string()
            } else if ok {
                w.patched = true;
                "Pas de patch éditeur : contournement appliqué. Ça tient. Probablement.".to_string()
            } else {
                "Patch appliqué… sur le mauvais serveur. Investiguer d'abord aurait aidé."
                    .to_string()
            }
        }
        WarAction::Restore => {
            if !w.contained {
                w.spread = (w.spread + 20.0).min(100.0);
                "Restauration lancée sans isolement : les machines restaurées sont réinfectées dans l'heure.".to_string()
            } else if s.m(Metric::Resilience) < 30.0 && s.rng.random::<f32>() < 0.5 {
                w.uptime_hit += 6.0;
                w.restored = true;
                "Les sauvegardes étaient… sur le même NAS. Restauration partielle depuis une copie de 2019.".to_string()
            } else {
                w.restored = true;
                w.uptime_hit = (w.uptime_hit - 6.0).max(0.0);
                "Restauration depuis les sauvegardes. Elles marchent. Quelqu'un pleure de joie dans l'open space.".to_string()
            }
        }
        WarAction::Communicate => {
            w.communicated = true;
            "Message aux utilisateurs : clair, honnête, sans le mot « incident mineur ». Ils apprécient.".to_string()
        }
        WarAction::CallAnssi => {
            w.anssi = true;
            w.spread = (w.spread - 15.0).max(0.0);
            "L'ANSSI / CERT est en ligne. Le vocabulaire de la salle de crise monte de deux niveaux.".to_string()
        }
        WarAction::InformComex => {
            w.comex_informed = true;
            "COMEX prévenu. La DG demande « on est dans les journaux ? ». Pas encore.".to_string()
        }
    };
    w.log.push(line);
    w.hours_left = w.hours_left.saturating_sub(1);
    if !w.contained {
        let rate = d.spread_rate * if w.anssi { 0.7 } else { 1.0 };
        w.spread = (w.spread + rate).min(100.0);
        if w.spread >= 100.0 {
            w.log.push("La menace s'est propagée à tout le SI.".into());
        }
    }
}

/// Clôture de crise : calcul des conséquences.
pub fn close(content: &Content, s: &mut GameState, w: &WarRoom) -> Resolution {
    let title = content
        .crisis(&w.scenario)
        .map(|c| c.title.clone())
        .unwrap_or_default();
    let mut fx = Effects::new();
    let mut notes = Vec::new();
    let total = w.spread >= 100.0;
    let clean = w.contained && w.patched && w.restored;
    fx.insert(
        Metric::Uptime,
        -(w.uptime_hit + w.spread * 0.25 + if w.restored { 0.0 } else { 8.0 }),
    );
    fx.insert(
        Metric::Budget,
        -(3.0 + w.spread * 0.06 + if w.anssi { 0.0 } else { 1.0 }),
    );
    if total {
        fx.insert(Metric::Security, -20.0);
        fx.insert(Metric::Comex, -18.0);
        fx.insert(Metric::UserSat, -10.0);
        notes.push("Propagation totale : le SI est à reconstruire en partie.".into());
    } else if clean {
        fx.insert(Metric::Security, if w.investigated { 6.0 } else { 2.0 });
        fx.insert(Metric::Resilience, 3.0);
        notes.push(
            "Crise maîtrisée. Le retour d'expérience sera cité en exemple (en interne).".into(),
        );
    } else {
        fx.insert(Metric::Security, -6.0);
        notes.push("Crise incomplètement traitée : la cause rôde encore.".into());
        s.scheduled.push(crate::state::Scheduled {
            due: s.turn() + 4,
            event: None,
            effects: Effects::from([(Metric::Security, -5.0), (Metric::Uptime, -5.0)]),
            text: format!(
                "Réplique de la crise « {title} » : la faille n'avait pas été entièrement corrigée."
            ),
            origin: w.scenario.clone(),
        });
    }
    if w.comex_informed {
        *fx.entry(Metric::Comex).or_insert(0.0) += 3.0;
    } else {
        *fx.entry(Metric::Comex).or_insert(0.0) -= 12.0;
        notes.push("Le COMEX l'a appris par LinkedIn.".into());
    }
    if w.communicated {
        *fx.entry(Metric::UserSat).or_insert(0.0) += 3.0;
    } else {
        *fx.entry(Metric::UserSat).or_insert(0.0) -= 6.0;
        notes.push("Les utilisateurs ont appris la panne par la machine à café.".into());
    }
    if w.anssi {
        *fx.entry(Metric::Security).or_insert(0.0) += 2.0;
    }
    let deltas = effects::apply(s, &fx);
    s.push_log(
        format!(
            "Crise « {title} » clôturée — propagation {:.0} %.",
            w.spread
        ),
        if clean { Tone::Good } else { Tone::Bad },
    );
    s.decisions
        .push(format!("warroom:{}:{}", w.scenario, w.spread as i32));
    s.check_game_over();
    let text = if total {
        "La salle de crise ressemble à une scène de film catastrophe. Le PRA s'appelait « Gérard ». Gérard est en vacances.".to_string()
    } else if clean {
        "Isolé, compris, corrigé, restauré. Personne ne dormira cette nuit, mais demain le SI tourne.".to_string()
    } else {
        "Le SI repart, en boitant. On fera le post-mortem « quand on aura le temps ».".to_string()
    };
    Resolution {
        title: format!("WAR ROOM — {title}"),
        text,
        deltas,
        notes,
        speaker: String::new(), delayed_hint: !clean && !total,
    }
}
