//! Contrats fournisseurs : coût mensuel, date de renouvellement, dépendance.
//! Le renouvellement arrive tout seul ; le négocier tôt coûte du temps d'agenda
//! mais donne du levier. Attendre J-2 coûte très cher.

use crate::events::At;
use crate::state::{Calendar, GameState};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ContractDef {
    pub id: String,
    pub vendor: String,
    pub label: String,
    /// Points de budget par mois.
    pub monthly_cost: f32,
    /// Durée d'un engagement (mois).
    #[serde(default = "twelve")]
    pub term: u32,
    /// Premier renouvellement.
    #[serde(default)]
    pub first_renewal: Option<At>,
    /// Hausse demandée par le fournisseur au renouvellement (0.6 = +60 %).
    #[serde(default)]
    pub increase: f32,
    /// Poids dans la dépendance fournisseurs (0–1).
    #[serde(default)]
    pub dependency: f32,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Technologies ou projets qui rendent une migration crédible.
    #[serde(default)]
    pub alternatives: Vec<String>,
    /// Projet de migration proposé au renouvellement.
    #[serde(default)]
    pub migration: Vec<String>,
    /// Actif dès le début de la partie.
    #[serde(default)]
    pub initial: bool,
    /// Texte d'ouverture du renouvellement.
    #[serde(default)]
    pub pitch: String,
}

fn twelve() -> u32 {
    12
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ContractState {
    pub id: String,
    pub active: bool,
    pub monthly_cost: f32,
    /// Mois absolu du prochain renouvellement.
    pub next_renewal: u32,
    /// Hausse déjà négociée en amont (appliquée au renouvellement).
    #[serde(default)]
    pub pre_deal: Option<f32>,
    #[serde(default)]
    pub exit_clause: bool,
    #[serde(default)]
    pub renewals: u32,
    /// Poids de dépendance (copié de la définition).
    #[serde(default)]
    pub dependency: f32,
}

impl ContractState {
    pub fn from_def(d: &ContractDef, now: u32) -> Self {
        let next = d
            .first_renewal
            .map(|a| {
                Calendar {
                    year: a.year,
                    month: a.month,
                }
                .abs()
            })
            .unwrap_or(now + d.term);
        Self {
            id: d.id.clone(),
            active: true,
            monthly_cost: d.monthly_cost,
            next_renewal: next,
            pre_deal: None,
            exit_clause: false,
            renewals: 0,
            dependency: d.dependency,
        }
    }

    /// Poids de dépendance (0–1), réduit par une clause de sortie.
    pub fn dependency_weight(&self) -> f32 {
        if self.exit_clause {
            self.dependency * 0.6
        } else {
            self.dependency
        }
    }

    pub fn months_left(&self, now: u32) -> i32 {
        self.next_renewal as i32 - now as i32
    }
}

/// Coût mensuel total des contrats actifs.
pub fn run_cost(s: &GameState) -> f32 {
    s.contracts
        .iter()
        .filter(|c| c.active)
        .map(|c| c.monthly_cost)
        .sum()
}

/// Nombre d'alternatives crédibles pour un contrat (technos possédées ou projets lancés/terminés).
pub fn alternatives(def: &ContractDef, s: &GameState) -> usize {
    def.alternatives
        .iter()
        .filter(|a| {
            s.has_tech(a) || s.done_projects.contains(*a) || s.projects.iter().any(|p| &p.id == *a)
        })
        .count()
}
