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
    /// Fin visée (saison 2) : titre, conditions remplies, total.
    #[serde(default)]
    pub ambition: Option<(String, usize, usize)>,
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
    // Au-delà des années écrites, les objectifs de la dernière année s'appliquent.
    let def = content
        .dialogues
        .years
        .iter()
        .find(|y| y.year == s.cal.year)
        .or_else(|| content.dialogues.years.iter().max_by_key(|y| y.year));
    if let Some(y) = def {
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
        last: s.cal.year >= final_year(content, s),
        ambition: s.ambition.as_deref().and_then(|a| {
            ending_goals(s).into_iter().find(|g| g.id == a).map(|g| {
                let (k, n) = g.progress();
                (g.title.to_string(), k, n)
            })
        }),
    }
}

/// Dernière année de la partie en cours (campagne + prolongations).
pub fn final_year(content: &Content, s: &GameState) -> u32 {
    content.dialogues.campaign_length + s.extension
}

/// Une condition mesurable d'une fin.
#[derive(Clone, Debug, PartialEq)]
pub struct GoalCheck {
    pub label: String,
    /// Valeur actuelle, pour l'affichage (« 64 », « non débloquée »…).
    pub current: String,
    pub ok: bool,
}

/// Une fin visée : conditions explicites et leviers pour l'atteindre.
#[derive(Clone, Debug, PartialEq)]
pub struct EndingGoal {
    pub id: &'static str,
    pub title: &'static str,
    /// Ce que cette fin raconte, en une phrase.
    pub pitch: &'static str,
    /// Projets et décisions qui y mènent.
    pub levers: &'static str,
    pub checks: Vec<GoalCheck>,
}

impl EndingGoal {
    pub fn met(&self) -> bool {
        self.checks.iter().all(|c| c.ok)
    }
    pub fn progress(&self) -> (usize, usize) {
        (
            self.checks.iter().filter(|c| c.ok).count(),
            self.checks.len(),
        )
    }
}

/// Points de score ajoutés quand la fin visée (le cap) est atteinte.
pub const AMBITION_BONUS: u32 = 5;

fn min_check(s: &GameState, m: Metric, v: f32) -> GoalCheck {
    let cur = s.m(m);
    GoalCheck {
        label: format!("{} ≥ {v:.0}", m.label()),
        current: format!("{cur:.0}"),
        ok: cur >= v,
    }
}

fn max_check(s: &GameState, m: Metric, v: f32) -> GoalCheck {
    let cur = s.m(m);
    GoalCheck {
        label: format!("{} ≤ {v:.0}", m.label()),
        current: format!("{cur:.0}"),
        ok: cur <= v,
    }
}

/// Les fins remarquables, par ordre de priorité, avec leurs conditions exactes.
/// « Toujours en poste » est la fin par défaut quand aucune n'est atteinte.
pub fn ending_goals(s: &GameState) -> Vec<EndingGoal> {
    let mut pragmatic: Vec<GoalCheck> = SCORE_PARTS
        .iter()
        .map(|(m, inv)| {
            if *inv {
                max_check(s, *m, 45.0)
            } else {
                min_check(s, *m, 55.0)
            }
        })
        .collect();
    pragmatic.push(max_check(s, Metric::Complexity, 50.0));
    pragmatic.push(max_check(s, Metric::VendorDep, 55.0));
    let context = s.has_tech("context_platform");
    vec![
        EndingGoal {
            id: "pragmatic",
            title: "Pragmatic CIO",
            pitch: "Aucun point faible : les 10 composantes du score au-dessus de 55 (dette sous 45), complexité et dépendance maîtrisées. La plus difficile.",
            levers: "Équilibrer plutôt que pousser un seul axe : cartographie et décommissionnement, PRA, SSO + MFA, forge et CI/CD, internalisation, renégociations.",
            checks: pragmatic,
        },
        EndingGoal {
            id: "agentic",
            title: "Agentic Enterprise",
            pitch: "Humains, logiciels et agents travaillent ensemble, avec du contexte, des permissions et des traces.",
            levers: "Gouvernance data → Plateforme IA → Agents IA métiers → Enterprise Context Platform. Sécurité à surveiller (Zero Trust, passerelle MCP).",
            checks: vec![
                GoalCheck {
                    label: "Enterprise Context Platform livrée".into(),
                    current: if context {
                        "livrée".into()
                    } else {
                        "non".into()
                    },
                    ok: context,
                },
                min_check(s, Metric::AiMaturity, 60.0),
                min_check(s, Metric::Security, 55.0),
            ],
        },
        EndingGoal {
            id: "software",
            title: "Software Company",
            pitch: "Vous construisez ce qui différencie l'entreprise et achetez le reste. Les équipes livrent tous les jours.",
            levers: "Internalisation, recrutements, Internal Developer Platform, CI/CD, plateforme API, Agentic Engineering. Moins d'ESN.",
            checks: vec![
                min_check(s, Metric::Make, 72.0),
                min_check(s, Metric::Velocity, 50.0),
            ],
        },
        EndingGoal {
            id: "sovereign",
            title: "DSI souveraine",
            pitch: "Vous maîtrisez vos dépendances, vos données et vos clés.",
            levers: "Sortie de VMware (XCP-ng, Proxmox), PostgreSQL, cloud de confiance, suite collaborative open source, modèles open weights, clauses de sortie.",
            checks: vec![
                min_check(s, Metric::Sovereignty, 70.0),
                max_check(s, Metric::VendorDep, 40.0),
            ],
        },
        EndingGoal {
            id: "cloud_native",
            title: "Cloud Native Enterprise",
            pitch: "Infrastructure as code, déploiements continus, FinOps au quotidien.",
            levers: "Migration cloud (public ou de confiance), Infrastructure as Code, transformation DevOps, CI/CD, SRE. Surveiller la facture.",
            checks: vec![
                min_check(s, Metric::CloudMaturity, 60.0),
                min_check(s, Metric::DevopsMaturity, 60.0),
            ],
        },
        EndingGoal {
            id: "engineering",
            title: "Engineering Culture",
            pitch: "Les meilleurs développeurs veulent travailler chez vous.",
            levers: "Forge, assistants de code, IDP, transformation Agile/DevOps, open source de la plateforme, recrutements honnêtes, éviter les crises à répétition.",
            checks: vec![
                min_check(s, Metric::Attractiveness, 70.0),
                min_check(s, Metric::DevSat, 70.0),
            ],
        },
    ]
}

