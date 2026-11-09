//! Sauvegarde : l'état complet sérialisé en JSON, versionné.
//! Le stockage (localStorage, fichier) est fourni par la crate `platform`.

use crate::state::GameState;
use serde::{Deserialize, Serialize};

pub const SAVE_VERSION: u32 = 1;
pub const AUTOSAVE_KEY: &str = "autosave";

#[derive(Serialize, Deserialize)]
pub struct SaveFile {
    pub version: u32,
    pub game: GameState,
    /// Horodatage de la sauvegarde (ms depuis l'époque Unix), 0 si inconnu.
    #[serde(default)]
    pub saved_at: u64,
}

/// Nombre d'emplacements de sauvegarde manuelle.
pub const SLOT_COUNT: usize = 3;

/// Clé de stockage de l'emplacement `i` (0, 1, 2).
pub fn slot_key(i: usize) -> String {
    format!("slot{}", i + 1)
}

/// Résumé d'une sauvegarde, pour la liste des emplacements.
#[derive(Clone, Debug, PartialEq)]
pub struct SlotInfo {
    pub name: String,
    pub company: String,
    pub date: String,
    pub score: u32,
    pub rank: String,
    pub status: String,
    pub saved_at: u64,
}

pub fn slot_info(s: &GameState, saved_at: u64) -> SlotInfo {
    let sc = crate::score::score(s);
    let status = if let Some(o) = s.over {
        format!("Défaite : {}", o.title())
    } else if let Some(e) = &s.ending {
        format!("Terminée : {}", e.title)
    } else if s.extension > 0 {
        "Saison 2 en cours".to_string()
    } else {
        "En cours".to_string()
    };
    SlotInfo {
        name: s.profile.name.clone(),
        company: s.profile.company.clone(),
        date: s.cal.label(),
        score: sc,
        rank: crate::score::rank(sc).to_string(),
        status,
        saved_at,
    }
}

pub fn to_json(s: &GameState) -> Result<String, String> {
    to_json_at(s, 0)
}

pub fn to_json_at(s: &GameState, saved_at: u64) -> Result<String, String> {
    serde_json::to_string(&SaveFile {
        version: SAVE_VERSION,
        game: s.clone(),
        saved_at,
    })
    .map_err(|e| e.to_string())
}

/// Lit une sauvegarde et son horodatage.
pub fn read(json: &str) -> Result<(GameState, u64), String> {
    let f: SaveFile =
        serde_json::from_str(json).map_err(|e| format!("sauvegarde illisible : {e}"))?;
    if f.version != SAVE_VERSION {
        return Err(format!(
            "version de sauvegarde {} incompatible (attendue {SAVE_VERSION})",
            f.version
        ));
    }
    Ok((f.game, f.saved_at))
}

pub fn from_json(json: &str) -> Result<GameState, String> {
    let f: SaveFile =
        serde_json::from_str(json).map_err(|e| format!("sauvegarde illisible : {e}"))?;
    if f.version != SAVE_VERSION {
        return Err(format!(
            "version de sauvegarde {} incompatible (attendue {SAVE_VERSION})",
            f.version
        ));
    }
    Ok(f.game)
}
