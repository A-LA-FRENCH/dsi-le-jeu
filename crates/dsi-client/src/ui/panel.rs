//! Panneau de droite : à traiter + journal, indicateurs, contrats, Technology Map, équipe.

use super::style::*;
use super::widgets::*;
use super::{Ctx, Tab};
use bevy_egui::egui::{self, RichText};
use dsi_core::state::{Inbox, Metric, Room};
use dsi_core::{Command, skills};

pub fn show(ui: &mut egui::Ui, c: &mut Ctx) {
    ui.horizontal_wrapped(|ui| {
        let pending = c.g().inbox.len();
        let tabs = [
            (
                Tab::Inbox,
                if pending > 0 {
                    format!("📥 À traiter ({pending})")
                } else {
                    "📥 À traiter".into()
                },
                "A",
            ),
            (Tab::Metrics, "📊 Indicateurs".into(), "I"),
            (Tab::Contracts, "📄 Contrats".into(), "C"),
            (Tab::Tech, "🗺 Tech map".into(), "T"),
            (Tab::Team, "👥 Équipe".into(), "E"),
        ];
        for (tab, label, key) in tabs {
            let sel = c.ui.tab == tab;
            let b = ui.add(egui::Button::selectable(
                sel,
                RichText::new(label).size(13.5),
            ));
            if tab == Tab::Contracts {
                mark(&mut c.ui.marks, "tab_contracts", b.rect);
            }
            if b.on_hover_text(format!("Raccourci : {key}")).clicked() {
                c.ui.tab = tab;
                c.click();
            }
        }
    });
    ui.separator();
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| match c.ui.tab {
            Tab::Inbox => inbox(ui, c),
            Tab::Metrics => metrics(ui, c),
            Tab::Contracts => contracts(ui, c),
            Tab::Tech => tech(ui, c),
            Tab::Team => team(ui, c),
        });
}

fn inbox(ui: &mut egui::Ui, c: &mut Ctx) {
    let g = c.g();
    section(ui, &format!("{} — À TRAITER", g.cal.label().to_uppercase()));
    if g.inbox.is_empty() {
        muted(
            ui,
            "Rien en attente. Utilisez vos créneaux d'agenda dans les salles, puis passez au mois suivant.",
        );
    }
    let items: Vec<(usize, Inbox)> = g.inbox.iter().cloned().enumerate().collect();
    let mut open = None;
    for (i, item) in items {
        let room = dsi_core::game::inbox_room(c.content, &item);
        let title = dsi_core::game::inbox_title(c.content, c.g(), &item);
        let (kind, color) = match &item {
            Inbox::Event { id } => {
                let k = c.content.event(id).map(|e| e.kind).unwrap_or_default();
                (k.label(), kind_color(k))
            }
            Inbox::Renewal { .. } => ("CONTRACT RENEWAL", BLUE),
            Inbox::Consequence { .. } => ("CONSÉQUENCE", PURPLE),
            Inbox::YearReview => ("COMEX MEETING", ORANGE),
            Inbox::Chapter { .. } => ("CHAPITRE", YELLOW),
        };
        let resp = card()
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    kind_banner(ui, kind, color);
                    muted(ui, room.label());
                });
                ui.label(RichText::new(title).font(bold(14.0)).color(TEXT));
            })
            .response
            .interact(egui::Sense::click());
        if i == 0 {
            mark(&mut c.ui.marks, "inbox_first", resp.rect);
        }
        if resp
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            open = Some(i);
        }
        ui.add_space(2.0);
    }
    if let Some(i) = open {
        c.run(Command::Open(i));
    }
    section(ui, "FIL TEAMS");
    let g = c.g();
    for e in g.log.iter().rev().take(60) {
        let cal = dsi_core::state::Calendar::from_abs(e.turn);
        ui.horizontal_wrapped(|ui| {
            let m: String = cal.month_name().chars().take(4).collect();
            ui.label(
                RichText::new(format!("{m} A{}", cal.year))
                    .size(11.0)
                    .color(MUTED),
            );
            ui.add(
                egui::Label::new(RichText::new(&e.text).size(13.0).color(tone_color(e.tone)))
                    .wrap(),
            );
        });
    }
}

fn metrics(ui: &mut egui::Ui, c: &mut Ctx) {
    // En saison 2, les objectifs de fin passent en tête de l'onglet.
    let season2 = c.g().extension > 0;
    if season2 {
        super::ambition::section_goals(ui, c);
    }
    metrics_body(ui, c);
    if !season2 {
        super::ambition::section_goals(ui, c);
    }
}

