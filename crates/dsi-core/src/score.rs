//! Score, classement humoristique, revue annuelle du COMEX et fins de partie.

use crate::conditions::Conditions;
use crate::content::Content;
use crate::state::{Ending, GameState, Metric};
use serde::{Deserialize, Serialize};

/// Les dix composantes du score final.
pub const SCORE_PARTS: [(Metric, bool); 10] = [
    (Metric::Uptime, false),
    (Metric::Budget, false),
    (Metric::Security, false),
    (Metric::Debt, true),
    (Metric::Velocity, false),
    (Metric::Sovereignty, false),
    (Metric::Make, false),
    (Metric::DevSat, false),
    (Metric::UserSat, false),
    (Metric::Resilience, false),
];

pub fn score(s: &GameState) -> u32 {
    let sum: f32 = SCORE_PARTS
        .iter()
        .map(|(m, inv)| if *inv { 100.0 - s.m(*m) } else { s.m(*m) })
        .sum();
    (sum / SCORE_PARTS.len() as f32).round().clamp(0.0, 100.0) as u32
}

pub fn rank(score: u32) -> &'static str {
    match score {
        0..20 => "Directeur du PowerPoint",
        20..40 => "Chef de projet Jira",
        40..60 => "DSI survivant",
        60..80 => "CTO de combat",
        80..95 => "Grand architecte",
        _ => "Vous avez probablement triché.",
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Objective {
    pub label: String,
    #[serde(default)]
    pub conditions: Conditions,
    /// Baisse requise du coût de fonctionnement (0.15 = −15 %).
    #[serde(default)]
    pub run_cut: Option<f32>,
    #[serde(default = "reward")]
    pub reward: f32,
    #[serde(default = "penalty")]
    pub penalty: f32,
}

fn reward() -> f32 {
    6.0
}
fn penalty() -> f32 {
    -8.0
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct YearDef {
    pub year: u32,
    pub title: String,
    pub objectives: Vec<Objective>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct YearReview {
    pub year: u32,
    pub score: u32,
    pub rank: String,
    pub objectives: Vec<(String, bool)>,
    pub comex_delta: f32,
    pub run_cost: f32,
    pub baseline: f32,
    pub verdict: String,
    pub last: bool,
}

/// Coût de fonctionnement mensuel complet (contrats + équipe + cloud + IA).
pub fn run_cost(s: &GameState) -> f32 {
    crate::contracts::run_cost(s) + crate::people::payroll(s) + s.cloud.bill + s.ai.token_bill
}

pub fn year_review(content: &Content, s: &GameState) -> YearReview {
    let sc = score(s);
    let run = run_cost(s);
    let mut objectives = Vec::new();
    let mut delta = 0.0;
    if let Some(y) = content
        .dialogues
        .years
        .iter()
        .find(|y| y.year == s.cal.year)
    {
        for o in &y.objectives {
            let ok = o.conditions.met(s)
                && o.run_cut
                    .is_none_or(|cut| run <= s.baseline_run * (1.0 - cut));
            delta += if ok { o.reward } else { o.penalty };
            objectives.push((o.label.clone(), ok));
        }
    }
    let wins = objectives.iter().filter(|(_, ok)| *ok).count();
    let verdict = match (wins, objectives.len()) {
        (w, n) if w == n => {
            "La DG : « Remarquable. Du coup, on peut couper encore 10 % l'an prochain ? »"
        }
        (0, _) => {
            "La DG : « Je vais être franche. Le conseil d'administration pose des questions. Et moi aussi. »"
        }
        _ => "La DG : « C'est… contrasté. Le cabinet de conseil propose un diagnostic gratuit. »",
    };
    YearReview {
        year: s.cal.year,
        score: sc,
        rank: rank(sc).to_string(),
        objectives,
        comex_delta: delta,
        run_cost: run,
        baseline: s.baseline_run,
        verdict: verdict.into(),
        last: s.cal.year >= content.campaign_years(),
    }
}

/// Fin de campagne : choisit la fin la plus remarquable atteinte.
pub fn ending(s: &GameState) -> Ending {
    let sc = score(s);
    let m = |k: Metric| s.m(k);
    let balanced = SCORE_PARTS
        .iter()
        .all(|(k, inv)| if *inv { m(*k) <= 45.0 } else { m(*k) >= 55.0 })
        && m(Metric::Complexity) <= 50.0
        && m(Metric::VendorDep) <= 55.0;
    let (id, title, text) = if balanced {
        (
            "pragmatic",
            "Pragmatic CIO",
            "Vous n'avez suivi aucune mode aveuglément. Un peu de cloud, un peu de MAKE, beaucoup de bon sens. Le SI est équilibré, les équipes dorment la nuit. C'est la fin la plus rare : personne n'écrira de livre blanc sur vous.",
        )
    } else if s.has_tech("context_platform")
        && m(Metric::AiMaturity) >= 60.0
        && m(Metric::Security) >= 55.0
    {
        (
            "agentic",
            "Agentic Enterprise",
            "Humains, logiciels et agents travaillent ensemble, avec des permissions, du contexte et des traces d'audit. Les agents ouvrent des tickets ; les humains décident. Le consultant n'a plus de slide à vous vendre.",
        )
    } else if m(Metric::Make) >= 72.0 && m(Metric::Velocity) >= 50.0 {
        (
            "software",
            "Software Company",
            "Vous construisez ce qui différencie l'entreprise et achetez le reste. Vos équipes livrent tous les jours. La DRH a enfin compris ce qu'est un Staff Engineer.",
        )
    } else if m(Metric::Sovereignty) >= 70.0 && m(Metric::VendorDep) <= 40.0 {
        (
            "sovereign",
            "DSI souveraine",
            "Vous maîtrisez vos dépendances, vos données et vos clés. Les commerciaux vous appellent « le client difficile ». Vous le prenez comme un compliment.",
        )
    } else if m(Metric::CloudMaturity) >= 60.0 && m(Metric::DevopsMaturity) >= 60.0 {
        (
            "cloud_native",
            "Cloud Native Enterprise",
            "Infrastructure as code, déploiements continus, FinOps au quotidien. Le datacenter est devenu une salle de ping-pong. La facture, elle, est toujours là.",
        )
    } else if m(Metric::Attractiveness) >= 70.0 && m(Metric::DevSat) >= 70.0 {
        (
            "engineering",
            "Engineering Culture",
            "Les meilleurs développeurs veulent travailler chez vous. Vos conférences internes sont pleines. Vos posts de blog technique sont lus. Par des concurrents, surtout.",
        )
    } else {
        (
            "survivor",
            "Toujours en poste",
            "Trois ans de DSI. Le SI tient debout, plus ou moins. Vous comprenez maintenant pourquoi les DSI ont l'air fatigués. On garde SAP encore un an.",
        )
    };
    Ending {
        id: id.into(),
        title: title.into(),
        text: text.into(),
        score: sc,
        rank: rank(sc).into(),
    }
}
