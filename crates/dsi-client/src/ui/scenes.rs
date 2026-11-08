//! Les popups : événement, renouvellement, résultat, chapitre, revue annuelle du COMEX,
//! game over et fins.

use super::style::*;
use super::widgets::*;
use super::{Ctx, Screen};
use bevy_egui::egui::{self, Color32, RichText};
use dsi_core::Command;
use dsi_core::state::{Active, Metric};

const KEYS: [egui::Key; 5] = [
    egui::Key::Num1,
    egui::Key::Num2,
    egui::Key::Num3,
    egui::Key::Num4,
    egui::Key::Num5,
];

fn key_choice(ctx: &egui::Context) -> Option<usize> {
    if ctx.egui_wants_keyboard_input() {
        return None;
    }
    ctx.input(|i| KEYS.iter().position(|k| i.key_pressed(*k)))
}

fn enter(ctx: &egui::Context) -> bool {
    !ctx.egui_wants_keyboard_input()
        && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
}

pub fn show(ctx: &egui::Context, c: &mut Ctx) {
    let g = c.g();
    if g.over.is_some() {
        game_over(ctx, c);
        return;
    }
    if g.ending.is_some() {
        ending(ctx, c);
        return;
    }
    match g.active.clone() {
        Some(Active::Event { id }) => event(ctx, c, &id),
        Some(Active::Renewal { contract }) => renewal(ctx, c, &contract),
        Some(Active::Result(r)) => result(ctx, c, &r),
        Some(Active::Chapter { index }) => chapter(ctx, c, index),
        Some(Active::Review(r)) => review(ctx, c, &r),
        Some(Active::Notice { .. }) => {
            c.run(Command::Continue);
        }
        Some(Active::Negotiation(n)) => super::minigames::negotiation(ctx, c, &n),
        Some(Active::Interview(iv)) => super::minigames::interview(ctx, c, &iv),
        Some(Active::WarRoom(w)) => super::minigames::war_room(ctx, c, &w),
        None => {}
    }
}

fn speaker_header(ui: &mut egui::Ui, c: &Ctx, speaker: &str, title: &str, title_color: Color32) {
    ui.horizontal_top(|ui| {
        if !speaker.is_empty() {
            c.face(ui, speaker, 72.0);
        }
        ui.vertical(|ui| {
            ui.add(
                egui::Label::new(RichText::new(title).font(pixel(24.0)).color(title_color)).wrap(),
            );
            if let Some(ch) = c.content.character(speaker) {
                ui.label(
                    RichText::new(format!("{} — {}", ch.name, ch.role))
                        .size(13.0)
                        .color(hex(&ch.color)),
                );
            }
        });
    });
}

fn event(ctx: &egui::Context, c: &mut Ctx, id: &str) {
    let Some(def) = c.content.event(id).cloned() else {
        c.run(Command::Continue);
        return;
    };
    let pressed = key_choice(ctx);
    let color = kind_color(def.kind);
    let mut chosen = None;
    modal(ctx, "event", 660.0, color, |ui| {
        ui.horizontal(|ui| {
            kind_banner(ui, def.kind.label(), color);
            muted(ui, def.room.label());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                muted(ui, c.g().cal.label());
            });
        });
        ui.add_space(4.0);
        let title = c.g().tx(&def.title);
        speaker_header(ui, c, &def.speaker, &title, TEXT);
        ui.add_space(6.0);
        wrap(
            ui,
            RichText::new(c.g().tx(&def.text)).size(15.5).color(LIGHT),
        );
        if !def.immediate.is_empty() {
            let list: Vec<(Metric, f32)> = def.immediate.iter().map(|(m, d)| (*m, *d)).collect();
            ui.add_space(2.0);
            deltas(ui, &list);
        }
        ui.add_space(8.0);
        for (i, ch) in def.choices.iter().enumerate() {
            let g = c.g();
            let allowed = ch.requires.met(g) && g.agenda >= ch.agenda_cost;
            let frame = if allowed { card() } else { card().fill(PANEL) };
            let resp = frame
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{}", i + 1))
                                .font(pixel(15.0))
                                .color(if allowed { YELLOW } else { MUTED }),
                        );
                        ui.add(
                            egui::Label::new(
                                RichText::new(g.tx(&ch.label))
                                    .font(bold(15.0))
                                    .color(if allowed { TEXT } else { MUTED }),
                            )
                            .wrap(),
                        );
                    });
                    if !ch.hint.is_empty() {
                        muted(ui, g.tx(&ch.hint));
                    }
                    if !allowed {
                        let why = if !ch.requires.met(g) {
                            ch.requires.explain(g, &|t| c.content.tech_name(t))
                        } else {
                            "plus de créneau d'agenda".into()
                        };
                        ui.label(RichText::new(format!("🔒 {why}")).size(12.0).color(ORANGE));
                    } else {
                        let mut pv = ch.preview();
                        if ch.outcome.start_project.is_some() {
                            ui.label(RichText::new("▶ lance un projet").size(11.5).color(CYAN));
                        }
                        if ch.agenda_cost > 0 {
                            ui.label(
                                RichText::new(format!("⏱ {} créneau(x) d'agenda", ch.agenda_cost))
                                    .size(11.5)
                                    .color(YELLOW),
                            );
                        }
                        if let Some(m) = &ch.outcome.minigame {
                            let t = match m {
                                dsi_core::effects::Minigame::Negotiation { .. } => {
                                    "◆ mini-jeu : négociation"
                                }
                                dsi_core::effects::Minigame::Recruit { .. } => {
                                    "◆ mini-jeu : entretien"
                                }
                                dsi_core::effects::Minigame::WarRoom { .. } => {
                                    "◆ mini-jeu : war room"
                                }
                            };
                            ui.label(RichText::new(t).size(11.5).color(PURPLE));
                        }
                        pv.retain(|(_, d)| d.abs() >= 0.5 || *d == 0.0);
                        preview(ui, &pv, ch.has_hidden() && ch.outcome.minigame.is_none());
                    }
                })
                .response
                .interact(egui::Sense::click());
            if allowed
                && resp
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
            {
                chosen = Some(i);
            }
            ui.add_space(3.0);
        }
    });
    if let Some(k) = pressed
        && k < def.choices.len()
    {
        chosen = Some(k);
    }
    if let Some(i) = chosen {
        c.run(Command::Choose(i));
    }
}

