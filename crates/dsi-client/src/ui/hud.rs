//! Barre du haut (date, jauges, agenda), barre du bas (projets, mois suivant),
//! surcouche du monde (enseigne, survol des salles, changement de mois), raccourcis.

use super::style::*;
use super::widgets::*;
use super::{Ctx, Tab};
use crate::world::{self, WorldViewport};
use bevy_egui::egui::{self, Color32, RichText};
use dsi_core::state::{Metric, Room};
use dsi_core::{Command, projects};

fn chapter_title(c: &Ctx) -> String {
    let g = c.g();
    let now = g.turn();
    c.content
        .dialogues
        .chapters
        .iter()
        .rfind(|ch| {
            dsi_core::state::Calendar {
                year: ch.at.year,
                month: ch.at.month,
            }
            .abs()
                <= now
        })
        .map(|ch| ch.title.clone())
        .unwrap_or_default()
}

/// Ordre d'affichage : d'abord les jauges qui peuvent mettre fin à la partie.
const HUD_ORDER: [Metric; 18] = [
    Metric::Comex,
    Metric::Budget,
    Metric::Security,
    Metric::Debt,
    Metric::VendorDep,
    Metric::Make,
    Metric::Complexity,
    Metric::Uptime,
    Metric::DevSat,
    Metric::UserSat,
    Metric::Velocity,
    Metric::Sovereignty,
    Metric::Resilience,
    Metric::Attractiveness,
    Metric::DataQuality,
    Metric::CloudMaturity,
    Metric::DevopsMaturity,
    Metric::AiMaturity,
];

/// Libellé compact pour la barre du haut.
fn hud_label(m: Metric) -> &'static str {
    match m {
        Metric::Security => "SÉCU",
        Metric::DevSat => "ÉQUIPES",
        Metric::Sovereignty => "SOUVER.",
        Metric::Complexity => "COMPLEX.",
        Metric::Attractiveness => "ATTRACT.",
        Metric::Resilience => "RÉSIL.",
        _ => m.short(),
    }
}

/// Seuil de défaite éventuel : (valeur fatale, texte).
fn lethal(m: Metric) -> Option<&'static str> {
    Some(match m {
        Metric::Comex => "À 0 : vous êtes viré.",
        Metric::Budget => "À 0 : Bankruptcy as a Service.",
        Metric::Security => "À 0 : Cyber Apocalypse.",
        Metric::Debt => "À 100 : Legacy Forever.",
        Metric::VendorDep => "À 100 : Vendor Lock-In.",
        Metric::Make => "À 0 : Nobody Knows Anything.",
        Metric::Complexity => "À 100 : Kubernetes Singularity.",
        _ => return None,
    })
}

fn metric_help(m: Metric) -> &'static str {
    match m {
        Metric::Budget => {
            "Budget IT disponible. Allocation mensuelle moins les coûts de fonctionnement et de projets."
        }
        Metric::Security => "Niveau de sécurité : MFA, correctifs, sauvegardes, détection.",
        Metric::Debt => {
            "Dette technique : chaque modification coûte plus cher et casse plus souvent."
        }
        Metric::Comex => "Confiance du COMEX et de la DG dans la DSI.",
        Metric::DevSat => "Satisfaction et énergie des équipes d'ingénierie.",
        Metric::Uptime => {
            "Disponibilité du SI. Les pannes font baisser les utilisateurs et le COMEX."
        }
        Metric::Velocity => "Vélocité : capacité à livrer vite ce que demandent les métiers.",
        Metric::UserSat => "Satisfaction des utilisateurs métiers.",
        Metric::VendorDep => {
            "Dépendance aux fournisseurs : plus elle est haute, moins vous négociez."
        }
        Metric::Sovereignty => {
            "Maîtrise de vos données, de vos clés et de vos dépendances extraterritoriales."
        }
        Metric::Make => {
            "Capacité interne : ce que vos équipes savent construire et maintenir elles-mêmes."
        }
        Metric::Complexity => {
            "Complexité du SI : nombre de briques, d'abstractions et d'intégrations."
        }
        Metric::Attractiveness => {
            "Attractivité pour les candidats : facilite et accélère les recrutements."
        }
        Metric::DataQuality => "Qualité et gouvernance des données. Indispensable à l'IA.",
        Metric::CloudMaturity => "Maturité cloud : FinOps, automatisation, architecture.",
        Metric::DevopsMaturity => "Maturité DevOps : CI/CD, infrastructure as code, SRE.",
        Metric::AiMaturity => "Maturité IA : cas d'usage en production, évaluation, garde-fous.",
        Metric::Resilience => {
            "Résilience : PRA, sauvegardes testées, multi-site, gestion de crise."
        }
    }
}