fn metrics_body(ui: &mut egui::Ui, c: &mut Ctx) {
    let g = c.g();
    section(ui, "TABLEAU DE BORD COMPLET");
    egui::Grid::new("metrics")
        .num_columns(3)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            for m in Metric::ALL {
                let v = g.m(m);
                ui.label(RichText::new(m.label()).size(13.0));
                gauge(ui, v / 100.0, metric_color(m, v), 110.0, 9.0);
                ui.label(
                    RichText::new(format!("{v:.0}"))
                        .font(bold(13.0))
                        .color(metric_color(m, v)),
                );
                ui.end_row();
            }
        });
    section(ui, "BUDGET MENSUEL");
    let bd = dsi_core::economy::breakdown(c.content, g);
    budget_bar(ui, &bd, 260.0, 14.0).on_hover_text(budget_tooltip(&bd));
    egui::Grid::new("budget")
        .num_columns(3)
        .spacing([10.0, 3.0])
        .show(ui, |ui| {
            ui.label(RichText::new("Budget alloué").strong());
            ui.label(RichText::new(format!("{:.2}", bd.allocation)).color(GREEN));
            ui.label("");
            ui.end_row();
            for (k, col, v) in budget_items(&bd) {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(9.0, 9.0), egui::Sense::hover());
                    ui.painter().rect_filled(r, 0.0, col);
                    ui.label(k);
                });
                ui.label(
                    RichText::new(format!("{:+.2}", -v)).color(if v.abs() < 0.005 {
                        MUTED
                    } else {
                        RED
                    }),
                );
                let pct = if bd.allocation > 0.0 {
                    v / bd.allocation * 100.0
                } else {
                    0.0
                };
                ui.label(RichText::new(format!("{pct:.0} %")).color(MUTED));
                ui.end_row();
            }
            ui.label(RichText::new("Solde du mois").strong());
            ui.label(
                RichText::new(format!("{:+.2}", bd.balance))
                    .strong()
                    .color(if bd.balance >= 0.0 { GREEN } else { RED }),
            );
            ui.label("");
            ui.end_row();
        });
    muted(
        ui,
        format!(
            "Salaires : équipe historique {:.1} + recrues {:.1} · ESN : régie + renforts projets · Cloud {:.2} + tokens/GPU {:.2}",
            g.staff_payroll,
            dsi_core::people::payroll(g),
            g.cloud.bill,
            g.ai.token_bill
        ),
    );
    egui::Grid::new("budget_run").num_columns(2).show(ui, |ui| {
        let run = dsi_core::score::run_cost(g);
        ui.label(RichText::new("Coût de fonctionnement").strong());
        let diff = if g.baseline_run > 0.0 {
            (run / g.baseline_run - 1.0) * 100.0
        } else {
            0.0
        };
        ui.label(
            RichText::new(format!("{run:.2} ({diff:+.0} % vs janvier)")).color(if diff <= -15.0 {
                GREEN
            } else if diff <= 0.0 {
                YELLOW
            } else {
                RED
            }),
        );
        ui.end_row();
    });
    section(ui, "TRANSFORMATION");
    let make = skills::make_share(g);
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("BUY {}%", 100 - make)).color(ORANGE));
        gauge(ui, make as f32 / 100.0, CYAN, 120.0, 10.0);
        ui.label(RichText::new(format!("{make}% MAKE")).color(CYAN));
    })
    .response
    .on_hover_text("Part de ce que l'entreprise sait construire et maîtriser elle-même. MAKE n'est pas toujours mieux : Core vs Commodity.");
    ui.label(RichText::new(format!("Culture : {}", skills::culture_label(g))).color(LIGHT));
    let ladder: Vec<String> = skills::LADDER
        .iter()
        .map(|(id, l)| {
            if g.has_tech(id) {
                format!("■ {l}")
            } else {
                format!("□ {l}")
            }
        })
        .collect();
    muted(ui, ladder.join("  "));
    if g.cloud.active || g.cloud.provider.is_some() {
        ui.label(format!(
            "Cloud : {} · facture {:.2}/mois · FinOps {:.0}",
            g.cloud
                .provider
                .as_deref()
                .and_then(|p| c.content.vendor(p))
                .map(|v| v.name.as_str())
                .unwrap_or("—"),
            g.cloud.bill,
            g.cloud.finops
        ));
    }
    if g.ai.code_generated > 0.0 {
        let r = g.ai.ratio();
        ui.label(RichText::new(format!("CODE GÉNÉRÉ / CODE COMPRIS : {:.1}", r)).font(bold(13.0)).color(if r > 2.0 { RED } else if r > 1.4 { ORANGE } else { GREEN }))
            .on_hover_text("Si le code produit par IA dépasse largement ce que les équipes comprennent, une catastrophe se prépare.");
    }
    let sc = dsi_core::score::score(g);
    section(ui, "SCORE");
    ui.label(
        RichText::new(format!("{sc}/100 — « {} »", dsi_core::score::rank(sc)))
            .font(bold(14.0))
            .color(YELLOW),
    );
    if !g.year_scores.is_empty() {
        muted(
            ui,
            format!(
                "Scores annuels : {}",
                g.year_scores
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(" · ")
            ),
        );
    }
}

