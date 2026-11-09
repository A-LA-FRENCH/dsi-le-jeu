//! Saison 2 : objectifs de fin explicites et choix du cap (la fin visée).

use super::Ctx;
use super::style::*;
use super::widgets::*;
use bevy_egui::egui::{self, RichText};
use dsi_core::Command;
use dsi_core::score::{self, EndingGoal};

/// Liste des conditions d'une fin, en pastilles ✔ / ×.
pub fn checks(ui: &mut egui::Ui, g: &EndingGoal) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(10.0, 2.0);
        for k in &g.checks {
            ui.label(
                RichText::new(format!(
                    "{} {} ({})",
                    if k.ok { "✔" } else { "×" },
                    k.label,
                    k.current
                ))
                .size(12.5)
                .color(if k.ok { GREEN } else { LIGHT }),
            );
        }
    });
}

/// Titre + progression d'une fin.
pub fn header(ui: &mut egui::Ui, g: &EndingGoal, chosen: bool) {
    let (k, n) = g.progress();
    ui.horizontal(|ui| {
        ui.label(RichText::new(g.title).font(bold(15.0)).color(if g.met() {
            GREEN
        } else if chosen {
            YELLOW
        } else {
            TEXT
        }));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(format!("{k}/{n}"))
                    .font(bold(13.0))
                    .color(if g.met() { GREEN } else { MUTED }),
            );
            gauge(
                ui,
                k as f32 / n as f32,
                if g.met() { GREEN } else { CYAN },
                80.0,
                8.0,
            );
            if g.met() {
                ui.label(RichText::new("ATTEINTE").font(pixel(10.0)).color(GREEN));
            }
        });
    });
}

/// Fenêtre de choix du cap : au lancement de la saison 2, ou pour en changer.
pub fn picker(ctx: &egui::Context, c: &mut Ctx) {
    let Some(g) = c.session.game.as_ref() else {
        return;
    };
    let starting = g.extension == 0;
    let current = g.ambition.clone();
    let goals = score::ending_goals(g);
    let years = c.content.dialogues.extension_years;
    let last_year = score::final_year(c.content, g) + if starting { years } else { 0 };
    let mut close = false;
    let mut confirm = false;
    modal(ctx, "ambition", 720.0, ORANGE, |ui| {
        kind_banner(ui, "SAISON 2 — CAP DE FIN", ORANGE);
        ui.label(
            RichText::new("Quelle DSI voulez-vous être au terme de l'année 6 ?")
                .font(pixel(20.0))
                .color(YELLOW),
        );
        wrap(
            ui,
            RichText::new(format!(
                "Choisissez la fin que vous visez. Elle sera suivie en bas de l'écran, dans l'onglet Indicateurs et à chaque COMEX annuel. Si toutes ses conditions sont remplies fin décembre de l'année {last_year}, c'est elle qui est retenue (même si une autre est aussi atteinte) et le score gagne +{} points. Sans cap, la fin la plus remarquable atteinte est retenue. Changer de cap en cours de route coûte 1 créneau d'agenda et 2 points de COMEX.",
                score::AMBITION_BONUS
            ))
            .size(13.5)
            .color(LIGHT),
        );
        ui.add_space(6.0);
        for goal in &goals {
            let chosen = c.ui.ambition_choice.as_deref() == Some(goal.id);
            let frame = card().stroke(egui::Stroke::new(
                if chosen { 2.0 } else { 1.0 },
                if chosen { YELLOW } else { LINE },
            ));
            let r = frame
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    header(ui, goal, chosen);
                    muted(ui, goal.pitch);
                    checks(ui, goal);
                    ui.label(
                        RichText::new(format!("Leviers : {}", goal.levers))
                            .size(12.0)
                            .italics()
                            .color(MUTED),
                    );
                })
                .response;
            if ui
                .interact(
                    r.rect,
                    egui::Id::new(("goal", goal.id)),
                    egui::Sense::click(),
                )
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                c.ui.ambition_choice = Some(goal.id.to_string());
            }
            ui.add_space(3.0);
        }
        ui.horizontal(|ui| {
            let none = c.ui.ambition_choice.is_none();
            if ui.selectable_label(none, "Pas de cap : je verrai bien (la fin la plus remarquable atteinte sera retenue)").clicked() {
                c.ui.ambition_choice = None;
            }
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let changing = !starting && current.is_some() && c.ui.ambition_choice != current;
            let label = if starting {
                format!("Commencer la saison 2 ({years} ans) ▶")
            } else if changing {
                "Changer de cap (1 créneau, COMEX −2)".to_string()
            } else {
                "Valider".to_string()
            };
            if big_button(ui, &label, true, 340.0).clicked() {
                confirm = true;
            }
            if ghost_button(ui, "Annuler", 120.0).clicked() {
                close = true;
            }
        });
    });
    if confirm {
        let choice = c.ui.ambition_choice.clone();
        let ok = if starting {
            c.run(Command::ExtendCampaign)
        } else {
            true
        };
        if ok && (choice.is_some() || current.is_some()) && c.g().ambition != choice {
            c.run(Command::SetAmbition(choice));
        }
        close = true;
    }
    if close || (ctx.input(|i| i.key_pressed(egui::Key::Escape))) {
        c.ui.show_ambition = false;
    }
}

/// Ouvre la fenêtre de choix, présélectionnée sur le cap actuel.
pub fn open(c: &mut Ctx) {
    c.ui.ambition_choice = c.g().ambition.clone();
    c.ui.show_ambition = true;
}

/// Section « Objectifs de fin » de l'onglet Indicateurs.
pub fn section_goals(ui: &mut egui::Ui, c: &mut Ctx) {
    let g = c.g();
    let season2 = g.extension > 0;
    let goals = score::ending_goals(g);
    let ambition = g.ambition.clone();
    section(ui, "OBJECTIFS DE FIN");
    if season2 {
        let t = ambition
            .as_deref()
            .and_then(|a| goals.iter().find(|x| x.id == a))
            .map_or("Aucun cap choisi".to_string(), |x| {
                format!("Cap : {}", x.title)
            });
        ui.horizontal(|ui| {
            ui.label(RichText::new(t).font(bold(14.0)).color(YELLOW));
            if ui
                .small_button(if ambition.is_some() {
                    "Changer de cap"
                } else {
                    "Choisir un cap"
                })
                .clicked()
            {
                open(c);
            }
        });
    } else {
        muted(
            ui,
            "Fin d'année 3 : la première fin atteinte dans cet ordre est retenue. En saison 2, vous pourrez choisir celle que vous visez.",
        );
    }
    for goal in &goals {
        let chosen = ambition.as_deref() == Some(goal.id);
        card()
            .stroke(egui::Stroke::new(
                if chosen { 2.0 } else { 1.0 },
                if chosen { YELLOW } else { LINE },
            ))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                header(ui, goal, chosen);
                egui::CollapsingHeader::new(
                    RichText::new("Conditions et leviers")
                        .size(12.0)
                        .color(MUTED),
                )
                .id_salt(("goal_detail", goal.id))
                .default_open(chosen)
                .show(ui, |ui| {
                    checks(ui, goal);
                    ui.label(
                        RichText::new(format!("Leviers : {}", goal.levers))
                            .size(12.0)
                            .italics()
                            .color(MUTED),
                    );
                });
            });
    }
    muted(ui, "Si aucune n'est atteinte : « Toujours en poste ».");
}