/// Nombre de colonnes de jauges selon la largeur disponible.
fn metric_cols(width: f32) -> usize {
    if width >= 9.0 * 66.0 { 9 } else { 6 }
}

/// Hauteur de la barre du haut pour une largeur d'écran donnée.
pub fn top_bar_height(screen_w: f32) -> f32 {
    if metric_cols(screen_w - 380.0) == 9 {
        64.0
    } else {
        88.0
    }
}

pub fn top_bar(root: &mut egui::Ui, c: &mut Ctx) {
    let h = top_bar_height(root.ctx().content_rect().width());
    egui::Panel::top("hud").exact_size(h).frame(egui::Frame::new().fill(NEAR_BLACK).inner_margin(egui::Margin::symmetric(12, 5))).show(root, |ui| {
        ui.horizontal_centered(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(RichText::new("DSI — LE JEU").font(pixel(16.0)).color(YELLOW));
                let g = c.g();
                ui.label(RichText::new(g.cal.label()).font(bold(13.0)).color(TEXT)).on_hover_text(chapter_title(c));
                if g.extension > 0 {
                    ui.label(RichText::new("SAISON 2").font(pixel(10.0)).color(ORANGE))
                        .on_hover_text(format!("Partie prolongée jusqu'à la fin de l'année {}.", dsi_core::score::final_year(c.content, g)));
                }
            });
            ui.add_space(8.0);
            let width = ui.available_width() - 150.0;
            let cols = metric_cols(width);
            let cell_w = (width / cols as f32 - 8.0).clamp(54.0, 112.0);
            let pulse = (ui.input(|i| i.time) * 4.0).sin() > 0.0;
            let gr = ui
                .vertical(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 3.0);
                    for row in HUD_ORDER.chunks(cols) {
                        ui.horizontal(|ui| {
                            for &m in row {
                                let v = c.g().m(m);
                                let col = metric_color(m, v);
                                let good = if m.inverted() { 100.0 - v } else { v };
                                let fatal = lethal(m);
                                let danger = fatal.is_some() && good < 20.0;
                                let label_col = if danger && pulse { RED } else if fatal.is_some() { LIGHT } else { MUTED };
                                ui.vertical(|ui| {
                                    ui.set_width(cell_w);
                                    ui.spacing_mut().item_spacing.y = 1.0;
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 3.0;
                                        ui.label(RichText::new(hud_label(m)).font(bold(10.5)).color(label_col));
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(RichText::new(format!("{v:.0}")).font(bold(12.5)).color(col));
                                        });
                                    });
                                    let tip = match fatal {
                                        Some(f) => format!("{} : {v:.1}\n{}\n{f}", m.label(), metric_help(m)),
                                        None => format!("{} : {v:.1}\n{}", m.label(), metric_help(m)),
                                    };
                                    gauge(ui, v / 100.0, col, cell_w, 6.0).on_hover_text(tip);
                                });
                            }
                        });
                    }
                })
                .response
                .rect;
            mark(&mut c.ui.marks, "gauges", gr);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(RichText::new("⚙").size(15.0)).on_hover_text("Réglages (Échap)").clicked() {
                    c.ui.show_settings = true;
                }
                if ui.button(RichText::new("?").size(15.0)).on_hover_text("Aide").clicked() {
                    c.ui.show_help = true;
                }
                ui.add_space(4.0);
                let g = c.g();
                let mut slots = String::new();
                for i in 0..g.agenda_max {
                    slots.push(if i < g.agenda { '●' } else { '○' });
                }
                let ar = ui
                    .vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(RichText::new("AGENDA").font(bold(10.5)).color(MUTED));
                        ui.label(RichText::new(slots).size(15.0).color(if g.agenda > 0 { YELLOW } else { MUTED }))
                            .on_hover_text("Créneaux d'agenda de ce mois : votre temps. Chaque action dans une salle en consomme un.");
                    })
                    .response
                    .rect;
                mark(&mut c.ui.marks, "agenda", ar);
            });
        });
    });
}

