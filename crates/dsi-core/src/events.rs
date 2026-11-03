//! Moteur d'événements data-driven : événements scriptés (date fixe), aléatoires
//! (pondérés par l'état du SI), enchaînés ou différés.

use crate::conditions::Conditions;
use crate::content::Content;
use crate::effects::{ChoiceDef, Effects};
use crate::state::{GameState, Inbox, Metric, Room};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Type de popup.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    #[default]
    Event,
    Vendor,
    Comex,
    Incident,
    Project,
    Recruitment,
    Contract,
    Architecture,
}

impl EventKind {
    pub fn label(self) -> &'static str {
        match self {
            EventKind::Event => "ÉVÉNEMENT",
            EventKind::Vendor => "VENDOR MEETING",
            EventKind::Comex => "COMEX MEETING",
            EventKind::Incident => "INCIDENT",
            EventKind::Project => "PROJECT DECISION",
            EventKind::Recruitment => "RECRUTEMENT",
            EventKind::Contract => "CONTRACT RENEWAL",
            EventKind::Architecture => "ARCHITECTURE DECISION",
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct At {
    pub year: u32,
    pub month: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EventDef {
    pub id: String,
    #[serde(default)]
    pub kind: EventKind,
    #[serde(default)]
    pub room: Room,
    /// Identifiant de personnage (dialogues.json) ou vide.
    #[serde(default)]
    pub speaker: String,
    pub title: String,
    #[serde(alias = "description")]
    pub text: String,
    /// Événement scripté à une date fixe.
    #[serde(default)]
    pub at: Option<At>,
    /// Poids dans le tirage aléatoire (0 = jamais tiré au hasard).
    #[serde(default)]
    pub weight: f32,
    /// Le poids augmente avec ces métriques : poids × (1 + Σ coef × valeur/100).
    #[serde(default)]
    pub weight_by: BTreeMap<Metric, f32>,
    #[serde(default)]
    pub conditions: Conditions,
    /// Peut se reproduire (sinon une seule fois par partie).
    #[serde(default)]
    pub repeatable: bool,
    /// Délai minimal entre deux occurrences (mois).
    #[serde(default = "default_cooldown")]
    pub cooldown: u32,
    /// Effets appliqués dès que l'événement est ouvert.
    #[serde(default)]
    pub immediate: Effects,
    pub choices: Vec<ChoiceDef>,
}

fn default_cooldown() -> u32 {
    6
}

impl EventDef {
    pub fn random_weight(&self, s: &GameState) -> f32 {
        if self.weight <= 0.0 {
            return 0.0;
        }
        let mut w = self.weight;
        let mut k = 1.0;
        for (m, c) in &self.weight_by {
            k += c * s.m(*m) / 100.0;
        }
        w *= k.max(0.05);
        w
    }
}

/// Peut-on (re)déclencher cet événement maintenant ?
pub fn eligible(def: &EventDef, s: &GameState) -> bool {
    if let Some(last) = s.seen.get(&def.id) {
        if !def.repeatable {
            return false;
        }
        if s.turn() < last + def.cooldown {
            return false;
        }
    }
    let queued = s
        .inbox
        .iter()
        .any(|i| matches!(i, Inbox::Event { id } if id == &def.id));
    !queued && def.conditions.met(s)
}

/// Tire `n` événements aléatoires distincts parmi les éligibles.
pub fn draw_random(content: &Content, s: &mut GameState, n: usize) -> Vec<String> {
    let mut picked: Vec<String> = Vec::new();
    for _ in 0..n {
        let pool: Vec<(&EventDef, f32)> = content
            .events
            .iter()
            .filter(|e| e.at.is_none() && !picked.contains(&e.id) && eligible(e, s))
            .map(|e| (e, e.random_weight(s)))
            .filter(|(_, w)| *w > 0.0)
            .collect();
        let total: f32 = pool.iter().map(|(_, w)| w).sum();
        if total <= 0.0 {
            break;
        }
        let mut r = s.rng.random::<f32>() * total;
        for (e, w) in &pool {
            if r < *w {
                picked.push(e.id.clone());
                break;
            }
            r -= w;
        }
    }
    picked
}

/// Événements scriptés pour le mois courant.
pub fn scripted_now(content: &Content, s: &GameState) -> Vec<String> {
    content
        .events
        .iter()
        .filter(|e| {
            e.at.is_some_and(|a| a.year == s.cal.year && a.month == s.cal.month)
        })
        .filter(|e| eligible(e, s))
        .map(|e| e.id.clone())
        .collect()
}
