//! Chargement du contenu data-driven (`data/*.json`, embarqués à la compilation).
//! Ajouter un événement = éditer `data/events.json`, sans toucher au moteur.

use crate::conditions::Conditions;
use crate::contracts::ContractDef;
use crate::effects::{ChoiceDef, Effects, Gamble, Outcome};
use crate::events::{At, EventDef};
use crate::people::Careers;
use crate::projects::ProjectDef;
use crate::score::YearDef;
use crate::security::CrisisDef;
use crate::state::Room;
use crate::technology::TechDef;
use crate::vendors::{CardDef, HazardDef, VendorDef};
use serde::{Deserialize, Serialize};

pub const EVENTS_JSON: &str = include_str!("../../../data/events.json");
pub const VENDORS_JSON: &str = include_str!("../../../data/vendors.json");
pub const TECHNOLOGIES_JSON: &str = include_str!("../../../data/technologies.json");
pub const PROJECTS_JSON: &str = include_str!("../../../data/projects.json");
pub const CAREERS_JSON: &str = include_str!("../../../data/careers.json");
pub const DIALOGUES_JSON: &str = include_str!("../../../data/dialogues.json");
pub const CRISES_JSON: &str = include_str!("../../../data/crises.json");
pub const MENTORS_JSON: &str = include_str!("../../../data/mentors.json");

/// Action optionnelle dans une salle : coûte du temps d'agenda.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ActionDef {
    pub id: String,
    pub room: Room,
    pub label: String,
    pub desc: String,
    #[serde(default = "one")]
    pub agenda: u32,
    #[serde(default)]
    pub requires: Conditions,
    /// Mois minimum entre deux utilisations.
    #[serde(default)]
    pub cooldown: u32,
    #[serde(flatten)]
    pub outcome: Outcome,
    #[serde(default)]
    pub gamble: Option<Gamble>,
}

fn one() -> u32 {
    1
}

