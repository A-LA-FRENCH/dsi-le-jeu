//! Pont entre Bevy et la simulation : contenu, partie en cours, sauvegarde
//! automatique, paramètres et paramètres d'URL.

use crate::audio::{Cue, Sfx};
use bevy::prelude::*;
use dsi_core::state::{Active, Inbox};
use dsi_core::{Command, Content, GameState};
use platform::Storage;
use platform::params::LaunchParams;
use serde::{Deserialize, Serialize};

#[derive(Resource)]
pub struct GameContent(pub Content);

/// La partie en cours (None sur l'écran titre).
#[derive(Resource, Default)]
pub struct Session {
    pub game: Option<GameState>,
    /// Incrémenté à chaque commande réussie : déclenche la sauvegarde automatique.
    pub rev: u64,
    pub saved_rev: u64,
    pub has_save: bool,
    pub params: LaunchParams,
    pub last_error: Option<String>,
    /// Animation de changement de mois (secondes restantes).
    pub month_flash: f32,
}

#[derive(Resource)]
pub struct Store(pub Box<dyn Storage>);

#[derive(Resource, Clone, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    pub ui_scale: f32,
    pub sound: bool,
    pub reduce_motion: bool,
    pub high_contrast: bool,
    #[serde(default)]
    pub seen_disclaimer: bool,
    #[serde(default = "yes")]
    pub music: bool,
    #[serde(default = "default_volume")]
    pub music_volume: f32,
}

fn yes() -> bool {
    true
}

fn default_volume() -> f32 {
    0.35
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            ui_scale: 1.0,
            sound: true,
            reduce_motion: false,
            high_contrast: false,
            seen_disclaimer: false,
            music: true,
            music_volume: 0.35,
        }
    }
}

pub const SETTINGS_KEY: &str = "settings";

pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        let content = Content::load().expect("contenu embarqué invalide");
        let mut store = Store(platform::default_storage());
        let settings: Settings = store
            .0
            .get(SETTINGS_KEY)
            .ok()
            .flatten()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let has_save = store
            .0
            .get(dsi_core::save::AUTOSAVE_KEY)
            .ok()
            .flatten()
            .is_some();
        let params = platform::params::current();
        let _ = &mut store;
        app.insert_resource(GameContent(content))
            .insert_resource(Session {
                has_save,
                params,
                ..default()
            })
            .insert_resource(store)
            .insert_resource(settings)
            .add_systems(Update, (autosave, save_settings, tick_flash));
    }
}

impl Session {
    pub fn new_game(&mut self, content: &Content, seed: u64, profile: dsi_core::Profile) {
        let mut g = dsi_core::new_game(content, seed, profile);
        g.debug = self.params.debug;
        self.game = Some(g);
        self.rev += 1;
        self.month_flash = 2.2;
    }

    pub fn load(&mut self, store: &Store) -> Result<(), String> {
        let json = store
            .0
            .get(dsi_core::save::AUTOSAVE_KEY)
            .map_err(|e| e.to_string())?
            .ok_or("aucune sauvegarde")?;
        let mut g = dsi_core::save::from_json(&json)?;
        g.debug = self.params.debug;
        self.game = Some(g);
        self.saved_rev = self.rev;
        self.month_flash = 1.6;
        Ok(())
    }

    /// Sauvegarde la partie en cours dans l'emplacement `i`.
    pub fn save_slot(&mut self, store: &mut Store, i: usize) -> Result<(), String> {
        let g = self.game.as_ref().ok_or("pas de partie en cours")?;
        let json = dsi_core::save::to_json_at(g, platform::clock::now_millis())?;
        store
            .0
            .set(&dsi_core::save::slot_key(i), &json)
            .map_err(|e| e.to_string())
    }

    /// Charge l'emplacement `i` ; il devient la partie en cours (et la sauvegarde automatique).
    pub fn load_slot(&mut self, store: &Store, i: usize) -> Result<(), String> {
        let json = store
            .0
            .get(&dsi_core::save::slot_key(i))
            .map_err(|e| e.to_string())?
            .ok_or("emplacement vide")?;
        let (mut g, _) = dsi_core::save::read(&json)?;
        g.debug = self.params.debug;
        self.game = Some(g);
        self.rev += 1;
        self.month_flash = 1.6;
        Ok(())
    }

