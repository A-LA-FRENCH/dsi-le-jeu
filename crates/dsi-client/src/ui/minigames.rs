//! Mini-jeux : négociation commerciale (cartes), entretien de recrutement, WAR ROOM.

use super::Ctx;
use super::style::*;
use super::widgets::*;
use bevy_egui::egui::{self, RichText};
use dsi_core::Command;
use dsi_core::negotiation::{Negotiation, Who};
use dsi_core::people::Interview;
use dsi_core::security::{self, WarAction, WarRoom};

pub fn negotiation(ctx: &egui::Context, c: &mut Ctx, n: &Negotiation) {
    let mut play: Option<String> = None;
    let mut conclude = false;
    let mut accept = false;
    let label = c
        .content
        .contract(&n.contract)
        .map(|d| d.label.clone())
        .unwrap_or_default();
    modal(ctx, "negotiation", 860.0, BLUE, |ui| {
        ui.horizontal(|ui| {
            kind_banner(ui, "VENDOR MEETING", BLUE);
            muted(
                ui,
                if n.early {
                    "Négociation anticipée"
                } else {
                    "Renouvellement à échéance"
                },
            );
        });
        ui.label(
            RichText::new(format!("{} — {}", n.vendor, label))
                .font(pixel(24.0))
                .color(TEXT),
        );
        ui.add_space(4.0);
        ui.columns(2, |cols| {
            // Colonne gauche : la conversation.
            let ui = &mut cols[0];
            ui.horizontal(|ui| {
                portrait(ui, c.tex, c.portrait_of("vendor"), 56.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Jean-Christophe")
                            .font(pixel(14.0))
                            .color(ORANGE),
                    );
                    muted(
                        ui,
                        "Key Account Manager — « Ce partenariat est stratégique. »",
                    );
                });
            });
            egui::ScrollArea::vertical()
                .max_height(250.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for l in &n.log {
                        let (col, prefix) = match l.who {
                            Who::You => (YELLOW, "Vous : "),
                            Who::Vendor => (LIGHT, "Commercial : "),
                            Who::Hazard => (PURPLE, "⚡ "),
                        };
                        let delta = if l.delta.abs() >= 0.5 {
                            format!("  [{:+.0} levier]", l.delta)
                        } else {
                            String::new()
                        };
                        ui.add(
                            egui::Label::new(
                                RichText::new(format!("{prefix}{}{delta}", l.text))
                                    .size(13.5)
                                    .color(col),
                            )
                            .wrap(),
                        );
                    }
                });
            // Colonne droite : rapport de force et cartes.
            let ui = &mut cols[1];
            section(ui, "RAPPORT DE FORCE");
            let lev = n.leverage;
            ui.horizontal(|ui| {
                gauge(
                    ui,
                    lev / 100.0,
                    if lev >= 60.0 {
                        GREEN
                    } else if lev >= 35.0 {
                        YELLOW
                    } else {
                        RED
                    },
                    220.0,
                    14.0,
                );
                ui.label(RichText::new(format!("{lev:.0}")).font(bold(16.0)));
            });
            let inc = n.projected_increase() * 100.0;
            ui.label(
                RichText::new(format!("Si vous concluez maintenant : {:+.0} %", inc))
                    .font(bold(15.0))
                    .color(if inc <= 0.0 {
                        GREEN
                    } else if inc < 30.0 {
                        YELLOW
                    } else {
                        RED
                    }),
            );
            muted(
                ui,
                format!("Hausse demandée : +{:.0} %", n.base_increase * 100.0),
            );
            egui::Grid::new("nego_info")
                .num_columns(2)
                .spacing([10.0, 2.0])
                .show(ui, |ui| {
                    for (k, v) in [
                        ("Dépendance", format!("{:.0} %", n.dependency * 100.0)),
                        (
                            "Temps avant échéance",
                            format!("{} mois", n.months_left.max(0)),
                        ),
                        ("Alternatives crédibles", n.alternatives.to_string()),
                        (
                            "Coût de migration estimé",
                            if n.migration_cost > 0.0 {
                                format!("{:.0} pts", n.migration_cost)
                            } else {
                                "—".into()
                            },
                        ),
                        ("Risque opérationnel", format!("{:.0}/100", n.op_risk)),
                        (
                            "Crédibilité d'une menace de départ",
                            format!("{:.0} %", n.credibility * 100.0),
                        ),
                        ("Tours restants", n.rounds_left.to_string()),
                    ] {
                        ui.label(RichText::new(k).size(12.5).color(MUTED));
                        ui.label(RichText::new(v).size(12.5));
                        ui.end_row();
                    }
                });
        });
        section(ui, "VOS CARTES");
        for row in n.hand.chunks(4) {
            ui.horizontal(|ui| {
                for id in row {
                    let Some(card_def) = c.content.card(id) else {
                        continue;
                    };
                    let used = n.played.contains(id);
                    let enabled = !used && n.rounds_left > 0;
                    let lev = if card_def.credibility_scaled {
                        "± crédibilité".to_string()
                    } else {
                        format!("{:+.0}", card_def.leverage)
                    };
                    let resp =
                        egui::Frame::new()
                            .fill(if enabled { CARD_2 } else { PANEL })
                            .stroke(egui::Stroke::new(1.0, if enabled { BLUE } else { LINE }))
                            .corner_radius(3)
                            .inner_margin(egui::Margin::same(6))
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    ui.set_width(178.0);
                                    ui.set_height(70.0);
                                    ui.spacing_mut().item_spacing.y = 2.0;
                                    let title = if used {
                                        format!("✔ {}", card_def.label)
                                    } else {
                                        card_def.label.clone()
                                    };
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(title)
                                                .font(bold(13.0))
                                                .color(if enabled { TEXT } else { MUTED }),
                                        )
                                        .wrap(),
                                    );
                                    ui.label(RichText::new(lev).size(12.0).color(if enabled {
                                        YELLOW
                                    } else {
                                        MUTED
                                    }));
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(&card_def.text).size(11.0).color(MUTED),
                                        )
                                        .wrap(),
                                    );
                                });
                            })
                            .response
                            .interact(egui::Sense::click());
                    if enabled
                        && resp
                            .on_hover_cursor(egui::CursorIcon::PointingHand)
                            .clicked()
                    {
                        play = Some(id.clone());
                    }
                }
            });
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if big_button(ui, "Conclure l'accord", true, 220.0).clicked() {
                conclude = true;
            }
            if ghost_button(
                ui,
                &format!("Accepter tel quel (+{:.0} %)", n.base_increase * 100.0),
                220.0,
            )
            .clicked()
            {
                accept = true;
            }
        });
    });
    if let Some(card) = play {
        c.run(Command::PlayCard(card));
    } else if conclude {
        c.run(Command::Conclude);
    } else if accept {
        c.run(Command::AcceptOffer);
    }
}

