//! Projets multi-mois : coût de lancement, coût mensuel, durée, complexité,
//! risque de glissement, effets pendant et après, technologies débloquées.

use crate::conditions::Conditions;
use crate::content::Content;
use crate::effects::{self, Effects};
use crate::state::{GameState, Metric, Tone};
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Nombre maximal de projets en parallèle.
pub const MAX_PARALLEL: usize = 5;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProjectDef {
    pub id: String,
    pub name: String,
    pub desc: String,
    #[serde(default)]
    pub category: String,
    /// Coût de lancement (points de budget).
    #[serde(default)]
    pub cost: f32,
    /// Coût mensuel pendant le projet.
    #[serde(default)]
    pub burn: f32,
    /// Durée nominale (mois).
    pub duration: u32,
    /// 0–100 : ralentit le projet et augmente le risque.
    #[serde(default)]
    pub complexity: f32,
    /// Probabilité mensuelle de base de glisser d'un mois.
    #[serde(default)]
    pub risk: f32,
    #[serde(default)]
    pub requires: Conditions,
    /// Effets au lancement.
    #[serde(default)]
    pub on_start: Effects,
    /// Effets chaque mois pendant le projet (perturbation, conduite du changement).
    #[serde(default)]
    pub monthly: Effects,
    /// Effets à la livraison.
    #[serde(default)]
    pub on_done: Effects,
    #[serde(default)]
    pub add_tech: Vec<String>,
    #[serde(default)]
    pub remove_tech: Vec<String>,
    /// Contrats résiliés à la livraison.
    #[serde(default)]
    pub end_contract: Vec<String>,
    /// Contrats souscrits à la livraison.
    #[serde(default)]
    pub add_contract: Vec<String>,
    #[serde(default)]
    pub set_flag: Vec<String>,
    /// Événement déclenché à mi-parcours (chaîne narrative).
    #[serde(default)]
    pub midway_event: Option<String>,
    /// Événement déclenché à la livraison.
    #[serde(default)]
    pub done_event: Option<String>,
    #[serde(default)]
    pub done_text: String,
    /// Peut être proposé dans la salle projet (sinon seulement via un événement).
    #[serde(default = "yes")]
    pub catalog: bool,
    /// Active le cloud avec ce fournisseur à la livraison.
    #[serde(default)]
    pub cloud_provider: Option<String>,
    #[serde(default)]
    pub cloud_bill: f32,
}

fn yes() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProjectState {
    pub id: String,
    pub progress: f32,
    pub started: u32,
    pub slips: u32,
    /// Renfort ESN : plus rapide, plus cher, plus dépendant.
    #[serde(default)]
    pub esn: bool,
    #[serde(default)]
    pub midway_done: bool,
}

pub fn can_start(content: &Content, s: &GameState, id: &str) -> Result<(), String> {
    let Some(p) = content.project(id) else {
        return Err("projet inconnu".into());
    };
    if s.projects.iter().any(|x| x.id == id) {
        return Err("déjà en cours".into());
    }
    if s.done_projects.contains(id) {
        return Err("déjà livré".into());
    }
    if s.projects.len() >= MAX_PARALLEL {
        return Err(format!(
            "{MAX_PARALLEL} projets en parallèle maximum : l'organisation sature"
        ));
    }
    if !p.requires.met(s) {
        return Err(p.requires.explain(s, &|t| content.tech_name(t)));
    }
    if s.m(Metric::Budget) < p.cost {
        return Err("budget insuffisant".into());
    }
    Ok(())
}

pub fn start(content: &Content, s: &mut GameState, id: &str) -> Vec<(Metric, f32)> {
    let Some(p) = content.project(id).cloned() else {
        return vec![];
    };
    let mut fx = p.on_start.clone();
    *fx.entry(Metric::Budget).or_insert(0.0) -= p.cost;
    let d = effects::apply(s, &fx);
    s.projects.push(ProjectState {
        id: p.id.clone(),
        progress: 0.0,
        started: s.turn(),
        slips: 0,
        esn: false,
        midway_done: false,
    });
    s.push_log(format!("Projet lancé : {}", p.name), Tone::Story);
    s.decisions.push(format!("project:{}", p.id));
    d
}