    pub fn delete_slot(store: &mut Store, i: usize) -> Result<(), String> {
        store
            .0
            .remove(&dsi_core::save::slot_key(i))
            .map_err(|e| e.to_string())
    }

    /// Résumé des emplacements : `None` si vide, `Err` si illisible.
    pub fn slot_infos(store: &Store) -> Vec<Option<Result<dsi_core::save::SlotInfo, String>>> {
        (0..dsi_core::save::SLOT_COUNT)
            .map(|i| {
                let json = store.0.get(&dsi_core::save::slot_key(i)).ok().flatten()?;
                Some(dsi_core::save::read(&json).map(|(g, at)| dsi_core::save::slot_info(&g, at)))
            })
            .collect()
    }

    /// Exécute une commande, émet les sons correspondants. Renvoie l'erreur lisible.
    pub fn run(&mut self, content: &Content, sfx: &mut Sfx, cmd: Command) -> Result<(), String> {
        let Some(g) = self.game.as_mut() else {
            return Err("pas de partie".into());
        };
        let month_before = g.turn();
        let inbox_before = g.inbox.len();
        let was_over = g.over.is_some();
        let advance = matches!(cmd, Command::AdvanceMonth);
        let res = dsi_core::apply(content, g, cmd);
        match res {
            Ok(()) => {
                self.rev += 1;
                let g = self.game.as_ref().unwrap();
                if g.over.is_some() && !was_over {
                    sfx.push(Cue::GameOver);
                } else if g.turn() != month_before {
                    self.month_flash = if g.cal.month == 1 { 2.4 } else { 1.4 };
                    sfx.push(Cue::Month);
                    let incident = g.inbox.iter().any(|i| matches!(i, Inbox::Event { id } if content.event(id).is_some_and(|e| e.kind == dsi_core::events::EventKind::Incident)));
                    sfx.push(if incident { Cue::Incident } else { Cue::Notify });
                } else if advance && g.inbox.len() > inbox_before {
                    sfx.push(Cue::Notify);
                } else {
                    match &g.active {
                        Some(Active::Event { id }) => {
                            let kind = content.event(id).map(|e| e.kind).unwrap_or_default();
                            sfx.push(match kind {
                                dsi_core::events::EventKind::Incident => Cue::Incident,
                                dsi_core::events::EventKind::Vendor
                                | dsi_core::events::EventKind::Contract => Cue::Phone,
                                _ => Cue::Mail,
                            });
                        }
                        Some(Active::Renewal { .. }) | Some(Active::Negotiation(_)) => {
                            sfx.push(Cue::Phone)
                        }
                        Some(Active::WarRoom(_)) => sfx.push(Cue::Incident),
                        Some(Active::Result(r)) => {
                            let score: f32 = r
                                .deltas
                                .iter()
                                .map(|(m, d)| if m.inverted() { -d } else { *d })
                                .sum();
                            sfx.push(if score >= 0.0 {
                                Cue::Success
                            } else {
                                Cue::Fail
                            });
                        }
                        Some(Active::Review(_)) => sfx.push(Cue::Success),
                        _ => sfx.push(Cue::Click),
                    }
                }
                if g.ending.is_some() {
                    sfx.push(Cue::Success);
                }
                self.last_error = None;
                Ok(())
            }
            Err(e) => {
                sfx.push(Cue::Fail);
                Err(e.label())
            }
        }
    }
}

fn autosave(mut session: ResMut<Session>, mut store: ResMut<Store>) {
    if session.rev == session.saved_rev {
        return;
    }
    session.saved_rev = session.rev;
    let Some(g) = &session.game else { return };
    match dsi_core::save::to_json(g) {
        Ok(json) => {
            if let Err(e) = store.0.set(dsi_core::save::AUTOSAVE_KEY, &json) {
                session.last_error = Some(format!("Sauvegarde impossible : {e}"));
            } else {
                session.has_save = true;
            }
        }
        Err(e) => session.last_error = Some(e),
    }
}

fn save_settings(settings: Res<Settings>, mut store: ResMut<Store>) {
    if settings.is_changed()
        && !settings.is_added()
        && let Ok(json) = serde_json::to_string(&*settings)
    {
        let _ = store.0.set(SETTINGS_KEY, &json);
    }
}

fn tick_flash(time: Res<Time>, mut session: ResMut<Session>) {
    if session.month_flash > 0.0 {
        session.month_flash = (session.month_flash - time.delta_secs()).max(0.0);
    }
}
