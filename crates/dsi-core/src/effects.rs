//! Effets data-driven : variations de métriques, conséquences différées, paris
//! risqués, lancement de projets, mini-jeux.

use crate::conditions::{Conditions, one_or_many};
use crate::state::{GameState, Metric};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Variations de métriques : `{"budget": -5, "debt": 3}`.
pub type Effects = BTreeMap<Metric, f32>;

/// Applique des effets, renvoie les variations réellement appliquées (non nulles).
pub fn apply(s: &mut GameState, fx: &Effects) -> Vec<(Metric, f32)> {
    let mut out = Vec::new();
    for (m, d) in fx {
        // Rendements décroissants : l'excellence coûte plus cher à atteindre.
        let cur = s.m(*m);
        let improving_high = !m.inverted() && *d > 0.0 && cur > 72.0 && *m != Metric::Budget;
        let improving_low = m.inverted() && *d < 0.0 && cur < 28.0;
        let d = if improving_high || improving_low {
            d * 0.5
        } else {
            *d
        };
        let real = s.metrics.add(*m, d);
        if real.abs() > 0.05 {
            out.push((*m, real));
        }
    }
    out
}

/// Fusionne des variations (pour l'affichage d'un résultat).
pub fn merge(into: &mut Vec<(Metric, f32)>, more: Vec<(Metric, f32)>) {
    for (m, d) in more {
        if let Some(x) = into.iter_mut().find(|(k, _)| *k == m) {
            x.1 += d;
        } else {
            into.push((m, d));
        }
    }
    into.retain(|(_, d)| d.abs() > 0.05);
}

/// Conséquence différée : `{"in": 6, "chance": 0.5, "event": "incident_tests"}`.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Delayed {
    /// Dans combien de mois.
    #[serde(rename = "in")]
    pub months: u32,
    /// Aléa supplémentaire de 0 à `jitter` mois.
    #[serde(default)]
    pub jitter: u32,
    /// Probabilité que la conséquence arrive (1.0 par défaut).
    #[serde(default = "one")]
    pub chance: f32,
    /// La probabilité augmente avec ces métriques (coef × valeur/100).
    #[serde(default)]
    pub chance_by: BTreeMap<Metric, f32>,
    #[serde(default)]
    pub event: Option<String>,
    #[serde(default)]
    pub effects: Effects,
    #[serde(default)]
    pub text: String,
}

fn one() -> f32 {
    1.0
}

/// Mini-jeu déclenché par un choix.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Minigame {
    Negotiation { contract: String },
    Recruit { role: String },
    WarRoom { scenario: String },
}

/// Ce qui se passe quand on fait un choix (ou une action).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Outcome {
    #[serde(default)]
    pub effects: Effects,
    /// Texte de résultat (1–3 phrases).
    #[serde(default)]
    pub result: String,
    #[serde(default)]
    pub delayed: Vec<Delayed>,
    #[serde(default)]
    pub start_project: Option<String>,
    #[serde(default, deserialize_with = "one_or_many")]
    pub add_tech: Vec<String>,
    #[serde(default, deserialize_with = "one_or_many")]
    pub remove_tech: Vec<String>,
    #[serde(default, deserialize_with = "one_or_many")]
    pub set_flag: Vec<String>,
    #[serde(default, deserialize_with = "one_or_many")]
    pub clear_flag: Vec<String>,
    /// Événement enchaîné immédiatement (même mois).
    #[serde(default)]
    pub chain: Option<String>,
    #[serde(default)]
    pub minigame: Option<Minigame>,
    /// Variation permanente de l'allocation mensuelle (ex. -0.7 = budget coupé).
    #[serde(default)]
    pub allocation: f32,
    /// Créneaux d'agenda consommés ou rendus ce mois-ci.
    #[serde(default)]
    pub agenda: i32,
    /// Coût mensuel ajouté à un contrat (`{"contract": "vmware", "cost": 0.3}`).
    #[serde(default)]
    pub contract_cost: Option<ContractCost>,
    /// Résilie un contrat.
    #[serde(default, deserialize_with = "one_or_many")]
    pub end_contract: Vec<String>,
    /// Ajoute un contrat (nouveau fournisseur).
    #[serde(default, deserialize_with = "one_or_many")]
    pub add_contract: Vec<String>,
    /// Perd une personne de l'équipe (démission).
    #[serde(default)]
    pub lose_hire: bool,
    /// Active le cloud avec ce fournisseur.
    #[serde(default)]
    pub cloud_provider: Option<String>,
    /// Variation de la facture cloud mensuelle.
    #[serde(default)]
    pub cloud_bill: f32,
    /// Variation de la maturité FinOps.
    #[serde(default)]
    pub finops: f32,
    /// Lignes de code générées / comprises (vibe coding).
    #[serde(default)]
    pub code_generated: f32,
    #[serde(default)]
    pub code_understood: f32,
}

