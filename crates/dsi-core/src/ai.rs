//! IA générative, vibe coding, agents. Métrique clé : CODE GÉNÉRÉ / CODE COMPRIS.
//! L'IA accélère la production… et la création de dette si personne ne relit.

use crate::state::{GameState, Metric, Scheduled, Tone};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct AiState {
    /// Milliers de lignes générées par IA.
    pub code_generated: f32,
    /// Milliers de lignes effectivement relues et comprises.
    pub code_understood: f32,
    /// Coût mensuel des tokens / GPU.
    pub token_bill: f32,
    #[serde(default)]
    pub warned: bool,
}

impl AiState {
    /// Ratio généré / compris (1.0 = tout est compris).
    pub fn ratio(&self) -> f32 {
        if self.code_generated <= 0.1 {
            1.0
        } else {
            self.code_generated / self.code_understood.max(1.0)
        }
    }
}

pub fn monthly(s: &mut GameState) {
    let vibe = s.has_tech("vibe_coding") || s.has_tech("copilot");
    if vibe {
        let generated =
            8.0 + s.m(Metric::AiMaturity) * 0.25 + if s.has_tech("agents") { 12.0 } else { 0.0 };
        s.ai.code_generated += generated;
        // La relecture dépend des humains compétents et des garde-fous.
        let review = 4.0
            + s.m(Metric::Make) * 0.12
            + s.m(Metric::DevopsMaturity) * 0.06
            + if s.has_tech("ai_guardrails") {
                8.0
            } else {
                0.0
            };
        s.ai.code_understood += review.min(generated * 1.2);
        s.ai.code_understood = s.ai.code_understood.min(s.ai.code_generated);
        let ratio = s.ai.ratio();
        if ratio > 1.6 {
            s.metrics.add(Metric::Debt, 0.4 * (ratio - 1.0));
            s.metrics.add(Metric::Security, -0.2 * (ratio - 1.0));
        }
        if ratio > 2.5 && !s.ai.warned {
            s.ai.warned = true;
            let due = s.turn() + 2 + s.rng.random_range(0..3);
            s.scheduled.push(Scheduled {
                due,
                event: Some("vibe_catastrophe".into()),
                effects: Default::default(),
                text: String::new(),
                origin: "vibe".into(),
            });
            s.push_log("Alerte : le ratio code généré / code compris dépasse 2,5. Personne ne relit plus rien.", Tone::Bad);
        }
    }
    if s.ai.token_bill > 0.0 {
        s.ai.token_bill *= 1.02;
    }
}