pub fn bottom_bar(root: &mut egui::Ui, c: &mut Ctx) {
    egui::Panel::bottom("projects").exact_size(118.0).frame(egui::Frame::new().fill(NEAR_BLACK).inner_margin(egui::Margin::symmetric(12, 8))).show(root, |ui| {
        ui.horizontal(|ui| {
            let right_w = 300.0;
            let avail = ui.available_width() - right_w - 12.0;
            let pr = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(avail, 102.0));
            mark(&mut c.ui.marks, "projects_bar", pr);
            ui.allocate_ui_with_layout(egui::vec2(avail, 102.0), egui::Layout::left_to_right(egui::Align::Min), |ui| {
                ui.set_min_width(avail);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("PROJETS EN COURS").font(pixel(12.0)).color(MUTED));
                        muted(ui, format!("{}/{} · lancez-les depuis la salle projet", c.g().projects.len(), projects::MAX_PARALLEL));
                    });
                    egui::ScrollArea::horizontal().id_salt("projects_scroll").max_width(avail).show(ui, |ui| ui.horizontal(|ui| {
                        let list = c.g().projects.clone();
                        if list.is_empty() {
                            card().show(ui, |ui| {
                                ui.set_width(avail - 20.0);
                                muted(ui, "Aucun projet. Cliquez sur la SALLE PROJET du siège pour en lancer un (1 créneau d'agenda).");
                            });
                        }
                        let n = projects::MAX_PARALLEL as f32;
                        let w = ((avail - 8.0 * n) / n).max(170.0);
                        for p in list {
                            let Some(def) = c.content.project(&p.id).cloned() else { continue };
                            card().show(ui, |ui| ui.vertical(|ui| {
                                ui.set_width(w - 18.0);
                                ui.spacing_mut().item_spacing.y = 3.0;
                                ui.add(egui::Label::new(RichText::new(&def.name).font(bold(13.0)).color(TEXT)).truncate()).on_hover_text(&def.desc);
                                let frac = p.progress / def.duration as f32;
                                gauge(ui, frac, if p.slips > 1 { ORANGE } else { CYAN }, w - 24.0, 8.0)
                                    .on_hover_text(format!("{:.0} % — {} glissement(s)", frac * 100.0, p.slips));
                                ui.horizontal(|ui| {
                                    let left = ((def.duration as f32 - p.progress) / projects::speed(&def, &p, c.g()).max(0.1)).ceil().max(1.0);
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.spacing_mut().item_spacing.x = 4.0;
                                        if ui.add(egui::Button::new(RichText::new("×").size(12.0)).small()).on_hover_text("Abandonner le projet (le COMEX et les équipes n'aimeront pas).").clicked() {
                                            c.run(Command::CancelProject(p.id.clone()));
                                        }
                                        if !p.esn && ui.add(egui::Button::new(RichText::new("ESN").size(11.5)).small()).on_hover_text("Renfort ESN : plus rapide (+35 %), coût ×1,6, −3 budget, dépendance en hausse.").clicked() {
                                            c.run(Command::EsnBoost(p.id.clone()));
                                        }
                                        let txt = format!("≈ {left:.0} mois · {:.1}/m{}", def.burn * if p.esn { 1.6 } else { 1.0 }, if p.esn { " · ESN" } else { "" });
                                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                            ui.add(egui::Label::new(RichText::new(txt).size(12.0).color(MUTED)).truncate())
                                                .on_hover_text("Durée restante estimée · coût mensuel du projet (points de budget)");
                                        });
                                    });
                                });
                            }));
                        }
                    }));
                });
            });
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 3.0;
                let g = c.g();
                let b = dsi_core::economy::breakdown(c.content, g);
                let tip = budget_tooltip(&b);
                let r = ui
                    .vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("BUDGET MENSUEL").font(pixel(12.0)).color(MUTED));
                            ui.label(RichText::new(format!("{:.1}", b.allocation)).font(bold(14.0)).color(YELLOW));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(RichText::new(format!("solde {:+.1}", b.balance)).font(bold(13.0)).color(if b.balance >= 0.0 { GREEN } else { RED }));
                            });
                        });
                        budget_bar(ui, &b, right_w, 12.0);
                        let items = budget_items(&b);
                        ui.horizontal(|ui| {
                            for (k, col, v) in &items[..3] {
                                budget_chip(ui, k, *col, *v);
                            }
                        });
                        ui.horizontal(|ui| {
                            for (k, col, v) in &items[3..] {
                                budget_chip(ui, k, *col, *v);
                            }
                            let run = dsi_core::score::run_cost(g);
                            let diff = if g.baseline_run > 0.0 { (run / g.baseline_run - 1.0) * 100.0 } else { 0.0 };
                            ui.label(RichText::new(format!("· fonct. {diff:+.0} %")).size(12.0).color(if diff <= -15.0 { GREEN } else { MUTED }))
                                .on_hover_text("Coût de fonctionnement (contrats + recrues + cloud + IA) par rapport à janvier. Objectif de l'année 1 : −15 %.");
                        });
                    })
                    .response
                    .rect;
                ui.interact(r, egui::Id::new("budget_tip"), egui::Sense::hover()).on_hover_text(tip);
                mark(&mut c.ui.marks, "budget", r);
            });
        });
    });
}

