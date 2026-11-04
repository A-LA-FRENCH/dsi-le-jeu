//! Les mentors en jeu : le portrait du mentor principal en bas à gauche du monde,
//! et le panneau des trois mentors (conseils, délais, dernier conseil reçu).

use super::Ctx;
use super::style::*;
use super::widgets::*;
use bevy_egui::egui::{self, Color32, RichText};
use dsi_core::{Command, mentors};

pub fn widget(ctx: &egui::Context, c: &mut Ctx, central: egui::Rect) {
    let g = c.g();
    let Some(def) = c.content.mentor(&g.profile.mentor).cloned() else { return };
    let wait = mentors::wait(g, &def);
    let ready = c.content.mentors.iter().any(|m| mentors::wait(g, m) == 0);
    let idx = c.mentor_index(&def.id);
    let accent = hex(&def.color);
    let t = ctx.input(|i| i.time) as f32;
    let pulse = if ready && !c.settings.reduce_motion { 0.5 + 0.5 * (t * 3.0).sin() } else { 1.0 };
    let mut open = false;
    let resp = egui::Area::new(egui::Id::new("mentor_widget"))
        .fixed_pos(egui::pos2(central.min.x + 10.0, central.max.y - 86.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let stroke = if ready { Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), (120.0 + 135.0 * pulse) as u8) } else { LINE };
            egui::Frame::new()
                .fill(Color32::from_black_alpha(210))
                .stroke(egui::Stroke::new(if ready { 2.0 } else { 1.0 }, stroke))
                .corner_radius(4)
                .inner_margin(egui::Margin::same(6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if mentor_portrait(ui, c.mentors_tex, idx, 60.0).on_hover_text("Parler à tes mentors (M)").clicked() {
                            open = true;
                        }
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.label(RichText::new(&def.name).font(pixel(16.0)).color(accent));
                            muted(ui, "ton mentor");
                            let label = if wait == 0 {
                                "💬 Demander un conseil".to_string()
                            } else if ready {
                                "💬 Autre mentor dispo".to_string()
                            } else {
                                format!("Prochain conseil dans {wait} mois")
                            };
                            if ui.add(egui::Button::new(RichText::new(label).size(13.0))).clicked() {
                                open = true;
                            }
                        });
                    });
                })
                .response
        })
        .inner;
    mark(&mut c.ui.marks, "mentor", resp.rect);
    if open {
        c.ui.show_mentors = true;
        c.click();
    }
}

pub fn panel(ctx: &egui::Context, c: &mut Ctx) {
    let mut close = false;
    let mut consult: Option<String> = None;
    let list = c.content.mentors.clone();
    modal(ctx, "mentors", 720.0, YELLOW, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Tes mentors").font(pixel(24.0)).color(YELLOW));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("×").on_hover_text("Fermer (Échap)").clicked() {
                    close = true;
                }
            });
        });
        muted(ui, "Un conseil ne coûte aucun créneau d'agenda. Il donne un petit bonus adapté à ta situation. Ton mentor principal revient tous les 2 mois, les autres tous les 4.");
        ui.add_space(4.0);
        for (i, m) in list.iter().enumerate() {
            let g = c.g();
            let main = g.profile.mentor == m.id;
            let wait = mentors::wait(g, m);
            let accent = hex(&m.color);
            card().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_top(|ui| {
                    mentor_portrait(ui, c.mentors_tex, i as u32, 72.0);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&m.name).font(pixel(20.0)).color(accent));
                            if main {
                                kind_banner(ui, "MENTOR PRINCIPAL", accent);
                            }
                        });
                        ui.add(egui::Label::new(RichText::new(&m.title).size(12.5).color(LIGHT)).wrap());
                        if main {
                            ui.label(RichText::new(format!("Bonus permanent : {}", m.passive_label)).size(12.5).color(GREEN));
                        }
                        if let Some(last) = g.mentors.last_advice.get(&m.id) {
                            ui.add(egui::Label::new(RichText::new(format!("Dernier conseil : « {last} »")).size(12.5).italics().color(MUTED)).wrap());
                        }
                        ui.add_space(2.0);
                        if wait == 0 {
                            if big_button(ui, "Demander un conseil", true, 240.0).clicked() {
                                consult = Some(m.id.clone());
                            }
                        } else {
                            ui.label(RichText::new(format!("En tournage du podcast · disponible dans {wait} mois")).size(13.0).color(ORANGE));
                        }
                    });
                });
            });
            ui.add_space(3.0);
        }
    });
    if let Some(id) = consult {
        c.ui.show_mentors = false;
        c.run(Command::ConsultMentor(id));
    } else if close {
        c.ui.show_mentors = false;
    }
}
