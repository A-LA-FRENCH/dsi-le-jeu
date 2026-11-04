//! Recrutement (mini-jeu d'entretien) et équipe interne. Chaque recrue fait monter
//! la capacité MAKE et une compétence précise, coûte chaque mois, et peut partir.

use crate::conditions::Conditions;
use crate::content::Content;
use crate::effects::{self, Effects};
use crate::state::{GameState, Metric, Resolution, Scheduled, Tone};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RoleDef {
    pub id: String,
    pub label: String,
    /// Coût mensuel (points de budget).
    pub salary: f32,
    #[serde(default)]
    pub on_hire: Effects,
    /// Effets mensuels tant que la personne est là.
    #[serde(default)]
    pub monthly: Effects,
    /// Plafond au-delà duquel les effets mensuels s'arrêtent.
    #[serde(default = "cap")]
    pub cap: f32,
    #[serde(default)]
    pub pitch: String,
    #[serde(default)]
    pub requires: Conditions,
}

fn cap() -> f32 {
    70.0
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AnswerDef {
    pub text: String,
    #[serde(default)]
    pub interest: f32,
    /// L'intérêt varie avec ces métriques : coef × (valeur − 50) / 5.
    #[serde(default)]
    pub interest_by: std::collections::BTreeMap<Metric, f32>,
    /// Nombre de jours de télétravail annoncés (comparé à la préférence du candidat).
    #[serde(default)]
    pub remote: Option<u32>,
    /// Réponse mensongère si ces conditions sont réunies (découverte après l'embauche).
    #[serde(default)]
    pub lie_if: Option<Conditions>,
    #[serde(default)]
    pub reaction: String,
    /// Le candidat quitte l'entretien.
    #[serde(default)]
    pub leave: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct QuestionDef {
    pub id: String,
    pub prompt: String,
    pub answers: Vec<AnswerDef>,
    /// Rôles concernés (vide = tous).
    #[serde(default)]
    pub roles: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OfferDef {
    pub label: String,
    pub interest: f32,
    pub salary_mult: f32,
    #[serde(default)]
    pub hint: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Careers {
    pub roles: Vec<RoleDef>,
    pub questions: Vec<QuestionDef>,
    pub offers: Vec<OfferDef>,
    pub first_names: Vec<String>,
    pub last_names: Vec<String>,
    #[serde(default)]
    pub quirks: Vec<String>,
}

/// Une personne recrutée en interne.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Hire {
    pub role: String,
    pub name: String,
    pub salary: f32,
    pub since: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Interview {
    pub role: String,
    pub name: String,
    pub quirk: String,
    pub interest: f32,
    pub remote_pref: u32,
    pub questions: Vec<String>,
    pub step: usize,
    pub lines: Vec<(bool, String)>,
    pub gone: bool,
    pub lied: bool,
}

pub fn start(content: &Content, s: &mut GameState, role: &str) -> Option<Interview> {
    let c = &content.careers;
    let r = c.roles.iter().find(|r| r.id == role)?;
    let first = &c.first_names[s.rng.random_range(0..c.first_names.len())];
    let last = &c.last_names[s.rng.random_range(0..c.last_names.len())];
    let quirk = if c.quirks.is_empty() {
        String::new()
    } else {
        c.quirks[s.rng.random_range(0..c.quirks.len())].clone()
    };
    let pool: Vec<&QuestionDef> = c
        .questions
        .iter()
        .filter(|q| q.roles.is_empty() || q.roles.iter().any(|x| x == role))
        .collect();
    let mut qs: Vec<String> = Vec::new();
    // La question Java tombe souvent : c'est le moment préféré des candidats.
    while qs.len() < 3 && qs.len() < pool.len() {
        let q = pool[s.rng.random_range(0..pool.len())];
        if !qs.contains(&q.id) {
            qs.push(q.id.clone());
        }
    }
    let interest = 30.0 + s.m(Metric::Attractiveness) * 0.4 + s.m(Metric::DevSat) * 0.1
        - s.m(Metric::Debt) * 0.1;
    let name = format!("{first} {last}");
    let lines = vec![(false, format!("{name}, {} — {}", r.label, r.pitch))];
    Some(Interview {
        role: role.into(),
        name,
        quirk,
        interest: interest.clamp(10.0, 80.0),
        remote_pref: s.rng.random_range(1..=5),
        questions: qs,
        step: 0,
        lines,
        gone: false,
        lied: false,
    })
}

pub fn current_question<'a>(content: &'a Content, iv: &Interview) -> Option<&'a QuestionDef> {
    iv.questions
        .get(iv.step)
        .and_then(|id| content.careers.questions.iter().find(|q| &q.id == id))
}

pub fn answer(content: &Content, s: &mut GameState, iv: &mut Interview, idx: usize) -> bool {
    let Some(q) = current_question(content, iv).cloned() else {
        return false;
    };
    let Some(a) = q.answers.get(idx) else {
        return false;
    };
    let mut d = a.interest;
    for (m, k) in &a.interest_by {
        d += k * (s.m(*m) - 50.0) / 5.0;
    }
    if let Some(days) = a.remote {
        d += 12.0 - 6.0 * (iv.remote_pref as f32 - days as f32).abs();
    }
    if let Some(cond) = &a.lie_if
        && cond.met(s)
    {
        iv.lied = true;
    }
    iv.lines.push((true, a.text.clone()));
    if !a.reaction.is_empty() {
        iv.lines.push((false, a.reaction.clone()));
    }
    iv.interest = (iv.interest + d).clamp(0.0, 100.0);
    iv.step += 1;
    if a.leave || iv.interest <= 0.0 {
        iv.gone = true;
    }
    true
}

/// Fait une offre (index dans `offers`) ou abandonne (`None`). Termine l'entretien.
pub fn offer(
    content: &Content,
    s: &mut GameState,
    iv: &Interview,
    idx: Option<usize>,
) -> Resolution {
    let role = content
        .careers
        .roles
        .iter()
        .find(|r| r.id == iv.role)
        .cloned();
    let Some(role) = role else {
        return Resolution::default();
    };
    if iv.gone {
        s.push_log(format!("{} a quitté l'entretien.", iv.name), Tone::Bad);
        let deltas = effects::apply(s, &Effects::from([(Metric::Attractiveness, -2.0)]));
        return Resolution {
            title: "Recrutement — échec".into(),
            text: format!(
                "{} a disparu de l'écran. Son profil LinkedIn affiche déjà « Open to work — sauf {} ».",
                iv.name, s.profile.company
            ),
            deltas,
            notes: vec![],
            speaker: String::new(), delayed_hint: false,
        };
    }
    let Some(o) = idx.and_then(|i| content.careers.offers.get(i)) else {
        return Resolution { title: "Recrutement abandonné".into(), text: "Vous remerciez le candidat. La DRH est soulagée : la grille salariale reste intacte.".into(), ..Default::default() };
    };
    let final_interest = iv.interest + o.interest;
    let roll: f32 = s.rng.random::<f32>() * 30.0 - 15.0;
    if final_interest + roll >= 50.0 {
        let salary = role.salary * o.salary_mult;
        s.team.push(Hire {
            role: role.id.clone(),
            name: iv.name.clone(),
            salary,
            since: s.turn(),
        });
        let deltas = effects::apply(s, &role.on_hire);
        s.push_log(
            format!("{} rejoint l'équipe ({}).", iv.name, role.label),
            Tone::Good,
        );
        s.decisions.push(format!("hire:{}", role.id));
        let mut notes = vec![format!("Coût : {salary:.2} pt de budget / mois.")];
        if iv.lied {
            let due = s.turn() + 2 + s.rng.random_range(0..3);
            s.scheduled.push(Scheduled {
                due,
                event: Some("lie_discovered".into()),
                effects: Effects::new(),
                text: String::new(),
                origin: iv.name.clone(),
            });
            notes.push("Vous avez un peu arrangé la vérité pendant l'entretien…".into());
        }
        Resolution {
            title: format!("{} recruté·e !", role.label),
            text: format!(
                "{} signe. La DRH demande s'il ou elle acceptera de manager 40 personnes. Réponse : non.",
                iv.name
            ),
            deltas,
            notes,
            speaker: String::new(), delayed_hint: iv.lied,
        }
    } else {
        s.push_log(format!("{} a décliné votre offre.", iv.name), Tone::Bad);
        let deltas = effects::apply(s, &Effects::from([(Metric::Attractiveness, -1.0)]));
        Resolution {
            title: "Offre déclinée".into(),
            text: format!(
                "{} a accepté une offre d'une scale-up qui « fait du Rust en prod ». Il ou elle vous souhaite bonne chance avec la migration.",
                iv.name
            ),
            deltas,
            notes: vec![],
            speaker: String::new(), delayed_hint: false,
        }
    }
}

/// Coût mensuel de l'équipe recrutée.
pub fn payroll(s: &GameState) -> f32 {
    s.team.iter().map(|h| h.salary).sum()
}

/// Effets mensuels de l'équipe, et départs éventuels.
pub fn monthly(content: &Content, s: &mut GameState) {
    let team = s.team.clone();
    for h in &team {
        if let Some(r) = content.careers.roles.iter().find(|r| r.id == h.role) {
            for (m, v) in &r.monthly {
                if *v > 0.0 && s.m(*m) >= r.cap {
                    continue;
                }
                s.metrics.add(*m, *v);
            }
        }
    }
    // Départs : plus la satisfaction des devs est basse, plus le risque est élevé.
    if !s.team.is_empty() {
        let p = ((45.0 - s.m(Metric::DevSat)) / 300.0).max(0.0) + 0.01;
        if s.rng.random::<f32>() < p {
            let i = s.rng.random_range(0..s.team.len());
            let h = s.team.remove(i);
            let label = content
                .careers
                .roles
                .iter()
                .find(|r| r.id == h.role)
                .map(|r| r.label.clone())
                .unwrap_or_default();
            s.metrics.add(Metric::Make, -4.0);
            s.metrics.add(Metric::DevSat, -2.0);
            s.push_log(
                format!(
                    "{} ({label}) a démissionné. Pot de départ jeudi, en visio.",
                    h.name
                ),
                Tone::Bad,
            );
        }
    }
}
