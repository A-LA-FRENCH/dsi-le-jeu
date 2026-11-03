//! Technologies (Technology Map) : chaque techno possédée a des effets mensuels,
//! éventuellement conditionnels. Exemple : Kubernetes augmente la vélocité si
//! l'organisation a les compétences, et la complexité sinon.

use crate::conditions::Conditions;
use crate::content::Content;
use crate::effects::{self, Effects};
use crate::state::GameState;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Passive {
    #[serde(default)]
    pub when: Conditions,
    #[serde(default)]
    pub then: Effects,
    #[serde(default, rename = "else")]
    pub otherwise: Effects,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TechDef {
    pub id: String,
    pub name: String,
    /// infra, dev, data, ai, identity, apps, culture
    pub branch: String,
    /// Colonne dans la Technology Map (0 = historique).
    #[serde(default)]
    pub tier: u32,
    pub desc: String,
    #[serde(default)]
    pub passive: Vec<Passive>,
    /// Possédée au début de la partie.
    #[serde(default)]
    pub initial: bool,
    /// Projet qui la débloque (pour l'affichage).
    #[serde(default)]
    pub via: Option<String>,
}

pub const BRANCHES: [(&str, &str); 7] = [
    ("infra", "Infrastructure"),
    ("dev", "Développement"),
    ("culture", "Culture engineering"),
    ("data", "Data"),
    ("ai", "IA"),
    ("identity", "Identité & sécurité"),
    ("apps", "Applications"),
];

/// Effets mensuels de toutes les technos possédées.
pub fn monthly(content: &Content, s: &mut GameState) {
    let owned: Vec<String> = s.techs.iter().cloned().collect();
    for id in owned {
        let Some(t) = content.tech(&id) else { continue };
        for p in &t.passive {
            let fx = if p.when.met(s) { &p.then } else { &p.otherwise };
            effects::apply(s, fx);
        }
    }
}
