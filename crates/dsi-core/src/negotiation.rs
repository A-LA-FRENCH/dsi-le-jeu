//! Mini-jeu de négociation commerciale : un rapport de force (0–100), quelques tours,
//! des cartes à jouer, des aléas. Le résultat fixe la hausse (ou la remise) du contrat.

use crate::content::Content;
use crate::contracts::{self, ContractDef};
use crate::effects::{self, Effects};
use crate::state::{GameState, Metric, Resolution, Tone};
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Who {
    You,
    Vendor,
    Hazard,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NegLine {
    pub who: Who,
    pub text: String,
    pub delta: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Negotiation {
    pub contract: String,
    pub vendor: String,
    pub leverage: f32,
    pub rounds_left: u32,
    pub hand: Vec<String>,
    pub played: Vec<String>,
    pub log: Vec<NegLine>,
    /// Négociation anticipée (avant l'échéance).
    pub early: bool,
    pub months_left: i32,
    pub credibility: f32,
    pub alternatives: usize,
    pub dependency: f32,
    pub base_increase: f32,
    pub migration_cost: f32,
    pub op_risk: f32,
    pub exit_clause: bool,
    pub term_5y: bool,
    pub scope_cut: bool,
    pub costs: Effects,
}

impl Negotiation {
    /// Hausse obtenue si l'on conclut maintenant.
    pub fn projected_increase(&self) -> f32 {
        let mut inc = self.base_increase * (1.0 - self.leverage / 65.0);
        if self.leverage > 65.0 {
            inc = -0.25 * ((self.leverage - 65.0) / 35.0).min(1.0);
        }
        if self.term_5y {
            inc -= 0.12;
        }
        inc.clamp(-0.3, self.base_increase * 1.3)
    }
}

/// Ouvre une négociation sur un contrat.
pub fn start(
    content: &Content,
    s: &mut GameState,
    contract: &str,
    early: bool,
) -> Option<Negotiation> {
    let def = content.contract(contract)?.clone();
    let st = s
        .contracts
        .iter()
        .find(|c| c.id == contract && c.active)?
        .clone();
    let vendor = content
        .vendor(&def.vendor)
        .map(|v| v.name.clone())
        .unwrap_or(def.vendor.clone());
    let now = s.turn();
    let months_left = st.months_left(now);
    let alts = contracts::alternatives(&def, s);
    let credibility =
        (alts as f32 * 0.3 + s.m(Metric::Make) / 100.0 + s.m(Metric::Sovereignty) / 200.0)
            .clamp(0.1, 1.0);
    let mut lev =
        45.0 + alts as f32 * 8.0 + (s.m(Metric::Make) - 20.0) * 0.15 - def.dependency * 25.0;
    let mut log = Vec::new();
    let rep_line = content
        .vendor(&def.vendor)
        .and_then(|v| v.lines.first().cloned())
        .unwrap_or_else(|| "Ce partenariat est stratégique.".into());
    log.push(NegLine {
        who: Who::Vendor,
        text: if def.pitch.is_empty() {
            rep_line
        } else {
            def.pitch.clone()
        },
        delta: 0.0,
    });
    if early && months_left >= 2 {
        lev += 12.0;
        log.push(NegLine { who: Who::Hazard, text: format!("Vous négociez {months_left} mois avant l'échéance. Le commercial n'a pas encore son objectif en tête."), delta: 12.0 });
    } else if months_left <= 0 {
        lev -= 40.0;
        log.push(NegLine {
            who: Who::Hazard,
            text: "Vous avez attendu J-2 avant de négocier.".into(),
            delta: -40.0,
        });
    }
    if s.flags.remove("mentor_leverage") {
        lev += 10.0;
        log.push(NegLine {
            who: Who::Hazard,
            text: "Vous avez suivi le conseil de votre mentor : vous arrivez préparé.".into(),
            delta: 10.0,
        });
    }
    let hand: Vec<String> = content
        .cards
        .iter()
        .filter(|c| c.only_tags.is_empty() || c.only_tags.iter().any(|t| def.tags.contains(t)))
        .filter(|c| c.requires.met(s))
        .filter(|c| !c.needs_alternative || alts > 0)
        .map(|c| c.id.clone())
        .collect();
    let migration_cost = migration_cost(content, &def);
    Some(Negotiation {
        contract: contract.to_string(),
        vendor,
        leverage: lev.clamp(0.0, 100.0),
        rounds_left: 3,
        hand,
        played: Vec::new(),
        log,
        early,
        months_left,
        credibility,
        alternatives: alts,
        dependency: def.dependency,
        base_increase: def.increase,
        migration_cost,
        op_risk: (s.m(Metric::Debt) * 0.5 + s.m(Metric::Complexity) * 0.5).round(),
        exit_clause: false,
        term_5y: false,
        scope_cut: false,
        costs: Effects::new(),
    })
}

fn migration_cost(content: &Content, def: &ContractDef) -> f32 {
    def.migration
        .iter()
        .filter_map(|p| content.project(p))
        .map(|p| p.cost + p.burn * p.duration as f32)
        .fold(0.0, f32::max)
}

/// Joue une carte. Renvoie faux si la carte n'est pas jouable.
pub fn play(content: &Content, s: &mut GameState, n: &mut Negotiation, card_id: &str) -> bool {
    let Some(card) = content.card(card_id).cloned() else {
        return false;
    };
    if n.rounds_left == 0
        || !n.hand.iter().any(|c| c == card_id)
        || n.played.iter().any(|c| c == card_id)
    {
        return false;
    }
    let mut d = card.leverage;
    if card.credibility_scaled {
        d = card.leverage * n.credibility * 1.3 - 12.0 * (1.0 - n.credibility);
    }
    n.leverage = (n.leverage + d).clamp(0.0, 100.0);
    n.rounds_left = n.rounds_left.saturating_sub(card.rounds);
    n.played.push(card.id.clone());
    for (m, v) in &card.effects {
        *n.costs.entry(*m).or_insert(0.0) += v;
    }
    match card.special.as_deref() {
        Some("exit_clause") => n.exit_clause = true,
        Some("term_5y") => n.term_5y = true,
        Some("scope") => n.scope_cut = true,
        _ => {}
    }
    n.log.push(NegLine {
        who: Who::You,
        text: card.label.clone(),
        delta: d,
    });
    // Réaction du commercial.
    let reaction = if !card.reaction.is_empty() {
        card.reaction.clone()
    } else {
        let lines = content.vendor_lines(
            &content
                .contract(&n.contract)
                .map(|c| c.vendor.clone())
                .unwrap_or_default(),
        );
        if lines.is_empty() {
            "Je dois en parler à mon management.".into()
        } else {
            lines[s.rng.random_range(0..lines.len())].clone()
        }
    };
    n.log.push(NegLine {
        who: Who::Vendor,
        text: reaction,
        delta: 0.0,
    });
    // Aléa.
    if n.rounds_left > 0 && s.rng.random::<f32>() < 0.35 && !content.hazards.is_empty() {
        let total: f32 = content.hazards.iter().map(|h| h.weight).sum();
        let mut r = s.rng.random::<f32>() * total;
        for h in &content.hazards {
            if r < h.weight {
                n.leverage = (n.leverage + h.leverage).clamp(0.0, 100.0);
                n.log.push(NegLine {
                    who: Who::Hazard,
                    text: format!("{} — {}", h.label, h.text),
                    delta: h.leverage,
                });
                break;
            }
            r -= h.weight;
        }
    }
    true
}

/// Conclut (ou accepte telle quelle si `accept`). Applique au contrat et renvoie le résultat.
pub fn conclude(content: &Content, s: &mut GameState, n: &Negotiation, accept: bool) -> Resolution {
    let inc = if accept {
        n.base_increase
    } else {
        n.projected_increase()
    };
    let label = content
        .contract(&n.contract)
        .map(|c| c.label.clone())
        .unwrap_or_default();
    let term = content.contract(&n.contract).map(|c| c.term).unwrap_or(12);
    let mut fx = n.costs.clone();
    let mut notes = Vec::new();
    if n.term_5y {
        *fx.entry(Metric::VendorDep).or_insert(0.0) += 6.0;
        notes.push(
            "Engagement 5 ans : remise, mais vous êtes marié(e) jusqu'en année 6.".to_string(),
        );
    }
    if n.exit_clause {
        *fx.entry(Metric::VendorDep).or_insert(0.0) -= 5.0;
        notes.push("Clause de sortie obtenue : la porte est entrouverte.".to_string());
    }
    if n.scope_cut {
        *fx.entry(Metric::UserSat).or_insert(0.0) -= 4.0;
        notes.push("Périmètre réduit : 300 utilisateurs perdent un outil qu'ils utilisaient « tous les jours ».".to_string());
    }
    if inc <= 0.0 {
        *fx.entry(Metric::Comex).or_insert(0.0) += 4.0;
    } else if inc > 0.3 {
        *fx.entry(Metric::Comex).or_insert(0.0) -= 4.0;
    }
    if !accept && !n.played.is_empty() {
        // Le temps passé en négociation a un coût : moins pour les achats.
        *fx.entry(Metric::Velocity).or_insert(0.0) -= 1.0;
    }
    let scope = if n.scope_cut { 0.85 } else { 1.0 };
    let now = s.turn();
    let early = n.early && n.months_left > 0;
    let mut new_cost = 0.0;
    if let Some(c) = s.contracts.iter_mut().find(|c| c.id == n.contract) {
        if early {
            c.pre_deal = Some(inc);
            c.monthly_cost *= scope;
            new_cost = c.monthly_cost * (1.0 + inc);
        } else {
            c.monthly_cost = c.monthly_cost * (1.0 + inc) * scope;
            c.next_renewal = now.max(c.next_renewal) + if n.term_5y { 60 } else { term };
            c.pre_deal = None;
            c.renewals += 1;
            new_cost = c.monthly_cost;
        }
        c.exit_clause |= n.exit_clause;
    }
    let deltas = effects::apply(s, &fx);
    let pct = (inc * 100.0).round() as i32;
    let text = if accept {
        format!("Vous signez. Hausse de {pct} %. Le commercial vous envoie un panier garni.")
    } else if pct <= 0 {
        format!(
            "Accord conclu : {} %. Le commercial parle de « relation de confiance ». Le DAF vous sourit pour la première fois.",
            pct
        )
    } else if pct < 20 {
        format!(
            "Accord conclu : +{pct} %. Honorable. Le DAF demande quand même pourquoi ce n'est pas 0."
        )
    } else {
        format!(
            "Accord conclu : +{pct} %. Le commercial a atteint son objectif trimestriel. Vous, non."
        )
    };
    let tone = if pct <= 0 {
        Tone::Good
    } else if pct > 30 {
        Tone::Bad
    } else {
        Tone::Money
    };
    s.push_log(
        format!(
            "{label} : {}{pct} % ({})",
            if pct > 0 { "+" } else { "" },
            if early { "pré-accord" } else { "renouvelé" }
        ),
        tone,
    );
    s.decisions
        .push(format!("negotiation:{}:{}", n.contract, pct));
    if early {
        notes.push(format!(
            "Pré-accord enregistré : appliqué à l'échéance (coût mensuel futur ≈ {new_cost:.2})."
        ));
    } else {
        notes.push(format!("Nouveau coût mensuel : {new_cost:.2} pt/mois."));
    }
    s.check_game_over();
    Resolution {
        title: format!("Négociation — {}", n.vendor),
        text,
        deltas,
        notes,
        speaker: String::new(), delayed_hint: false,
    }
}