/// Enseigne, étiquette de survol et animation de changement de mois, par-dessus le monde.
pub fn world_overlay(
    ctx: &egui::Context,
    c: &mut Ctx,
    central: egui::Rect,
    vp: &WorldViewport,
    hover: Option<Room>,
    person: Option<&(String, bevy::math::Vec2)>,
    stats: &crate::plateau::PlateauStats,
) {
    view_switch(ctx, c, central, stats);
    if c.view.plateau {
        plateau_overlay(ctx, c, central, person);
        month_flash(ctx, c, central);
        return;
    }
    let painter = ctx
        .layer_painter(egui::LayerId::new(
            egui::Order::Middle,
            egui::Id::new("world_overlay"),
        ))
        .with_clip_rect(central);
    let g = c.g();
    let sign = world::world_to_screen(vp, world::sign_world());
    let size = (vp.zoom * 4.6).clamp(9.0, 26.0);
    let company = g.profile.company.to_uppercase();
    painter.text(
        egui::pos2(sign.x, sign.y),
        egui::Align2::CENTER_CENTER,
        &company,
        pixel(size),
        Color32::from_rgb(255, 170, 70),
    );
    // Étiquettes des salles qui ont des sujets en attente.
    for room in Room::ALL {
        let n = g
            .inbox
            .iter()
            .filter(|i| dsi_core::game::inbox_room(c.content, i) == room)
            .count();
        if n > 0 {
            let r = world::room_world_rect(room);
            let p = world::world_to_screen(vp, bevy::math::Vec2::new(r.center().x, r.min.y));
            let txt = if n > 1 {
                format!("{} ({n})", room.label())
            } else {
                room.label().to_string()
            };
            let font = egui::FontId::proportional((vp.zoom * 3.6).clamp(10.0, 15.0));
            let galley = painter.layout_no_wrap(txt, font, YELLOW);
            let rect = egui::Rect::from_center_size(
                egui::pos2(p.x, p.y - galley.size().y / 2.0 - 2.0),
                galley.size() + egui::vec2(8.0, 2.0),
            );
            painter.rect_filled(rect, 2.0, Color32::from_black_alpha(190));
            painter.galley(rect.min + egui::vec2(4.0, 1.0), galley, YELLOW);
        }
    }
    if let Some(room) = hover {
        let r = world::room_world_rect(room);
        let p = world::world_to_screen(vp, bevy::math::Vec2::new(r.center().x, r.max.y));
        let pending = g
            .inbox
            .iter()
            .filter(|i| dsi_core::game::inbox_room(c.content, i) == room)
            .count();
        let actions = c.content.actions.iter().filter(|a| a.room == room).count()
            + match room {
                Room::Projet | Room::Rh | Room::Achats => 1,
                _ => 0,
            };
        let sub = if pending > 0 {
            format!("{pending} sujet(s) — cliquer pour traiter")
        } else if actions > 0 {
            format!("{actions} action(s) possibles")
        } else {
            "Rien à faire ici".into()
        };
        egui::Area::new(egui::Id::new("room_tip"))
            .fixed_pos(egui::pos2(p.x - 90.0, p.y - 46.0))
            .order(egui::Order::Tooltip)
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(Color32::from_black_alpha(220))
                    .corner_radius(3)
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .stroke(egui::Stroke::new(1.0, YELLOW))
                    .show(ui, |ui| {
                        ui.set_width(180.0);
                        ui.label(RichText::new(room.label()).font(pixel(14.0)).color(YELLOW));
                        ui.label(RichText::new(sub).size(12.5).color(LIGHT));
                    });
            });
    }
    month_flash(ctx, c, central);
}

