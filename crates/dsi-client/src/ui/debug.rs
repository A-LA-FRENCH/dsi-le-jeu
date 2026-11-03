//! Mode debug (`?debug=true`) : modifier les métriques, déclencher des événements,
//! accélérer le temps, débloquer des technologies, inspecter l'état.

use super::Ctx;
use super::style::*;
use bevy_egui::egui::{self, RichText};
use dsi_core::Command;
use dsi_core::state::Metric;

pub fn window(ctx: &egui::Context, c: &mut Ctx) {
    let mut cmds: Vec<Command> = Vec::new();
    let mut open = c.ui.show_debug;
    egui::Window::new(RichText::new("DEBUG").font(pixel(14.0)).color(RED))
        .open(&mut open)
        .default_pos(egui::pos2(12.0, 120.0))
        .default_width(320.0)
        .resizable(true)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .max_height(560.0)
                .show(ui, |ui| {
                    let g = c.g().clone();
                    ui.label(format!(
                        "graine {} · tour {} · {} · rev {}",
                        g.seed,
                        g.turn(),
                        g.cal.label(),
                        c.session.rev
                    ));
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Mois suivant (forcé)").clicked() {
                            cmds.push(Command::DebugSkip);
                        }
                        if ui.button("×6 mois").clicked() {
                            for _ in 0..6 {
                                cmds.push(Command::DebugSkip);
                            }
                        }
                        if ui.button("Agenda plein").clicked() {
                            cmds.push(Command::DebugAgenda);
                        }
                    });
                    ui.collapsing("Recruter instantanément", |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for r in &c.content.careers.roles {
                                if ui.small_button(&r.label).clicked() {
                                    cmds.push(Command::DebugHire(r.id.clone()));
                                }
                            }
                        });
                    });
                    ui.collapsing("Métriques", |ui| {
                        for m in Metric::ALL {
                            let mut v = g.m(m);
                            if ui
                                .add(egui::Slider::new(&mut v, 0.0..=100.0).text(m.short()))
                                .changed()
                            {
                                cmds.push(Command::DebugSet(m, v));
                            }
                        }
                    });
                    ui.collapsing("Déclencher un événement", |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut c.ui.debug_filter)
                                .hint_text("filtrer…"),
                        );
                        let f = c.ui.debug_filter.to_lowercase();
                        for e in c
                            .content
                            .events
                            .iter()
                            .filter(|e| {
                                f.is_empty()
                                    || e.id.contains(&f)
                                    || e.title.to_lowercase().contains(&f)
                            })
                            .take(40)
                        {
                            if ui.small_button(format!("{} — {}", e.id, e.title)).clicked() {
                                cmds.push(Command::DebugEvent(e.id.clone()));
                            }
                        }
                    });
                    ui.collapsing("Débloquer une technologie", |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for t in &c.content.techs {
                                if !g.has_tech(&t.id) && ui.small_button(&t.name).clicked() {
                                    cmds.push(Command::DebugTech(t.id.clone()));
                                }
                            }
                        });
                    });
                    ui.collapsing("Conséquences programmées", |ui| {
                        for s in &g.scheduled {
                            ui.label(
                                RichText::new(format!(
                                    "tour {} : {} {}",
                                    s.due,
                                    s.event.clone().unwrap_or_default(),
                                    s.text
                                ))
                                .size(12.0),
                            );
                        }
                    });
                    ui.checkbox(&mut c.ui.debug_json, "Inspecter l'état (JSON)");
                    if c.ui.debug_json {
                        let mut json = serde_json::to_string_pretty(&g).unwrap_or_default();
                        egui::ScrollArea::vertical()
                            .max_height(300.0)
                            .id_salt("json")
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut json)
                                        .code_editor()
                                        .desired_width(f32::INFINITY),
                                );
                            });
                    }
                });
        });
    c.ui.show_debug = open;
    for cmd in cmds {
        c.run(cmd);
    }
}