pub fn interview(ctx: &egui::Context, c: &mut Ctx, iv: &Interview) {
    let mut answer = None;
    let mut offer: Option<Option<usize>> = None;
    let role = c
        .content
        .careers
        .roles
        .iter()
        .find(|r| r.id == iv.role)
        .cloned();
    let q = dsi_core::people::current_question(c.content, iv).cloned();
    let face = [5u32, 11, 12, 10, 2, 9, 3, 4][iv.name.len() % 8];
    let pressed = if ctx.egui_wants_keyboard_input() {
        None
    } else {
        ctx.input(|i| {
            [
                egui::Key::Num1,
                egui::Key::Num2,
                egui::Key::Num3,
                egui::Key::Num4,
            ]
            .iter()
            .position(|k| i.key_pressed(*k))
        })
    };
    modal(ctx, "interview", 700.0, PURPLE, |ui| {
        ui.horizontal(|ui| {
            kind_banner(ui, "RECRUTEMENT", PURPLE);
            muted(ui, "Ressources humaines");
        });
        ui.horizontal_top(|ui| {
            portrait(ui, c.tex, face, 64.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(&iv.name).font(pixel(22.0)).color(TEXT));
                if let Some(r) = &role {
                    ui.label(
                        RichText::new(format!(
                            "{} · {:.2} pt/mois au prix du marché",
                            r.label, r.salary
                        ))
                        .color(PURPLE),
                    );
                }
                if !iv.quirk.is_empty() {
                    muted(ui, &iv.quirk);
                }
            });
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Intérêt du candidat").size(13.0).color(MUTED));
            gauge(
                ui,
                iv.interest / 100.0,
                if iv.interest >= 55.0 {
                    GREEN
                } else if iv.interest >= 35.0 {
                    YELLOW
                } else {
                    RED
                },
                200.0,
                12.0,
            );
            ui.label(RichText::new(format!("{:.0}", iv.interest)).font(bold(14.0)));
        });
        ui.add_space(4.0);
        for (you, line) in iv.lines.iter().skip(1) {
            ui.add(
                egui::Label::new(
                    RichText::new(if *you {
                        format!("Vous : {line}")
                    } else {
                        format!("{} : {line}", iv.name.split(' ').next().unwrap_or(""))
                    })
                    .size(13.5)
                    .color(if *you { YELLOW } else { LIGHT }),
                )
                .wrap(),
            );
        }
        ui.add_space(6.0);
        if iv.gone {
            ui.label(
                RichText::new("Le candidat a disparu de l'écran.")
                    .font(bold(16.0))
                    .color(RED),
            );
            if big_button(ui, "Fermer", true, 160.0).clicked() {
                offer = Some(None);
            }
        } else if let Some(q) = q {
            ui.label(
                RichText::new(format!("« {} »", q.prompt))
                    .font(bold(16.0))
                    .color(TEXT),
            );
            for (i, a) in q.answers.iter().enumerate() {
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(format!("{}. {}", i + 1, a.text)).size(14.5),
                        )
                        .min_size(egui::vec2(ui.available_width(), 30.0)),
                    )
                    .clicked()
                {
                    answer = Some(i);
                }
            }
            if let Some(k) = pressed
                && k < q.answers.len()
            {
                answer = Some(k);
            }
        } else {
            section(ui, "VOTRE OFFRE");
            for (i, o) in c.content.careers.offers.iter().enumerate() {
                let salary = role
                    .as_ref()
                    .map(|r| r.salary * o.salary_mult)
                    .unwrap_or(0.0);
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(format!(
                                "{}. {} — {:.2} pt/mois · {}",
                                i + 1,
                                o.label,
                                salary,
                                o.hint
                            ))
                            .size(14.5),
                        )
                        .min_size(egui::vec2(ui.available_width(), 30.0)),
                    )
                    .clicked()
                {
                    offer = Some(Some(i));
                }
            }
            if ui.button("Remercier le candidat").clicked() {
                offer = Some(None);
            }
            if let Some(k) = pressed
                && k < c.content.careers.offers.len()
            {
                offer = Some(Some(k));
            }
        }
    });
    if let Some(i) = answer {
        c.run(Command::Answer(i));
    } else if let Some(o) = offer {
        c.run(Command::MakeOffer(o));
    }
}