fn month_flash(ctx: &egui::Context, c: &Ctx, central: egui::Rect) {
    let painter = ctx
        .layer_painter(egui::LayerId::new(
            egui::Order::Middle,
            egui::Id::new("month_flash"),
        ))
        .with_clip_rect(central);
    let g = c.g();
    let f = c.session.month_flash;
    if f > 0.0 && !c.settings.reduce_motion {
        let a = (f.min(1.0) * 255.0) as u8;
        let text = if g.cal.month == 1 {
            format!("ANNÉE {}", g.cal.year)
        } else {
            g.cal.month_name().to_uppercase()
        };
        let pos = central.center() - egui::vec2(0.0, central.height() * 0.28);
        painter.text(
            pos + egui::vec2(3.0, 3.0),
            egui::Align2::CENTER_CENTER,
            &text,
            pixel(54.0),
            Color32::from_black_alpha(a / 2 + 40),
        );
        painter.text(
            pos,
            egui::Align2::CENTER_CENTER,
            &text,
            pixel(54.0),
            Color32::from_rgba_unmultiplied(250, 214, 92, a),
        );
    }
}

pub fn shortcuts(ctx: &egui::Context, c: &mut Ctx) {
    if ctx.egui_wants_keyboard_input() {
        return;
    }
    let (space, esc, keys) = ctx.input(|i| {
        (
            i.key_pressed(egui::Key::Space),
            i.key_pressed(egui::Key::Escape),
            [
                egui::Key::A,
                egui::Key::I,
                egui::Key::C,
                egui::Key::T,
                egui::Key::E,
            ]
            .map(|k| i.key_pressed(k)),
        )
    });
    if !ctx.egui_wants_keyboard_input() && ctx.input(|i| i.key_pressed(egui::Key::V)) {
        c.view.plateau = !c.view.plateau;
    }
    let free = c
        .session
        .game
        .as_ref()
        .is_some_and(|g| g.active.is_none() && !g.finished())
        && c.ui.room_panel.is_none()
        && !c.ui.show_mentors
        && !c.ui.show_settings;
    if esc {
        if c.ui.show_mentors {
            c.ui.show_mentors = false;
        } else if c.ui.room_panel.is_some() {
            c.ui.room_panel = None;
        } else if c.ui.show_help || c.ui.show_credits {
            c.ui.show_help = false;
            c.ui.show_credits = false;
        } else {
            c.ui.show_settings = !c.ui.show_settings;
        }
    }
    if !free {
        return;
    }
    if !ctx.egui_wants_keyboard_input() && ctx.input(|i| i.key_pressed(egui::Key::M)) {
        c.ui.show_mentors = true;
    }
    if space {
        if c.g().inbox.is_empty() {
            c.run(Command::AdvanceMonth);
        } else {
            c.run(Command::Open(0));
        }
    }
    let tabs = [
        Tab::Inbox,
        Tab::Metrics,
        Tab::Contracts,
        Tab::Tech,
        Tab::Team,
    ];
    for (k, t) in keys.iter().zip(tabs) {
        if *k {
            c.ui.tab = t;
        }
    }
}