fn contracts(ui: &mut egui::Ui, c: &mut Ctx) {
    let top = ui.cursor().min;
    mark(
        &mut c.ui.marks,
        "contracts_list",
        egui::Rect::from_min_size(top, egui::vec2(ui.available_width(), 220.0)),
    );
    section(ui, "CONTRATS FOURNISSEURS");
    muted(
        ui,
        "Négocier tôt (salle Achats, 1 créneau) donne du levier. Attendre J-2 coûte très cher.",
    );
    let now = c.g().turn();
    let list = c.g().contracts.clone();
    let mut nego = None;
    for st in list.iter().filter(|x| x.active) {
        let Some(def) = c.content.contract(&st.id) else {
            continue;
        };
        let vendor = c
            .content
            .vendor(&def.vendor)
            .map(|v| v.name.clone())
            .unwrap_or_default();
        card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(&def.label).font(bold(13.5)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("{:.2}/mois", st.monthly_cost)).color(YELLOW));
                });
            });
            let left = st.months_left(now);
            let when = dsi_core::state::Calendar::from_abs(st.next_renewal);
            ui.horizontal_wrapped(|ui| {
                muted(ui, vendor);
                ui.label(
                    RichText::new(format!(
                        "échéance : {} A{} ({} mois)",
                        when.month_name(),
                        when.year,
                        left.max(0)
                    ))
                    .size(12.5)
                    .color(if left <= 2 { ORANGE } else { LIGHT }),
                );
                ui.label(
                    RichText::new(format!(
                        "dépendance {:.0} %",
                        st.dependency_weight() * 100.0
                    ))
                    .size(12.5)
                    .color(MUTED),
                );
                if let Some(p) = st.pre_deal {
                    ui.label(
                        RichText::new(format!("pré-accord {:+.0} %", p * 100.0))
                            .size(12.5)
                            .color(GREEN),
                    );
                }
                if st.exit_clause {
                    ui.label(RichText::new("clause de sortie").size(12.5).color(CYAN));
                }
            });
            if st.pre_deal.is_none()
                && (1..=9).contains(&left)
                && ui.small_button("Négocier maintenant (1 créneau)").clicked()
            {
                nego = Some(st.id.clone());
            }
        });
    }
    if let Some(id) = nego {
        c.run(Command::Negotiate(id));
    }
    let ended: Vec<_> = list
        .iter()
        .filter(|x| !x.active)
        .filter_map(|x| c.content.contract(&x.id))
        .map(|d| d.label.clone())
        .collect();
    if !ended.is_empty() {
        section(ui, "RÉSILIÉS");
        muted(ui, ended.join(" · "));
    }
}

fn tech(ui: &mut egui::Ui, c: &mut Ctx) {
    section(ui, "TECHNOLOGY MAP");
    muted(
        ui,
        "■ en place · □ à débloquer (via un projet). Aucune techno n'est magique : chacune a ses effets mensuels, parfois conditionnels.",
    );
    for (branch, label) in dsi_core::technology::BRANCHES {
        let mut techs: Vec<_> = c
            .content
            .techs
            .iter()
            .filter(|t| t.branch == branch)
            .collect();
        techs.sort_by_key(|t| t.tier);
        ui.add_space(4.0);
        ui.label(RichText::new(label).font(pixel(14.0)).color(YELLOW));
        ui.horizontal_wrapped(|ui| {
            for t in techs {
                let owned = c.g().has_tech(&t.id);
                let via = t
                    .via
                    .as_ref()
                    .and_then(|p| c.content.project(p))
                    .map(|p| format!("\nVia le projet : {}", p.name))
                    .unwrap_or_default();
                let txt = RichText::new(format!("{} {}", if owned { "■" } else { "□" }, t.name))
                    .size(12.5)
                    .color(if owned { GREEN } else { MUTED });
                egui::Frame::new()
                    .fill(if owned { CARD_2 } else { CARD })
                    .corner_radius(2)
                    .inner_margin(egui::Margin::symmetric(6, 2))
                    .show(ui, |ui| {
                        ui.label(txt).on_hover_text(format!("{}{via}", t.desc));
                    });
                ui.label(RichText::new("→").size(11.0).color(LINE));
            }
        });
    }
}