impl Outcome {
    /// Fusionne un autre résultat (issue d'un pari) dans celui-ci.
    pub fn absorb(&mut self, o: &Outcome) {
        for (m, v) in &o.effects {
            *self.effects.entry(*m).or_insert(0.0) += v;
        }
        self.delayed.extend(o.delayed.iter().cloned());
        self.add_tech.extend(o.add_tech.iter().cloned());
        self.remove_tech.extend(o.remove_tech.iter().cloned());
        self.set_flag.extend(o.set_flag.iter().cloned());
        self.clear_flag.extend(o.clear_flag.iter().cloned());
        self.end_contract.extend(o.end_contract.iter().cloned());
        self.add_contract.extend(o.add_contract.iter().cloned());
        if o.start_project.is_some() {
            self.start_project = o.start_project.clone();
        }
        if o.chain.is_some() {
            self.chain = o.chain.clone();
        }
        if o.minigame.is_some() {
            self.minigame = o.minigame.clone();
        }
        if o.contract_cost.is_some() {
            self.contract_cost = o.contract_cost.clone();
        }
        if o.cloud_provider.is_some() {
            self.cloud_provider = o.cloud_provider.clone();
        }
        self.allocation += o.allocation;
        self.agenda += o.agenda;
        self.lose_hire |= o.lose_hire;
        self.cloud_bill += o.cloud_bill;
        self.finops += o.finops;
        self.code_generated += o.code_generated;
        self.code_understood += o.code_understood;
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ContractCost {
    pub contract: String,
    pub cost: f32,
}

/// Un pari : `{"p": 0.5, "p_by": {"make": 0.4}, "win": {...}, "lose": {...}}`.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Gamble {
    pub p: f32,
    #[serde(default)]
    pub p_by: BTreeMap<Metric, f32>,
    #[serde(default)]
    pub win: Outcome,
    #[serde(default)]
    pub lose: Outcome,
}

impl Gamble {
    pub fn probability(&self, s: &GameState) -> f32 {
        let mut p = self.p;
        for (m, k) in &self.p_by {
            p += k * (s.m(*m) - 50.0) / 100.0;
        }
        p.clamp(0.05, 0.95)
    }
}

/// Un choix dans un événement.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ChoiceDef {
    pub label: String,
    /// Indice affiché sous le choix (« coûteux mais rassurant »…).
    #[serde(default)]
    pub hint: String,
    #[serde(default)]
    pub requires: Conditions,
    /// Créneaux d'agenda nécessaires (0 par défaut).
    #[serde(default)]
    pub agenda_cost: u32,
    #[serde(flatten)]
    pub outcome: Outcome,
    #[serde(default)]
    pub gamble: Option<Gamble>,
}

impl ChoiceDef {
    /// Métriques touchées immédiatement (pour les indices ▲▼ affichés au joueur).
    pub fn preview(&self) -> Vec<(Metric, f32)> {
        let mut v: Vec<(Metric, f32)> =
            self.outcome.effects.iter().map(|(m, d)| (*m, *d)).collect();
        if let Some(g) = &self.gamble {
            for (m, d) in g.win.effects.iter().chain(g.lose.effects.iter()) {
                if !v.iter().any(|(k, _)| k == m) {
                    v.push((*m, *d * 0.0));
                }
            }
        }
        v
    }

    pub fn has_hidden(&self) -> bool {
        !self.outcome.delayed.is_empty() || self.gamble.is_some() || self.outcome.minigame.is_some()
    }
}
