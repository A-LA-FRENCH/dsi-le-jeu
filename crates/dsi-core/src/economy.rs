//! Règles économiques mensuelles : budget (allocation − coûts de fonctionnement),
//! entropie (la dette monte toute seule), et métriques « de flux » qui tendent vers
//! une cible calculée à partir du reste du SI. Les métriques sont antagonistes :
//! réduire les coûts tout de suite fait souvent monter la dette et la dépendance.

use crate::content::Content;
use crate::state::{GameState, Metric};

/// Vitesse de convergence des métriques de flux vers leur cible.
const PULL: f32 = 0.25;

pub fn target(s: &GameState, m: Metric) -> Option<f32> {
    let g = |k: Metric| s.m(k);
    let culture = crate::skills::culture_level(s) as f32;
    Some(match m {
        Metric::Uptime => {
            95.0 - g(Metric::Debt) * 0.25 - g(Metric::Complexity) * 0.15
                + g(Metric::Resilience) * 0.15
                + g(Metric::DevopsMaturity) * 0.05
        }
        Metric::Velocity => {
            30.0 + g(Metric::DevopsMaturity) * 0.3 + g(Metric::Make) * 0.3
                - g(Metric::Debt) * 0.15
                - g(Metric::Complexity) * 0.1
                + g(Metric::DevSat) * 0.15
                + culture
        }
        Metric::UserSat => {
            12.0 + g(Metric::Uptime) * 0.3
                + g(Metric::Velocity) * 0.3
                + g(Metric::DataQuality) * 0.1
        }
        Metric::DevSat => {
            40.0 + g(Metric::DevopsMaturity) * 0.25 + g(Metric::Make) * 0.15 - g(Metric::Debt) * 0.2
                + g(Metric::Attractiveness) * 0.1
                + culture * 1.5
        }
        Metric::Attractiveness => {
            14.0 + g(Metric::DevSat) * 0.3 + g(Metric::Make) * 0.2 + g(Metric::DevopsMaturity) * 0.2
                - g(Metric::Debt) * 0.1
                + g(Metric::Sovereignty) * 0.05
        }
        Metric::VendorDep => {
            let deps: f32 = s
                .contracts
                .iter()
                .filter(|c| c.active)
                .map(|c| c.dependency_weight())
                .sum();
            20.0 + deps * 12.0 - g(Metric::Make) * 0.25
        }
        _ => return None,
    })
}

/// Recette mensuelle : allocation − (contrats + équipe + cloud + IA + projets).
pub fn monthly_balance(content: &Content, s: &GameState) -> f32 {
    s.allocation - crate::score::run_cost(s) - crate::projects::burn(content, s)
}

pub fn monthly(content: &Content, s: &mut GameState) {
    // Budget.
    let bal = monthly_balance(content, s);
    s.metrics.add(Metric::Budget, bal);

    // Entropie : le SI vieillit tout seul.
    let debt_drift = 0.7 + s.m(Metric::Complexity) * 0.008 - s.m(Metric::DevopsMaturity) * 0.012;
    s.metrics.add(Metric::Debt, debt_drift.max(0.1));
    let sec_drift = 0.6 + s.m(Metric::Debt) * 0.006 + s.m(Metric::Complexity) * 0.004
        - s.m(Metric::Make) * 0.003;
    s.metrics.add(Metric::Security, -sec_drift.max(0.05));
    s.metrics.add(Metric::Complexity, 0.15);
    s.metrics.add(Metric::DataQuality, -0.2);
    s.metrics.add(Metric::Make, -0.15);
    s.metrics.add(Metric::Resilience, -0.3);

    // Métriques de flux.
    for m in [
        Metric::Uptime,
        Metric::Velocity,
        Metric::UserSat,
        Metric::DevSat,
        Metric::Attractiveness,
        Metric::VendorDep,
    ] {
        if let Some(t) = target(s, m) {
            let cur = s.m(m);
            let pull = if m == Metric::VendorDep { 0.1 } else { PULL };
            s.metrics.add(m, (t.clamp(0.0, 100.0) - cur) * pull);
        }
    }

    // Le COMEX juge sur ce qu'il voit : la disponibilité, les utilisateurs, la vitesse, les finances.
    let mut comex = (s.m(Metric::UserSat) - 45.0) * 0.03
        + (s.m(Metric::Uptime) - 72.0) * 0.05
        + (s.m(Metric::Velocity) - 30.0) * 0.02;
    if s.m(Metric::Budget) < 15.0 {
        comex -= 1.5;
    }
    s.metrics.add(Metric::Comex, comex.clamp(-3.0, 2.0));
}
