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
}

pub fn to_json(s: &GameState) -> Result<String, String> {
    serde_json::to_string(&SaveFile {
        version: SAVE_VERSION,
        game: s.clone(),
    })
    .map_err(|e| e.to_string())
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