/// Bascule Siège / Plateau DSI, en haut à gauche de la zone du monde.
fn view_switch(
    ctx: &egui::Context,
    c: &mut Ctx,
    central: egui::Rect,
    stats: &crate::plateau::PlateauStats,
) {
    let vs = egui::Area::new(egui::Id::new("view_switch"))
        .fixed_pos(central.min + egui::vec2(10.0, 10.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(Color32::from_black_alpha(190))
                .corner_radius(3)
                .inner_margin(egui::Margin::same(4))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .add(egui::Button::selectable(
                                !c.view.plateau,
                                RichText::new("🏢 Siège").size(14.0),
                            ))
                            .on_hover_text("Le siège en coupe : cliquez les salles (touche V)")
                            .clicked()
                        {
                            c.view.plateau = false;
                            c.click();
                        }
                        let label = format!("💻 Plateau DSI · {} interne(s)", stats.internes);
                        if ui
                            .add(egui::Button::selectable(
                                c.view.plateau,
                                RichText::new(label).size(14.0),
                            ))
                            .on_hover_text(
                                "L'open space isométrique : votre équipe au travail (touche V)",
                            )
                            .clicked()
                        {
                            c.view.plateau = true;
                            c.click();
                        }
                    });
                });
        });
    mark(&mut c.ui.marks, "view_switch", vs.response.rect);
    if c.view.plateau {
        egui::Area::new(egui::Id::new("plateau_legend"))
            .pivot(egui::Align2::RIGHT_TOP)
            .fixed_pos(egui::pos2(central.max.x - 10.0, central.min.y + 10.0))
            .order(egui::Order::Foreground)
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(Color32::from_black_alpha(170))
                    .corner_radius(3)
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .show(ui, |ui| {
                        ui.label(RichText::new("PLATEAU DSI").font(pixel(14.0)).color(YELLOW));
                        ui.label(
                            RichText::new(format!(
                                "{} recrue(s) interne(s) · {} prestataire(s) ESN · {} agent(s) IA",
                                stats.internes, stats.esn, stats.agents
                            ))
                            .size(12.5)
                            .color(LIGHT),
                        );
                    });
            });
    }
}