fn renewal(ctx: &egui::Context, c: &mut Ctx, contract: &str) {
    let Some(def) = c.content.contract(contract).cloned() else {
        return;
    };
    let vendor = c
        .content
        .vendor(&def.vendor)
        .map(|v| v.name.clone())
        .unwrap_or_default();
    let choices = dsi_core::game::renewal_choices(c.content, c.g(), contract);
    let pressed = key_choice(ctx);
    let mut chosen = None;
    let cost = c
        .g()
        .contracts
        .iter()
        .find(|x| x.id == contract)
        .map(|x| x.monthly_cost)
        .unwrap_or(0.0);
    modal(ctx, "renewal", 640.0, BLUE, |ui| {
        ui.horizontal(|ui| {
            kind_banner(ui, "CONTRACT RENEWAL", BLUE);
            muted(ui, "Vendor meeting room");
        });
        speaker_header(
            ui,
            c,
            "vendor",
            &format!("Renouvellement : {}", def.label),
            TEXT,
        );
        ui.label(RichText::new(&vendor).size(13.0).color(BLUE));
        wrap(
            ui,
            RichText::new(format!("« {} »", def.pitch))
                .size(15.5)
                .italics()
                .color(LIGHT),
        );
        ui.label(
            RichText::new(format!(
                "Coût actuel : {cost:.2}/mois → demandé : {:.2}/mois (+{:.0} %)",
                cost * (1.0 + def.increase),
                def.increase * 100.0
            ))
            .font(bold(13.5))
            .color(YELLOW),
        );
        muted(
            ui,
            "Vous êtes à l'échéance : sans préparation, le rapport de force est faible.",
        );
        ui.add_space(6.0);
        for (i, (label, hint, ok)) in choices.iter().enumerate() {
            let resp = (if *ok { card() } else { card().fill(PANEL) })
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{}", i + 1))
                                .font(pixel(15.0))
                                .color(if *ok { YELLOW } else { MUTED }),
                        );
                        ui.label(RichText::new(label).font(bold(15.0)).color(if *ok {
                            TEXT
                        } else {
                            MUTED
                        }));
                    });
                    muted(ui, hint);
                })
                .response
                .interact(egui::Sense::click());
            if *ok && resp.clicked() {
                chosen = Some(i);
            }
        }
    });
    if let Some(k) = pressed
        && k < choices.len()
    {
        chosen = Some(k);
    }
    if let Some(i) = chosen {
        c.run(Command::Choose(i));
    }
}