fn ending_text(id: &str, s: &GameState) -> (&'static str, &'static str) {
    match id {
        "pragmatic" => (
            "Pragmatic CIO",
            "Vous n'avez suivi aucune mode aveuglément. Un peu de cloud, un peu de MAKE, beaucoup de bon sens. Le SI est équilibré, les équipes dorment la nuit. C'est la fin la plus rare : personne n'écrira de livre blanc sur vous.",
        ),
        "agentic" => (
            "Agentic Enterprise",
            "Humains, logiciels et agents travaillent ensemble, avec des permissions, du contexte et des traces d'audit. Les agents ouvrent des tickets ; les humains décident. Le consultant n'a plus de slide à vous vendre.",
        ),
        "software" => (
            "Software Company",
            "Vous construisez ce qui différencie l'entreprise et achetez le reste. Vos équipes livrent tous les jours. La DRH a enfin compris ce qu'est un Staff Engineer.",
        ),
        "sovereign" => (
            "DSI souveraine",
            "Vous maîtrisez vos dépendances, vos données et vos clés. Les commerciaux vous appellent « le client difficile ». Vous le prenez comme un compliment.",
        ),
        "cloud_native" => (
            "Cloud Native Enterprise",
            "Infrastructure as code, déploiements continus, FinOps au quotidien. Le datacenter est devenu une salle de ping-pong. La facture, elle, est toujours là.",
        ),
        "engineering" => (
            "Engineering Culture",
            "Les meilleurs développeurs veulent travailler chez vous. Vos conférences internes sont pleines. Vos posts de blog technique sont lus. Par des concurrents, surtout.",
        ),
        _ => (
            "Toujours en poste",
            if s.cal.year > 3 {
                "Six ans de DSI. Le SI tient debout, plus ou moins. Vous avez survécu à deux DG, trois réorganisations et une migration ERP. On garde SAP encore un an."
            } else {
                "Trois ans de DSI. Le SI tient debout, plus ou moins. Vous comprenez maintenant pourquoi les DSI ont l'air fatigués. On garde SAP encore un an."
            },
        ),
    }
}

/// Fin de campagne : la fin visée (cap) si elle est atteinte, sinon la plus remarquable atteinte.
pub fn ending(s: &GameState) -> Ending {
    let goals = ending_goals(s);
    let target = s
        .ambition
        .as_deref()
        .and_then(|a| goals.iter().find(|g| g.id == a));
    let hit = target
        .filter(|g| g.met())
        .or_else(|| goals.iter().find(|g| g.met()));
    let id = hit.map_or("survivor", |g| g.id);
    let (title, text) = ending_text(id, s);
    let ambition = target.map(|g| (g.title.to_string(), g.met()));
    let mut sc = score(s);
    if ambition.as_ref().is_some_and(|(_, ok)| *ok) {
        sc = (sc + AMBITION_BONUS).min(100);
    }
    Ending {
        id: id.into(),
        title: title.into(),
        text: text.into(),
        score: sc,
        rank: rank(sc).into(),
        ambition,
    }
}