fn team(ui: &mut egui::Ui, c: &mut Ctx) {
    section(ui, "ÉQUIPE INTERNE RECRUTÉE");
    if ui
        .button("💻 Voir l'équipe sur le plateau DSI (V)")
        .clicked()
    {
        c.view.plateau = true;
    }
    let g = c.g();
    let hires = dsi_core::people::payroll(g);
    let total = g.staff_payroll + hires;
    let pct = if g.allocation > 0.0 {
        total / g.allocation * 100.0
    } else {
        0.0
    };
    wrap(
        ui,
        RichText::new(format!(
            "Masse salariale : équipe historique {:.2} + recrues {hires:.2} = {total:.2} pt/mois ({pct:.0} % du budget mensuel).",
            g.staff_payroll
        ))
        .size(13.0)
        .color(LIGHT),
    );
    if g.team.is_empty() {
        muted(
            ui,
            "Personne pour l'instant : le SI est tenu par 18 ESN et quelques héros fatigués. Recrutez depuis la salle RH.",
        );
    } else {
        muted(
            ui,
            format!(
                "Licencier arrête le salaire dès ce mois (1 créneau d'agenda). Pendant les {} premiers mois (période d'essai) : sans indemnités ; ensuite : {} mois de salaire d'indemnités. MAKE, équipes et attractivité en pâtissent, d'autant plus si les départs se répètent.",
                dsi_core::people::TRIAL_MONTHS,
                dsi_core::people::SEVERANCE_MONTHS
            ),
        );
    }
    let busy = g.active.is_some() || g.finished();
    let agenda = g.agenda;
    let now = g.turn();
    let team = g.team.clone();
    let mut run: Option<Command> = None;
    for (i, h) in team.iter().enumerate() {
        let role = c.content.careers.roles.iter().find(|r| r.id == h.role);
        let months = now.saturating_sub(h.since);
        let trial = months < dsi_core::people::TRIAL_MONTHS;
        card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(&h.name).font(bold(13.5)));
                if trial {
                    ui.label(RichText::new("PÉRIODE D'ESSAI").font(bold(10.5)).color(CYAN));
                }
            });
            muted(
                ui,
                format!(
                    "{} · {:.2} pt/mois · {months} mois d'ancienneté",
                    role.map(|r| r.label.as_str()).unwrap_or(""),
                    h.salary
                ),
            );
            if let Some(r) = role
                && !r.monthly.is_empty()
            {
                let fx: Vec<String> = r
                    .monthly
                    .iter()
                    .map(|(m, v)| format!("{} {v:+.2}", m.label()))
                    .collect();
                ui.label(RichText::new(format!("Apport mensuel : {}", fx.join(" · "))).size(12.0).color(GREEN));
            }
            if c.ui.fire_pending == Some(i) {
                if let Some(t) = dsi_core::people::dismiss_terms(c.content, c.g(), i) {
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(format!(
                            "Licencier {} ? Économie : {:.2} pt/mois · indemnités : {:.2} pt · 1 créneau d'agenda.",
                            t.name, t.saving, t.severance
                        ))
                        .size(13.0)
                        .color(ORANGE),
                    );
                    let fx: Vec<String> = t
                        .effects
                        .iter()
                        .map(|(m, v)| if m.inverted() || v.fract().abs() > 0.01 { format!("{} {v:+.2}", m.label()) } else { format!("{} {v:+.0}", m.label()) })
                        .collect();
                    ui.add(egui::Label::new(RichText::new(format!("Effets immédiats : {}", fx.join(" · "))).size(12.5).color(LIGHT)).wrap());
                    if t.recent > 0 {
                        ui.label(RichText::new(format!("Déjà {} départ(s) contraint(s) cette année : l'équipe le vivra mal.", t.recent)).size(12.0).color(RED));
                    }
                    ui.horizontal(|ui| {
                        let can = !busy && agenda > 0;
                        let b = ui.add_enabled(can, egui::Button::new(RichText::new("Confirmer le licenciement").color(ORANGE)));
                        let b = if agenda == 0 { b.on_disabled_hover_text("Plus de créneau d'agenda ce mois-ci.") } else { b.on_disabled_hover_text("Terminez d'abord le sujet en cours.") };
                        if b.clicked() {
                            run = Some(Command::Dismiss(i));
                        }
                        if ui.button("Annuler").clicked() {
                            c.ui.fire_pending = None;
                        }
                    });
                }
            } else if ui
                .add_enabled(!busy, egui::Button::new(RichText::new("Licencier…").color(MUTED)).small())
                .on_hover_text("Voir le coût et les conséquences avant de confirmer")
                .clicked()
            {
                c.ui.fire_pending = Some(i);
            }
        });
    }
    if let Some(cmd) = run {
        c.ui.fire_pending = None;
        c.run(cmd);
    }
    let g = c.g();
    section(ui, "PERSONNAGES");
    for ch in &c.content.dialogues.characters {
        if ch.id == "notif" {
            continue;
        }
        ui.horizontal(|ui| {
            portrait(ui, c.tex, ch.portrait, 28.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(
                    RichText::new(&ch.name)
                        .font(bold(13.0))
                        .color(hex(&ch.color)),
                );
                muted(ui, &ch.role);
            });
        });
    }
    let _ = Room::Rh;
}