fn plateau_overlay(
    ctx: &egui::Context,
    c: &Ctx,
    central: egui::Rect,
    person: Option<&(String, bevy::math::Vec2)>,
) {
    if c.g().team.is_empty() {
        egui::Area::new(egui::Id::new("plateau_empty")).fixed_pos(egui::pos2(central.min.x + 10.0, central.min.y + 56.0)).order(egui::Order::Foreground).interactable(false).show(ctx, |ui| {
            egui::Frame::new().fill(Color32::from_black_alpha(190)).corner_radius(3).inner_margin(egui::Margin::symmetric(10, 6)).show(ui, |ui| {
                ui.set_width(340.0);
                ui.label(RichText::new("Les bureaux sont vides : le SI est tenu par l'ESN. Recrutez depuis la salle RH du siège pour remplir le plateau.").size(13.0).color(LIGHT));
            });
        });
    }
    if let Some((label, p)) = person {
        egui::Area::new(egui::Id::new("person_tip"))
            .fixed_pos(egui::pos2(p.x - 100.0, p.y - 70.0))
            .order(egui::Order::Tooltip)
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(Color32::from_black_alpha(220))
                    .corner_radius(3)
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .stroke(egui::Stroke::new(1.0, YELLOW))
                    .show(ui, |ui| {
                        ui.set_max_width(220.0);
                        ui.label(RichText::new(label).size(13.0).color(TEXT));
                    });
            });
    }
}

/// Le gros bouton d'action du mois, en bas à droite de la vue du monde : il ouvre le
/// sujet suivant tant qu'il en reste, puis passe au mois suivant.
pub fn next_button(ctx: &egui::Context, c: &mut Ctx, central: egui::Rect) {
    let g = c.g();
    if g.active.is_some() || g.finished() {
        return;
    }
    let pending = g.inbox.len();
    let december = g.cal.month == 12;
    let agenda = g.agenda;
    let next_month = dsi_core::state::Calendar::from_abs(g.turn() + 1).month_name();
    let t = ctx.input(|i| i.time) as f32;
    let pulse = if c.settings.reduce_motion {
        1.0
    } else {
        0.5 + 0.5 * (t * 2.5).sin()
    };
    let (label, sub, fill, glow) = if pending > 0 {
        let first = dsi_core::game::inbox_title(c.content, g, &g.inbox[0]);
        (format!("▶ SUJET SUIVANT ({pending})"), first, ORANGE, false)
    } else if december {
        (
            "COMEX ANNUEL ▶".to_string(),
            "Le bilan de l'année vous attend.".to_string(),
            YELLOW,
            true,
        )
    } else {
        let sub = if agenda > 0 {
            format!("Il vous reste {agenda} créneau(x) — ou passez en {next_month}")
        } else {
            format!("Passer en {next_month} · barre d'espace")
        };
        ("MOIS SUIVANT ▶".to_string(), sub, YELLOW, true)
    };
    let mut clicked = false;
    let area = egui::Area::new(egui::Id::new("next_button"))
        .pivot(egui::Align2::RIGHT_BOTTOM)
        .fixed_pos(egui::pos2(central.max.x - 12.0, central.max.y - 12.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let stroke = if glow {
                egui::Stroke::new(
                    3.0,
                    Color32::from_rgba_unmultiplied(255, 240, 170, (90.0 + 165.0 * pulse) as u8),
                )
            } else {
                egui::Stroke::new(2.0, Color32::from_rgb(255, 200, 120))
            };
            egui::Frame::new()
                .fill(Color32::from_black_alpha(200))
                .corner_radius(6)
                .inner_margin(egui::Margin::same(6))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        let b = ui.add(
                            egui::Button::new(
                                RichText::new(&label).font(pixel(20.0)).color(NEAR_BLACK),
                            )
                            .fill(fill)
                            .stroke(stroke)
                            .corner_radius(5)
                            .min_size(egui::vec2(300.0, 52.0)),
                        );
                        if b.on_hover_cursor(egui::CursorIcon::PointingHand)
                            .on_hover_text("Barre d'espace")
                            .clicked()
                        {
                            clicked = true;
                        }
                        ui.add(
                            egui::Label::new(RichText::new(&sub).size(12.5).color(LIGHT))
                                .truncate(),
                        );
                    });
                })
                .response
        });
    mark(&mut c.ui.marks, "month_btn", area.inner.rect);
    if clicked {
        if pending > 0 {
            c.run(Command::Open(0));
        } else {
            c.run(Command::AdvanceMonth);
        }
    }
}