pub fn war_room(ctx: &egui::Context, c: &mut Ctx, w: &WarRoom) {
    let mut act = None;
    let mut close = false;
    let title = c
        .content
        .crisis(&w.scenario)
        .map(|d| d.title.clone())
        .unwrap_or_default();
    let finished = security::done(w);
    modal(ctx, "warroom", 820.0, RED, |ui| {
        ui.horizontal(|ui| {
            kind_banner(ui, "WAR ROOM", RED);
            muted(ui, "Salle de crise — tout le monde parle en même temps");
        });
        ui.label(RichText::new(&title).font(pixel(24.0)).color(RED));
        ui.columns(2, |cols| {
            let ui = &mut cols[0];
            ui.horizontal(|ui| {
                ui.label(RichText::new("Propagation").size(13.0).color(MUTED));
                gauge(
                    ui,
                    w.spread / 100.0,
                    if w.spread > 60.0 {
                        RED
                    } else if w.spread > 30.0 {
                        ORANGE
                    } else {
                        YELLOW
                    },
                    200.0,
                    14.0,
                );
                ui.label(
                    RichText::new(format!("{:.0} %", w.spread))
                        .font(bold(14.0))
                        .color(RED),
                );
            });
            ui.label(
                RichText::new(format!("⏱ {} heure(s) d'action restante(s)", w.hours_left))
                    .font(bold(14.0))
                    .color(YELLOW),
            );
            for (ok, l) in [
                (w.contained, "Menace isolée"),
                (w.investigated, "Cause identifiée"),
                (w.patched, "Faille corrigée"),
                (w.restored, "Services restaurés"),
                (w.communicated, "Utilisateurs informés"),
                (w.anssi, "ANSSI / CERT sollicités"),
                (w.comex_informed, "COMEX prévenu"),
            ] {
                ui.label(
                    RichText::new(format!("{} {l}", if ok { "✔" } else { "·" }))
                        .size(13.5)
                        .color(if ok { GREEN } else { MUTED }),
                );
            }
            let ui = &mut cols[1];
            egui::ScrollArea::vertical()
                .max_height(250.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for l in &w.log {
                        ui.add(
                            egui::Label::new(
                                RichText::new(format!("» {l}")).size(13.0).color(LIGHT),
                            )
                            .wrap(),
                        );
                    }
                });
        });
        ui.add_space(6.0);
        if finished {
            ui.label(
                RichText::new(if w.spread >= 100.0 {
                    "Le SI est tombé."
                } else {
                    "Il est temps de clôturer la crise."
                })
                .font(bold(15.0))
                .color(YELLOW),
            );
            if big_button(ui, "Clôturer la crise", true, 240.0).clicked() {
                close = true;
            }
        } else {
            section(ui, "PRIORISER — CHAQUE ACTION PREND UNE HEURE");
            ui.horizontal_wrapped(|ui| {
                for a in WarAction::ALL {
                    let done = match a {
                        WarAction::Isolate => w.contained,
                        WarAction::Investigate => w.investigated,
                        WarAction::Patch => w.patched,
                        WarAction::Restore => w.restored,
                        WarAction::Communicate => w.communicated,
                        WarAction::CallAnssi => w.anssi,
                        WarAction::InformComex => w.comex_informed,
                    };
                    let b = ui
                        .add_enabled(
                            !done,
                            egui::Button::new(RichText::new(a.label()).size(14.0))
                                .min_size(egui::vec2(190.0, 32.0)),
                        )
                        .on_hover_text(a.hint());
                    if b.clicked() {
                        act = Some(a);
                    }
                }
            });
            if ui
                .small_button("Clôturer maintenant")
                .on_hover_text("Arrêter la crise dans l'état actuel.")
                .clicked()
            {
                close = true;
            }
        }
    });
    if let Some(a) = act {
        c.run(Command::War(a));
    } else if close {
        c.run(Command::WarClose);
    }
}