fn result(ctx: &egui::Context, c: &mut Ctx, r: &dsi_core::state::Resolution) {
    let go = enter(ctx)
        || (!ctx.egui_wants_keyboard_input()
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Space)));
    let mut cont = false;
    modal(ctx, "result", 560.0, YELLOW, |ui| {
        ui.label(RichText::new(&r.title).font(pixel(22.0)).color(YELLOW));
        if !r.text.is_empty() {
            ui.add_space(4.0);
            if r.speaker.is_empty() {
                wrap(ui, RichText::new(&r.text).size(15.5).color(TEXT));
            } else {
                ui.horizontal_top(|ui| {
                    c.face(ui, &r.speaker, 72.0);
                    ui.vertical(|ui| {
                        if let Some((name, col)) = c.speaker_name(&r.speaker) {
                            ui.label(RichText::new(name).font(pixel(14.0)).color(col));
                        }
                        wrap(
                            ui,
                            RichText::new(format!("« {} »", r.text))
                                .size(15.5)
                                .color(TEXT),
                        );
                    });
                });
            }
        }
        ui.add_space(6.0);
        if r.deltas.is_empty() {
            muted(ui, "Aucun effet immédiat visible.");
        } else {
            deltas(ui, &r.deltas);
        }
        for n in &r.notes {
            ui.label(RichText::new(format!("• {n}")).size(13.0).color(LIGHT));
        }
        if r.delayed_hint {
            ui.label(
                RichText::new(
                    "⏳ Cette décision pourrait avoir des suites dans les mois qui viennent…",
                )
                .size(13.0)
                .color(PURPLE),
            );
        }
        ui.add_space(8.0);
        if big_button(ui, "Continuer", true, 200.0)
            .on_hover_text("Entrée")
            .clicked()
        {
            cont = true;
        }
    });
    if cont || go {
        c.run(Command::Continue);
    }
}

fn chapter(ctx: &egui::Context, c: &mut Ctx, index: usize) {
    let Some(ch) = c.content.dialogues.chapters.get(index).cloned() else {
        c.run(Command::Continue);
        return;
    };
    let mut cont = enter(ctx);
    modal(ctx, "chapter", 660.0, YELLOW, |ui| {
        muted(
            ui,
            format!("{} · {}", c.g().cal.label(), c.g().profile.company),
        );
        ui.label(RichText::new(&ch.title).font(pixel(30.0)).color(YELLOW));
        ui.label(
            RichText::new(&ch.subtitle)
                .size(15.0)
                .italics()
                .color(LIGHT),
        );
        ui.add_space(8.0);
        for l in &ch.lines {
            ui.horizontal_top(|ui| {
                c.face(ui, &l.speaker, 56.0);
                ui.vertical(|ui| {
                    if let Some((name, col)) = c.speaker_name(&l.speaker) {
                        ui.label(RichText::new(name).font(pixel(13.0)).color(col));
                    }
                    wrap(ui, RichText::new(c.g().tx(&l.text)).size(15.0).color(TEXT));
                });
            });
            ui.add_space(4.0);
        }
        ui.add_space(6.0);
        if big_button(ui, "C'est parti", true, 220.0)
            .on_hover_text("Entrée")
            .clicked()
        {
            cont = true;
        }
    });
    if cont {
        c.run(Command::Continue);
    }
}

fn review(ctx: &egui::Context, c: &mut Ctx, r: &dsi_core::score::YearReview) {
    let mut cont = false;
    modal(ctx, "review", 620.0, ORANGE, |ui| {
        kind_banner(ui, "COMEX MEETING", ORANGE);
        speaker_header(ui, c, "dg", &format!("Bilan de l'année {}", r.year), TEXT);
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{}", r.score))
                    .font(pixel(48.0))
                    .color(YELLOW),
            );
            ui.vertical(|ui| {
                muted(ui, "Score / 100");
                ui.label(
                    RichText::new(format!("« {} »", r.rank))
                        .font(bold(17.0))
                        .color(TEXT),
                );
            });
        });
        ui.add_space(6.0);
        section(ui, "OBJECTIFS DE L'ANNÉE");
        for (label, ok) in &r.objectives {
            ui.label(
                RichText::new(format!("{} {label}", if *ok { "✔" } else { "×" }))
                    .size(14.5)
                    .color(if *ok { GREEN } else { RED }),
            );
        }
        let diff = if r.baseline > 0.0 {
            (r.run_cost / r.baseline - 1.0) * 100.0
        } else {
            0.0
        };
        muted(
            ui,
            format!(
                "Coût de fonctionnement : {:.2}/mois ({diff:+.0} % depuis janvier)",
                r.run_cost
            ),
        );
        ui.add_space(6.0);
        wrap(
            ui,
            RichText::new(&r.verdict).size(15.0).italics().color(LIGHT),
        );
        let (t, col) = delta_text(Metric::Comex, r.comex_delta);
        ui.label(
            RichText::new(format!("Confiance du COMEX : {t}"))
                .font(bold(14.0))
                .color(col),
        );
        ui.add_space(8.0);
        let label = if r.last {
            "Découvrir la fin"
        } else {
            "Année suivante"
        };
        if big_button(ui, label, true, 240.0).clicked() {
            cont = true;
        }
    });
    if cont || enter(ctx) {
        c.run(Command::Continue);
    }
}

