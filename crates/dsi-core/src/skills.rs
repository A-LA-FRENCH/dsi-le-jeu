//! Culture engineering : Waterfall → Agile → CI/CD → DevOps → DevSecOps → SRE →
//! Platform Engineering → MLOps → AI Engineering → Agentic Engineering.
//! Chaque palier est une technologie de la branche « culture », obtenue par un
//! projet de transformation (compétences, temps, confiance, budget) : on ne
//! « passe pas DevOps » en cliquant.

use crate::state::GameState;

pub const LADDER: [(&str, &str); 10] = [
    ("waterfall", "Waterfall"),
    ("agile", "Agile"),
    ("cicd", "CI/CD"),
    ("devops", "DevOps"),
    ("devsecops", "DevSecOps"),
    ("sre", "SRE"),
    ("platform_eng", "Platform Engineering"),
    ("mlops", "MLOps"),
    ("ai_eng", "AI Engineering"),
    ("agentic_eng", "Agentic Engineering"),
];

/// Palier de culture atteint (index dans `LADDER`).
pub fn culture_level(s: &GameState) -> usize {
    LADDER
        .iter()
        .rposition(|(id, _)| s.has_tech(id))
        .unwrap_or(0)
}

pub fn culture_label(s: &GameState) -> &'static str {
    LADDER[culture_level(s)].1
}

/// Le curseur BUY → MAKE affiché au joueur (en %).
pub fn make_share(s: &GameState) -> u32 {
    let make = s.m(crate::state::Metric::Make);
    (make * 0.9).round().clamp(5.0, 95.0) as u32
}
