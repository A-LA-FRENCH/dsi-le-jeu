//! Conditions data-driven : quand un événement peut arriver, quand un choix est
//! disponible, quand une techno produit un effet.

use crate::state::{GameState, Metric};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;

/// Accepte `"vmware"` comme `["vmware"]` dans les fichiers de données.
pub fn one_or_many<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(String),
        Many(Vec<String>),
    }
    Ok(match OneOrMany::deserialize(d)? {
        OneOrMany::One(s) => vec![s],
        OneOrMany::Many(v) => v,
    })
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Conditions {
    /// Seuils minimaux de métriques.
    #[serde(default)]
    pub min: BTreeMap<Metric, f32>,
    /// Seuils maximaux de métriques.
    #[serde(default)]
    pub max: BTreeMap<Metric, f32>,
    /// Technologies toutes requises (`"technology": "vmware"` accepté).
    #[serde(default, alias = "technology", deserialize_with = "one_or_many")]
    pub tech: Vec<String>,
    /// Au moins une de ces technologies.
    #[serde(default, deserialize_with = "one_or_many")]
    pub any_tech: Vec<String>,
    #[serde(default, deserialize_with = "one_or_many")]
    pub no_tech: Vec<String>,
    #[serde(default, deserialize_with = "one_or_many")]
    pub flags: Vec<String>,
    #[serde(default, deserialize_with = "one_or_many")]
    pub no_flags: Vec<String>,
    #[serde(default)]
    pub year_min: Option<u32>,
    #[serde(default)]
    pub year_max: Option<u32>,
    /// Projet en cours.
    #[serde(default, deserialize_with = "one_or_many")]
    pub project_active: Vec<String>,
    /// Projet terminé.
    #[serde(default, deserialize_with = "one_or_many")]
    pub project_done: Vec<String>,
    #[serde(default, deserialize_with = "one_or_many")]
    pub no_project: Vec<String>,
    /// Au moins un projet en cours.
    #[serde(default)]
    pub any_project: bool,
    /// Taille minimale de l'équipe interne recrutée.
    #[serde(default)]
    pub team_min: Option<usize>,
    /// Contrat encore actif.
    #[serde(default, deserialize_with = "one_or_many")]
    pub contract: Vec<String>,
    #[serde(default)]
    pub cloud: bool,
}

impl Conditions {
    pub fn is_empty(&self) -> bool {
        *self == Conditions::default()
    }

    pub fn met(&self, s: &GameState) -> bool {
        self.min.iter().all(|(m, v)| s.m(*m) >= *v)
            && self.max.iter().all(|(m, v)| s.m(*m) <= *v)
            && self.tech.iter().all(|t| s.has_tech(t))
            && (self.any_tech.is_empty() || self.any_tech.iter().any(|t| s.has_tech(t)))
            && self.no_tech.iter().all(|t| !s.has_tech(t))
            && self.flags.iter().all(|f| s.has_flag(f))
            && self.no_flags.iter().all(|f| !s.has_flag(f))
            && self.year_min.is_none_or(|y| s.cal.year >= y)
            && self.year_max.is_none_or(|y| s.cal.year <= y)
            && self
                .project_active
                .iter()
                .all(|p| s.projects.iter().any(|x| &x.id == p))
            && self
                .project_done
                .iter()
                .all(|p| s.done_projects.contains(p))
            && self
                .no_project
                .iter()
                .all(|p| !s.done_projects.contains(p) && !s.projects.iter().any(|x| &x.id == p))
            && (!self.any_project || !s.projects.is_empty())
            && self.team_min.is_none_or(|n| s.team.len() >= n)
            && self
                .contract
                .iter()
                .all(|c| s.contracts.iter().any(|x| &x.id == c && x.active))
            && (!self.cloud || s.cloud.active)
    }

    /// Phrase courte expliquant pourquoi un choix est verrouillé.
    pub fn explain(&self, s: &GameState, techs: &dyn Fn(&str) -> String) -> String {
        let mut out = Vec::new();
        for (m, v) in &self.min {
            if s.m(*m) < *v {
                out.push(format!("{} ≥ {}", m.label(), *v as i32));
            }
        }
        for (m, v) in &self.max {
            if s.m(*m) > *v {
                out.push(format!("{} ≤ {}", m.label(), *v as i32));
            }
        }
        for t in &self.tech {
            if !s.has_tech(t) {
                out.push(format!("requiert {}", techs(t)));
            }
        }
        if !self.any_tech.is_empty() && !self.any_tech.iter().any(|t| s.has_tech(t)) {
            out.push(format!(
                "requiert {}",
                self.any_tech
                    .iter()
                    .map(|t| techs(t))
                    .collect::<Vec<_>>()
                    .join(" ou ")
            ));
        }
        for t in &self.no_tech {
            if s.has_tech(t) {
                out.push(format!("incompatible avec {}", techs(t)));
            }
        }
        if let Some(n) = self.team_min
            && s.team.len() < n
        {
            out.push(format!("équipe interne ≥ {n}"));
        }
        if !self.project_done.is_empty()
            && !self
                .project_done
                .iter()
                .all(|p| s.done_projects.contains(p))
        {
            out.push("projet préalable non terminé".into());
        }
        if let Some(y) = self.year_min
            && s.cal.year < y
        {
            out.push(format!("à partir de l'année {y}"));
        }
        if out.is_empty() {
            "conditions non remplies".into()
        } else {
            out.join(", ")
        }
    }
}