impl ActionDef {
    pub fn as_choice(&self) -> ChoiceDef {
        ChoiceDef {
            label: self.label.clone(),
            hint: self.desc.clone(),
            requires: self.requires.clone(),
            agenda_cost: 0,
            outcome: self.outcome.clone(),
            gamble: self.gamble.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Character {
    pub id: String,
    pub name: String,
    pub role: String,
    /// Couleur d'accent (hex) pour le portrait et le nom.
    #[serde(default)]
    pub color: String,
    /// Index du portrait dans l'atlas.
    #[serde(default)]
    pub portrait: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Line {
    pub speaker: String,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Chapter {
    pub at: At,
    pub title: String,
    pub subtitle: String,
    pub lines: Vec<Line>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Background {
    pub id: String,
    pub label: String,
    pub desc: String,
    #[serde(default)]
    pub effects: Effects,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Dialogues {
    pub characters: Vec<Character>,
    pub chapters: Vec<Chapter>,
    pub years: Vec<YearDef>,
    pub backgrounds: Vec<Background>,
    /// Répliques mensuelles du DAF dans le fil.
    #[serde(default)]
    pub daf_lines: Vec<String>,
    /// Notifications d'ambiance (Teams, Outlook…).
    #[serde(default)]
    pub ambient: Vec<String>,
    pub disclaimer: String,
    /// Durée de la campagne de base (années). Chaque prolongation ajoute `extension_years`.
    #[serde(default = "three")]
    pub campaign_length: u32,
    #[serde(default = "three")]
    pub extension_years: u32,
}

#[derive(Deserialize)]
struct EventsFile {
    events: Vec<EventDef>,
}
#[derive(Deserialize)]
struct VendorsFile {
    vendors: Vec<VendorDef>,
    contracts: Vec<ContractDef>,
    cards: Vec<CardDef>,
    hazards: Vec<HazardDef>,
}
#[derive(Deserialize)]
struct TechFile {
    technologies: Vec<TechDef>,
}
#[derive(Deserialize)]
struct ProjectsFile {
    projects: Vec<ProjectDef>,
    actions: Vec<ActionDef>,
}
#[derive(Deserialize)]
struct MentorsFile {
    mentors: Vec<crate::mentors::MentorDef>,
}
#[derive(Deserialize)]
struct CrisesFile {
    crises: Vec<CrisisDef>,
}

#[derive(Clone, Debug)]
pub struct Content {
    pub events: Vec<EventDef>,
    pub vendors: Vec<VendorDef>,
    pub contracts: Vec<ContractDef>,
    pub cards: Vec<CardDef>,
    pub hazards: Vec<HazardDef>,
    pub techs: Vec<TechDef>,
    pub projects: Vec<ProjectDef>,
    pub actions: Vec<ActionDef>,
    pub careers: Careers,
    pub dialogues: Dialogues,
    pub crises: Vec<CrisisDef>,
    pub mentors: Vec<crate::mentors::MentorDef>,
}

fn parse<T: for<'de> Deserialize<'de>>(name: &str, s: &str) -> Result<T, String> {
    serde_json::from_str(s).map_err(|e| format!("data/{name}: {e}"))
}

impl Content {
    /// Le contenu embarqué dans le binaire.
    pub fn load() -> Result<Content, String> {
        Self::from_sources(
            EVENTS_JSON,
            VENDORS_JSON,
            TECHNOLOGIES_JSON,
            PROJECTS_JSON,
            CAREERS_JSON,
            DIALOGUES_JSON,
            CRISES_JSON,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_sources(
        events: &str,
        vendors: &str,
        techs: &str,
        projects: &str,
        careers: &str,
        dialogues: &str,
        crises: &str,
    ) -> Result<Content, String> {
        let e: EventsFile = parse("events.json", events)?;
        let v: VendorsFile = parse("vendors.json", vendors)?;
        let t: TechFile = parse("technologies.json", techs)?;
        let p: ProjectsFile = parse("projects.json", projects)?;
        let c: Careers = parse("careers.json", careers)?;
        let d: Dialogues = parse("dialogues.json", dialogues)?;
        let k: CrisesFile = parse("crises.json", crises)?;
        Ok(Content {
            events: e.events,
            vendors: v.vendors,
            contracts: v.contracts,
            cards: v.cards,
            hazards: v.hazards,
            techs: t.technologies,
            projects: p.projects,
            actions: p.actions,
            careers: c,
            dialogues: d,
            crises: k.crises,
            mentors: parse::<MentorsFile>("mentors.json", MENTORS_JSON)?.mentors,
        })
    }

    pub fn mentor(&self, id: &str) -> Option<&crate::mentors::MentorDef> {
        self.mentors.iter().find(|m| m.id == id)
    }

    pub fn event(&self, id: &str) -> Option<&EventDef> {
        self.events.iter().find(|e| e.id == id)
    }
    pub fn vendor(&self, id: &str) -> Option<&VendorDef> {
        self.vendors.iter().find(|e| e.id == id)
    }
    pub fn vendor_lines(&self, id: &str) -> Vec<String> {
        self.vendor(id).map(|v| v.lines.clone()).unwrap_or_default()
    }
    pub fn contract(&self, id: &str) -> Option<&ContractDef> {
        self.contracts.iter().find(|e| e.id == id)
    }
    pub fn card(&self, id: &str) -> Option<&CardDef> {
        self.cards.iter().find(|e| e.id == id)
    }
    pub fn tech(&self, id: &str) -> Option<&TechDef> {
        self.techs.iter().find(|e| e.id == id)
    }
    pub fn tech_name(&self, id: &str) -> String {
        self.tech(id)
            .map(|t| t.name.clone())
            .unwrap_or_else(|| id.to_string())
    }
    pub fn project(&self, id: &str) -> Option<&ProjectDef> {
        self.projects.iter().find(|e| e.id == id)
    }
    pub fn action(&self, id: &str) -> Option<&ActionDef> {
        self.actions.iter().find(|e| e.id == id)
    }
    pub fn crisis(&self, id: &str) -> Option<&CrisisDef> {
        self.crises.iter().find(|e| e.id == id)
    }
    pub fn character(&self, id: &str) -> Option<&Character> {
        self.dialogues.characters.iter().find(|e| e.id == id)
    }
    pub fn campaign_years(&self) -> u32 {
        self.dialogues
            .years
            .iter()
            .map(|y| y.year)
            .max()
            .unwrap_or(1)
    }

    /// Vérifie que toutes les références croisées existent. Utilisé par les tests.
    pub fn validate(&self) -> Vec<String> {
        let mut errs = Vec::new();
        let check_outcome = |ctx: &str, o: &Outcome, errs: &mut Vec<String>| {
            if let Some(p) = &o.start_project
                && self.project(p).is_none()
            {
                errs.push(format!("{ctx}: projet inconnu {p}"));
            }
            for t in o.add_tech.iter().chain(o.remove_tech.iter()) {
                if self.tech(t).is_none() {
                    errs.push(format!("{ctx}: techno inconnue {t}"));
                }
            }
            if let Some(c) = &o.chain
                && self.event(c).is_none()
            {
                errs.push(format!("{ctx}: événement enchaîné inconnu {c}"));
            }
            for d in &o.delayed {
                if let Some(e) = &d.event
                    && self.event(e).is_none()
                {
                    errs.push(format!("{ctx}: événement différé inconnu {e}"));
                }
            }
            for c in o.end_contract.iter().chain(o.add_contract.iter()) {
                if self.contract(c).is_none() {
                    errs.push(format!("{ctx}: contrat inconnu {c}"));
                }
            }
            if let Some(cc) = &o.contract_cost
                && self.contract(&cc.contract).is_none()
            {
                errs.push(format!("{ctx}: contrat inconnu {}", cc.contract));
            }
            if let Some(m) = &o.minigame {
                match m {
                    crate::effects::Minigame::Negotiation { contract }
                        if self.contract(contract).is_none() =>
                    {
                        errs.push(format!("{ctx}: négociation sur contrat inconnu {contract}"))
                    }
                    crate::effects::Minigame::Recruit { role }
                        if !self.careers.roles.iter().any(|r| &r.id == role) =>
                    {
                        errs.push(format!("{ctx}: rôle inconnu {role}"))
                    }
                    crate::effects::Minigame::WarRoom { scenario }
                        if self.crisis(scenario).is_none() =>
                    {
                        errs.push(format!("{ctx}: crise inconnue {scenario}"))
                    }
                    _ => {}
                }
            }
            if let Some(p) = &o.cloud_provider
                && self.vendor(p).is_none()
            {
                errs.push(format!("{ctx}: fournisseur cloud inconnu {p}"));
            }
        };
        let mut ids = std::collections::BTreeSet::new();
        for e in &self.events {
            if !ids.insert(e.id.clone()) {
                errs.push(format!("événement en double : {}", e.id));
            }
            if e.choices.is_empty() {
                errs.push(format!("{}: aucun choix", e.id));
            }
            if !e.speaker.is_empty() && self.character(&e.speaker).is_none() {
                errs.push(format!("{}: personnage inconnu {}", e.id, e.speaker));
            }
            for (i, c) in e.choices.iter().enumerate() {
                let ctx = format!("{}#{}", e.id, i);
                check_outcome(&ctx, &c.outcome, &mut errs);
                if let Some(g) = &c.gamble {
                    check_outcome(&ctx, &g.win, &mut errs);
                    check_outcome(&ctx, &g.lose, &mut errs);
                }
            }
        }
        for a in &self.actions {
            check_outcome(&format!("action {}", a.id), &a.outcome, &mut errs);
        }
        for p in &self.projects {
            for t in p.add_tech.iter().chain(p.remove_tech.iter()) {
                if self.tech(t).is_none() {
                    errs.push(format!("projet {}: techno inconnue {t}", p.id));
                }
            }
            for e in p.midway_event.iter().chain(p.done_event.iter()) {
                if self.event(e).is_none() {
                    errs.push(format!("projet {}: événement inconnu {e}", p.id));
                }
            }
            for c in p.end_contract.iter().chain(p.add_contract.iter()) {
                if self.contract(c).is_none() {
                    errs.push(format!("projet {}: contrat inconnu {c}", p.id));
                }
            }
            if let Some(v) = &p.cloud_provider
                && self.vendor(v).is_none()
            {
                errs.push(format!("projet {}: fournisseur cloud inconnu {v}", p.id));
            }
            let mut seen = std::collections::BTreeSet::new();
            for o in &p.options {
                let ctx = format!("projet {} / solution {}", p.id, o.id);
                if !seen.insert(&o.id) {
                    errs.push(format!("{ctx}: identifiant en double"));
                }
                if o.name.is_empty() || o.desc.is_empty() {
                    errs.push(format!("{ctx}: nom ou description manquant"));
                }
                for v in o.vendor.iter().chain(o.cloud_provider.iter()) {
                    if self.vendor(v).is_none() {
                        errs.push(format!("{ctx}: fournisseur inconnu {v}"));
                    }
                }
                for c in &o.add_contract {
                    if self.contract(c).is_none() {
                        errs.push(format!("{ctx}: contrat inconnu {c}"));
                    }
                }
            }
        }
        for c in &self.contracts {
            if self.vendor(&c.vendor).is_none() {
                errs.push(format!(
                    "contrat {}: fournisseur inconnu {}",
                    c.id, c.vendor
                ));
            }
            for p in &c.migration {
                if self.project(p).is_none() {
                    errs.push(format!("contrat {}: projet de migration inconnu {p}", c.id));
                }
            }
        }
        for ch in &self.dialogues.chapters {
            for l in &ch.lines {
                if l.speaker != "mentor" && self.character(&l.speaker).is_none() {
                    errs.push(format!(
                        "chapitre {}: personnage inconnu {}",
                        ch.title, l.speaker
                    ));
                }
            }
        }
        if self.mentors.len() != 3 {
            errs.push("il faut exactement 3 mentors".into());
        }
        for m in &self.mentors {
            if m.advice.last().is_none_or(|a| !a.conditions.is_empty()) {
                errs.push(format!(
                    "mentor {} : le dernier conseil doit être sans condition",
                    m.id
                ));
            }
            for (i, a) in m.advice.iter().enumerate() {
                check_outcome(&format!("mentor {}#{i}", m.id), &a.outcome, &mut errs);
            }
        }
        errs
    }
}

fn three() -> u32 {
    3
}
