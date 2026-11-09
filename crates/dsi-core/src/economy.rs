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
    s.allocation - s.staff_payroll - crate::score::run_cost(s) - crate::projects::burn(content, s)
}

/// Répartition du budget mensuel, pour l'affichage.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Breakdown {
    /// Budget mensuel alloué par le DAF.
    pub allocation: f32,
    /// Salaires : équipe historique + recrues.
    pub salaries: f32,
    /// Licences, abonnements et contrats fournisseurs (hors ESN).
    pub software: f32,
    /// Régie ESN et renforts ESN sur les projets.
    pub esn: f32,
    /// Facture cloud + tokens / GPU.
    pub cloud_ai: f32,
    /// Coût des projets en cours (hors surcoût ESN).
    pub projects: f32,
    /// Solde : allocation − dépenses.
    pub balance: f32,
}

impl Breakdown {
    pub fn spent(&self) -> f32 {
        self.salaries + self.software + self.esn + self.cloud_ai + self.projects
    }
}

pub fn breakdown(content: &Content, s: &GameState) -> Breakdown {
    let is_esn = |id: &str| {
        content
            .contract(id)
            .is_some_and(|d| d.tags.iter().any(|t| t == "esn"))
    };
    let (mut software, mut esn) = (0.0, 0.0);
    for c in s.contracts.iter().filter(|c| c.active) {
        if is_esn(&c.id) {
            esn += c.monthly_cost;
        } else {
            software += c.monthly_cost;
        }
    }
    let mut projects = 0.0;
    for p in &s.projects {
        if let Some(d) = crate::projects::def_of(content, p) {
            projects += d.burn;
            if p.esn {
                esn += d.burn * 0.6;
            }
        }
    }
    let b = Breakdown {
        allocation: s.allocation,
        salaries: s.staff_payroll + crate::people::payroll(s),
        software,
        esn,
        cloud_ai: s.cloud.bill + s.ai.token_bill,
        projects,
        balance: 0.0,
    };
    Breakdown {
        balance: b.allocation - b.spent(),
        ..b
    }
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
