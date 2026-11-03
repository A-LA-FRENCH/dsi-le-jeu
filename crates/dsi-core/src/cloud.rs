//! Cloud & FinOps : une facture mensuelle qui dérive si personne ne la regarde.
//! Le cloud n'est pas automatiquement moins cher.

use crate::content::Content;
use crate::state::{GameState, Metric, Tone};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct CloudState {
    pub active: bool,
    pub provider: Option<String>,
    /// Facture mensuelle (points de budget).
    pub bill: f32,
    /// Maturité FinOps (0–100) : freine la dérive.
    pub finops: f32,
}

impl CloudState {
    pub fn activate(&mut self, provider: &str) {
        self.active = true;
        if self.provider.is_none() {
            self.provider = Some(provider.to_string());
        }
    }
}

/// Dérive mensuelle de la facture et effets du fournisseur choisi.
pub fn monthly(content: &Content, s: &mut GameState) {
    if !s.cloud.active {
        return;
    }
    // Sans FinOps, la facture grossit : ressources oubliées, logs verbeux, egress.
    let drift = 0.04 * (1.0 - s.cloud.finops / 100.0) * (0.5 + s.rng.random::<f32>());
    s.cloud.bill = (s.cloud.bill * (1.0 + drift)).max(0.0);
    s.cloud.finops = (s.cloud.finops - 0.3).max(0.0);
    let sovereign = s
        .cloud
        .provider
        .as_deref()
        .and_then(|p| content.vendor(p))
        .is_some_and(|v| v.sovereign);
    if sovereign {
        if s.m(Metric::Sovereignty) < 80.0 {
            s.metrics.add(Metric::Sovereignty, 0.3);
        }
    } else if s.m(Metric::VendorDep) < 95.0 {
        s.metrics.add(Metric::VendorDep, 0.25);
    }
    if s.m(Metric::CloudMaturity) < 85.0 {
        s.metrics.add(Metric::CloudMaturity, 0.4);
    }
    if s.cloud.bill > 2.5 && s.rng.random::<f32>() < 0.15 {
        s.push_log(
            "Le DAF a reçu la facture cloud. Il demande ce qu'est un « NAT Gateway ».",
            Tone::Money,
        );
    }
}
