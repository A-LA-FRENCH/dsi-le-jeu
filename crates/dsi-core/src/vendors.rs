//! Fournisseurs (noms réels cités comme repères du marché ; situations fictives),
//! cartes de négociation et aléas.

use crate::conditions::Conditions;
use crate::effects::Effects;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VendorDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub category: String,
    /// Acteur français / européen.
    #[serde(default)]
    pub sovereign: bool,
    /// Le commercial (personnage générique).
    #[serde(default)]
    pub rep: String,
    /// Répliques du commercial pendant les négociations.
    #[serde(default)]
    pub lines: Vec<String>,
}

/// Une carte jouable pendant une négociation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CardDef {
    pub id: String,
    pub label: String,
    pub text: String,
    /// Variation du rapport de force.
    #[serde(default)]
    pub leverage: f32,
    /// Multiplie le levier par la crédibilité de la menace de départ (0–1).
    #[serde(default)]
    pub credibility_scaled: bool,
    /// Coût pour l'entreprise (métriques).
    #[serde(default)]
    pub effects: Effects,
    /// Tours de négociation consommés (1 par défaut).
    #[serde(default = "one")]
    pub rounds: u32,
    /// Réaction du commercial.
    #[serde(default)]
    pub reaction: String,
    /// Disponible seulement pour les contrats portant l'un de ces tags.
    #[serde(default)]
    pub only_tags: Vec<String>,
    /// Exige une alternative crédible (techno ou projet de migration).
    #[serde(default)]
    pub needs_alternative: bool,
    #[serde(default)]
    pub requires: Conditions,
    /// Effet spécial : exit_clause, term_5y, scope, audit.
    #[serde(default)]
    pub special: Option<String>,
}

fn one() -> u32 {
    1
}

/// Un aléa qui survient pendant la négociation (« Fin de trimestre commercial »).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct HazardDef {
    pub id: String,
    pub label: String,
    pub text: String,
    pub leverage: f32,
    #[serde(default = "weight")]
    pub weight: f32,
}

fn weight() -> f32 {
    1.0
}