/// Vitesse mensuelle d'un projet (1.0 = nominal).
pub fn speed(def: &ProjectDef, st: &ProjectState, s: &GameState) -> f32 {
    let mut v = 0.55
        + s.m(Metric::Make) / 160.0
        + s.m(Metric::Velocity) / 260.0
        + s.m(Metric::DevopsMaturity) / 400.0
        - def.complexity / 400.0
        - s.m(Metric::Complexity) / 500.0;
    if st.esn {
        v += 0.35;
    }
    v.clamp(0.25, 1.6)
}

pub fn burn(content: &Content, s: &GameState) -> f32 {
    s.projects
        .iter()
        .filter_map(|p| {
            content
                .project(&p.id)
                .map(|d| d.burn * if p.esn { 1.6 } else { 1.0 })
        })
        .sum()
}

/// Avance tous les projets d'un mois. Renvoie les événements à déclencher.
pub fn monthly(content: &Content, s: &mut GameState) -> Vec<String> {
    let mut events = Vec::new();
    let mut finished = Vec::new();
    let ids: Vec<String> = s.projects.iter().map(|p| p.id.clone()).collect();
    for id in ids {
        let Some(def) = content.project(&id).cloned() else {
            continue;
        };
        let st = s.projects.iter().find(|p| p.id == id).cloned().unwrap();
        effects::apply(s, &def.monthly);
        if st.esn {
            s.metrics.add(Metric::VendorDep, 0.6);
        }
        let mut step = speed(&def, &st, s);
        let p_slip = (def.risk
            * (1.0 + s.m(Metric::Complexity) / 100.0)
            * (1.2 - s.m(Metric::Make) / 120.0))
            .clamp(0.0, 0.6);
        let slipped = s.rng.random::<f32>() < p_slip;
        if slipped {
            step *= 0.2;
        }
        let midway_event = def.midway_event.clone();
        let p = s.projects.iter_mut().find(|p| p.id == id).unwrap();
        p.progress += step;
        if slipped {
            p.slips += 1;
        }
        let mut fire_mid = false;
        if !p.midway_done && p.progress >= def.duration as f32 / 2.0 {
            p.midway_done = true;
            fire_mid = true;
        }
        let done = p.progress >= def.duration as f32;
        if slipped {
            let txt = [
                "glisse d'un mois (« dépendance externe »)",
                "glisse : le jalon a été « re-baseliné »",
                "prend du retard : l'environnement de recette est down",
                "glisse : le sachant était en congé",
            ];
            let t = txt[s.rng.random_range(0..txt.len())];
            s.push_log(format!("Projet {} {t}.", def.name), Tone::Bad);
        }
        if fire_mid && let Some(e) = midway_event {
            events.push(e);
        }
        if done {
            finished.push(id.clone());
        }
    }
    for id in finished {
        if let Some(e) = complete(content, s, &id) {
            events.push(e);
        }
    }
    events
}

/// Livre un projet : effets, technologies, contrats. Renvoie l'événement de livraison éventuel.
pub fn complete(content: &Content, s: &mut GameState, id: &str) -> Option<String> {
    let def = content.project(id)?.clone();
    s.projects.retain(|p| p.id != id);
    s.done_projects.insert(id.to_string());
    effects::apply(s, &def.on_done);
    for t in &def.add_tech {
        s.techs.insert(t.clone());
    }
    for t in &def.remove_tech {
        s.techs.remove(t);
    }
    for c in &def.end_contract {
        if let Some(c) = s.contracts.iter_mut().find(|x| &x.id == c) {
            c.active = false;
        }
    }
    for c in &def.add_contract {
        crate::game::add_contract(content, s, c);
    }
    for f in &def.set_flag {
        s.flags.insert(f.clone());
    }
    if let Some(p) = &def.cloud_provider {
        s.cloud.activate(p);
    }
    s.cloud.bill += def.cloud_bill;
    let text = if def.done_text.is_empty() {
        format!("Projet livré : {}", def.name)
    } else {
        format!("{} — {}", def.name, def.done_text)
    };
    s.push_log(text, Tone::Good);
    def.done_event.clone()
}