fn final_stats(ui: &mut egui::Ui, c: &Ctx) {
    let g = c.g();
    egui::Grid::new("final")
        .num_columns(4)
        .spacing([10.0, 3.0])
        .show(ui, |ui| {
            for (i, (m, _)) in dsi_core::score::SCORE_PARTS.iter().enumerate() {
                let v = g.m(*m);
                ui.label(RichText::new(m.short()).size(12.0).color(MUTED));
                ui.label(
                    RichText::new(format!("{v:.0}"))
                        .font(bold(13.0))
                        .color(metric_color(*m, v)),
                );
                if i % 2 == 1 {
                    ui.end_row();
                }
            }
        });
    let decisions = g.decisions.len();
    muted(
        ui,
        format!(
            "{decisions} décisions · {} projets livrés · {} recrutements · graine {}",
            g.done_projects.len(),
            g.team.len(),
            g.seed
        ),
    );
}

fn end_buttons(ui: &mut egui::Ui, c: &mut Ctx) {
    ui.horizontal(|ui| {
        if big_button(ui, "Rejouer", true, 180.0).clicked() {
            c.session.game = None;
            c.ui.screen = Screen::Create;
        }
        if ghost_button(ui, "Même graine", 140.0)
            .on_hover_text("Rejouer exactement les mêmes aléas")
            .clicked()
        {
            let seed = c.g().seed;
            let profile = c.g().profile.clone();
            c.session.new_game(c.content, seed, profile);
        }
        if ghost_button(ui, "Écran titre", 120.0).clicked() {
            c.session.game = None;
            c.ui.screen = Screen::Title;
        }
    });
}

fn game_over(ctx: &egui::Context, c: &mut Ctx) {
    let Some(o) = c.g().over else { return };
    modal(ctx, "gameover", 600.0, RED, |ui| {
        kind_banner(ui, "GAME OVER", RED);
        ui.label(RichText::new(o.title()).font(pixel(34.0)).color(RED));
        muted(ui, c.g().cal.label());
        ui.add_space(4.0);
        wrap(ui, RichText::new(o.text()).size(15.5).color(TEXT));
        ui.add_space(8.0);
        let sc = dsi_core::score::score(c.g());
        ui.label(
            RichText::new(format!(
                "Score : {sc}/100 — « {} »",
                dsi_core::score::rank(sc)
            ))
            .font(bold(16.0))
            .color(YELLOW),
        );
        final_stats(ui, c);
        ui.add_space(8.0);
        end_buttons(ui, c);
    });
}

fn ending(ctx: &egui::Context, c: &mut Ctx) {
    let Some(e) = c.g().ending.clone() else {
        return;
    };
    modal(ctx, "ending", 640.0, GREEN, |ui| {
        kind_banner(ui, "ÉPILOGUE — FIN DE PARTIE", GREEN);
        ui.label(RichText::new(&e.title).font(pixel(34.0)).color(GREEN));
        ui.add_space(4.0);
        wrap(ui, RichText::new(&e.text).size(15.5).color(TEXT));
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{}", e.score))
                    .font(pixel(48.0))
                    .color(YELLOW),
            );
            ui.vertical(|ui| {
                muted(ui, "Score final / 100");
                ui.label(
                    RichText::new(format!("« {} »", e.rank))
                        .font(bold(17.0))
                        .color(TEXT),
                );
            });
        });
        final_stats(ui, c);
        muted(
            ui,
            "Fins possibles : DSI souveraine · Software Company · Cloud Native Enterprise · Engineering Culture · Agentic Enterprise · Pragmatic CIO (la plus difficile).",
        );
        let past = c.g().past_endings.clone();
        if !past.is_empty() {
            ui.add_space(4.0);
            section(ui, "FINS PRÉCÉDENTES");
            for p in &past {
                muted(
                    ui,
                    format!("Année 3 : « {} » — score {} ({})", p.title, p.score, p.rank),
                );
            }
        }
        ui.add_space(8.0);
        if c.g().extension == 0 {
            let years = c.content.dialogues.extension_years;
            card().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(format!("SAISON 2 — {years} ANS DE PLUS")).font(pixel(16.0)).color(ORANGE));
                wrap(
                    ui,
                    RichText::new(format!(
                        "Le conseil vous propose de rester. Vous gardez votre SI, votre équipe, vos contrats et vos indicateurs, et vous continuez jusqu'à la fin de l'année {} : NIS2, règlement IA, fusions, cryptographie post-quantique et entreprise agentique. Une nouvelle fin sera calculée à l'issue de la saison.",
                        c.content.dialogues.campaign_length + years
                    ))
                    .size(14.0)
                    .color(LIGHT),
                );
                ui.add_space(4.0);
                if big_button(ui, &format!("Continuer {years} années de plus ▶"), true, 320.0).clicked() {
                    c.run(Command::ExtendCampaign);
                }
            });
            ui.add_space(6.0);
        }
        end_buttons(ui, c);
    });
}
