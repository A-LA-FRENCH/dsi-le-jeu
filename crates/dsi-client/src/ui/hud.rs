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
        .filter(|ch| {
            dsi_core::state::Calendar {
                year: ch.at.year,
                month: ch.at.month,
            }
            .abs()
                <= now
        })
        .last()
        .map(|ch| ch.title.clone())
        .unwrap_or_default()
}

pub fn top_bar(root: &mut egui::Ui, c: &mut Ctx) {
    egui::Panel::top("hud").exact_size(58.0).frame(egui::Frame::new().fill(NEAR_BLACK).inner_margin(egui::Margin::symmetric(12, 6))).show(root, |ui| {
        ui.horizontal_centered(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(RichText::new("DSI — LE JEU").font(pixel(17.0)).color(YELLOW));
                let g = c.g();
                ui.label(RichText::new(g.cal.label()).font(bold(13.0)).color(TEXT)).on_hover_text(chapter_title(c));
            });
            ui.add_space(10.0);
            let width = ui.available_width();
            let gauge_w = ((width - 230.0) / 6.0 - 14.0).clamp(54.0, 110.0);
            for m in Metric::PRIMARY {
                let v = c.g().m(m);
                let col = metric_color(m, v);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.label(RichText::new(m.short()).font(pixel(11.0)).color(MUTED));
                        ui.label(RichText::new(format!("{:.0}", v)).font(bold(14.0)).color(col));
                    });
                    let help = match m {
                        Metric::Budget => "Budget IT disponible. À zéro : Bankruptcy as a Service.",
                        Metric::Security => "Sécurité. À zéro : Cyber Apocalypse.",
                        Metric::Debt => "Dette technique. À 100 : Legacy Forever.",
                        Metric::Comex => "Confiance du COMEX. À zéro : vous êtes viré.",
                        Metric::DevSat => "Satisfaction et énergie des équipes d'ingénierie.",
                        _ => "Disponibilité du SI.",
                    };
                    gauge(ui, v / 100.0, col, gauge_w, 9.0).on_hover_text(format!("{} : {:.1}\n{help}", m.label(), v));
                });
                ui.add_space(6.0);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(RichText::new("⚙").size(16.0)).on_hover_text("Réglages (Échap)").clicked() {
                    c.ui.show_settings = true;
                }
                if ui.button(RichText::new("?").size(16.0)).on_hover_text("Aide").clicked() {
                    c.ui.show_help = true;
                }
                ui.add_space(6.0);
                let g = c.g();
                let mut slots = String::new();
                for i in 0..g.agenda_max {
                    slots.push(if i < g.agenda { '●' } else { '○' });
                }
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.label(RichText::new("AGENDA").font(pixel(11.0)).color(MUTED));
                    ui.label(RichText::new(slots).size(16.0).color(if g.agenda > 0 { YELLOW } else { MUTED }))
                        .on_hover_text("Créneaux d'agenda de ce mois : votre temps. Chaque action dans une salle en consomme un.");
                });
            });
        });
    });
}

pub fn bottom_bar(root: &mut egui::Ui, c: &mut Ctx) {
    egui::Panel::bottom("projects").exact_size(112.0).frame(egui::Frame::new().fill(NEAR_BLACK).inner_margin(egui::Margin::symmetric(12, 8))).show(root, |ui| {
        ui.horizontal(|ui| {
            let pending = c.g().inbox.len();
            let right_w = 230.0;
            let avail = ui.available_width() - right_w - 12.0;
            ui.allocate_ui_with_layout(egui::vec2(avail, 96.0), egui::Layout::left_to_right(egui::Align::Min), |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("PROJETS EN COURS").font(pixel(12.0)).color(MUTED));
                        muted(ui, format!("{}/{} · lancez-les depuis la salle projet", c.g().projects.len(), projects::MAX_PARALLEL));
                    });
                    ui.horizontal(|ui| {
                        let list = c.g().projects.clone();
                        if list.is_empty() {
                            card().show(ui, |ui| {
                                ui.set_width(avail - 20.0);
                                muted(ui, "Aucun projet. Cliquez sur la SALLE PROJET du siège pour en lancer un (1 créneau d'agenda).");
                            });
                        }
                        let w = ((avail - 24.0) / 3.0).max(150.0);
                        for p in list {
                            let Some(def) = c.content.project(&p.id).cloned() else { continue };
                            card().show(ui, |ui| {
                                ui.set_width(w - 18.0);
                                ui.spacing_mut().item_spacing.y = 3.0;
                                ui.add(egui::Label::new(RichText::new(&def.name).font(bold(13.0)).color(TEXT)).truncate()).on_hover_text(&def.desc);
                                let frac = p.progress / def.duration as f32;
                                gauge(ui, frac, if p.slips > 1 { ORANGE } else { CYAN }, w - 24.0, 8.0)
                                    .on_hover_text(format!("{:.0} % — {} glissement(s)", frac * 100.0, p.slips));
                                ui.horizontal(|ui| {
                                    let left = ((def.duration as f32 - p.progress) / projects::speed(&def, &p, c.g()).max(0.1)).ceil().max(1.0);
                                    muted(ui, format!("≈ {left:.0} mois · {:.1} pt/mois", def.burn * if p.esn { 1.6 } else { 1.0 }));
                                    if !p.esn && ui.small_button("⚡ ESN").on_hover_text("Renfort ESN : plus rapide (+35 %), coût ×1,6, −3 budget, dépendance en hausse.").clicked() {
                                        c.run(Command::EsnBoost(p.id.clone()));
                                    }
                                    if ui.small_button("×").on_hover_text("Abandonner le projet (le COMEX et les équipes n'aimeront pas).").clicked() {
                                        c.run(Command::CancelProject(p.id.clone()));
                                    }
                                });
                            });
                        }
                    });
                });
            });
            ui.with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
                let g = c.g();
                let bal = dsi_core::economy::monthly_balance(c.content, g);
                ui.label(RichText::new(format!("Solde mensuel prévu : {:+.1}", bal)).font(bold(13.0)).color(if bal >= 0.0 { GREEN } else { RED }))
                    .on_hover_text("Allocation mensuelle − (contrats + équipe + cloud + IA + projets). Les décisions et événements s'y ajoutent.");
                let enabled = pending == 0 && g.active.is_none() && !g.finished();
                let agenda = g.agenda;
                let label = if pending > 0 { format!("{pending} sujet(s) à traiter") } else if g.cal.month == 12 { "COMEX ANNUEL ▶".into() } else { "MOIS SUIVANT ▶".into() };
                let b = big_button(ui, &label, enabled, right_w);
                if b.on_hover_text("Barre d'espace").clicked() {
                    c.run(Command::AdvanceMonth);
                }
                if pending > 0 {
                    muted(ui, "Cliquez sur les bulles du siège ou la liste « À traiter ».");
                } else if agenda > 0 {
                    muted(ui, format!("Il vous reste {agenda} créneau(x) : visitez une salle."));
                }
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
        && !c.ui.show_settings;
    if esc {
        if c.ui.room_panel.is_some() {
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
    if space && c.g().inbox.is_empty() {
        c.run(Command::AdvanceMonth);
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
    egui::Area::new(egui::Id::new("view_switch"))
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
    if c.view.plateau {
        egui::Area::new(egui::Id::new("plateau_legend"))
            .fixed_pos(egui::pos2(central.min.x + 10.0, central.max.y - 58.0))
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
        egui::Area::new(egui::Id::new("plateau_empty")).fixed_pos(egui::pos2(central.center().x - 170.0, central.min.y + 52.0)).order(egui::Order::Foreground).interactable(false).show(ctx, |ui| {
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
