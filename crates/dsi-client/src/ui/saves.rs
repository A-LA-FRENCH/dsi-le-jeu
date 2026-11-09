//! Sauvegardes manuelles : trois emplacements, en plus de la sauvegarde automatique.

use super::style::*;
use super::widgets::*;
use super::{Ctx, Screen};
use crate::sim::Session;
use bevy_egui::egui::{self, RichText};
use dsi_core::save::{SLOT_COUNT, SlotInfo};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pending {
    Overwrite(usize),
    Load(usize),
    Delete(usize),
}

/// Ouvre la fenêtre des sauvegardes (relit les emplacements).
pub fn open(c: &mut Ctx) {
    c.ui.slots = Session::slot_infos(c.store);
    c.ui.slot_pending = None;
    c.ui.show_saves = true;
}

fn ago(saved_at: u64) -> String {
    if saved_at == 0 {
        return String::new();
    }
    let s = platform::clock::now_millis().saturating_sub(saved_at) / 1000;
    match s {
        0..60 => "à l'instant".into(),
        60..3600 => format!("il y a {} min", s / 60),
        3600..86_400 => format!("il y a {} h", s / 3600),
        _ => format!("il y a {} j", s / 86_400),
    }
}

fn describe(ui: &mut egui::Ui, info: &SlotInfo) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(&info.name).font(bold(15.0)).color(TEXT));
        ui.label(
            RichText::new(format!("· {}", info.company))
                .size(13.0)
                .color(MUTED),
        );
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(&info.date).font(bold(13.5)).color(YELLOW));
        ui.label(
            RichText::new(format!("· score {} ({})", info.score, info.rank))
                .size(13.0)
                .color(LIGHT),
        );
        ui.label(
            RichText::new(format!("· {}", info.status))
                .size(13.0)
                .color(CYAN),
        );
    });
    let when = ago(info.saved_at);
    if !when.is_empty() {
        ui.label(
            RichText::new(format!("Sauvegardée {when}"))
                .size(12.0)
                .color(MUTED),
        );
    }
}

pub fn show(ctx: &egui::Context, c: &mut Ctx) {
    if !c.ui.show_saves {
        return;
    }
    let in_game = c.session.game.is_some() && c.ui.screen == Screen::Playing;
    let mut action: Option<Pending> = None;
    let mut close = false;
    modal(ctx, "saves", 560.0, CYAN, |ui| {
        kind_banner(ui, "SAUVEGARDES", CYAN);
        ui.label(
            RichText::new(if in_game {
                "Sauvegarder ou charger une partie"
            } else {
                "Charger une partie"
            })
            .font(pixel(20.0))
            .color(YELLOW),
        );
        muted(
            ui,
            "Trois emplacements, en plus de la sauvegarde automatique (« Continuer la partie »). Les sauvegardes restent dans ce navigateur.",
        );
        ui.add_space(6.0);
        for i in 0..SLOT_COUNT {
            let slot = c.ui.slots.get(i).cloned().flatten();
            let pending = c.ui.slot_pending;
            card().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("EMPLACEMENT {}", i + 1)).font(bold(12.5)).color(MUTED));
                });
                match &slot {
                    None => muted(ui, "Vide"),
                    Some(Err(e)) => {
                        ui.label(RichText::new(format!("Sauvegarde illisible : {e}")).size(12.5).color(RED));
                    }
                    Some(Ok(info)) => describe(ui, info),
                }
                ui.add_space(2.0);
                let confirm = |ui: &mut egui::Ui, msg: &str, yes: &str, p: Pending, action: &mut Option<Pending>, cancel: &mut bool| {
                    ui.label(RichText::new(msg).size(13.0).color(ORANGE));
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new(yes).color(ORANGE)).clicked() {
                            *action = Some(p);
                        }
                        if ui.button("Annuler").clicked() {
                            *cancel = true;
                        }
                    });
                };
                let mut cancel = false;
                match pending {
                    Some(p @ Pending::Overwrite(j)) if j == i => {
                        confirm(ui, "Écraser la partie de cet emplacement ?", "Oui, écraser", p, &mut action, &mut cancel)
                    }
                    Some(p @ Pending::Load(j)) if j == i => confirm(
                        ui,
                        "Charger cette partie ? La partie en cours sera remplacée (sauvegardez-la d'abord si besoin).",
                        "Oui, charger",
                        p,
                        &mut action,
                        &mut cancel,
                    ),
                    Some(p @ Pending::Delete(j)) if j == i => {
                        confirm(ui, "Supprimer définitivement cette sauvegarde ?", "Oui, supprimer", p, &mut action, &mut cancel)
                    }
                    _ => {
                        ui.horizontal(|ui| {
                            if in_game && ui.button(RichText::new("Sauvegarder ici").strong()).clicked() {
                                if slot.is_some() {
                                    c.ui.slot_pending = Some(Pending::Overwrite(i));
                                } else {
                                    action = Some(Pending::Overwrite(i));
                                }
                            }
                            if matches!(slot, Some(Ok(_))) && ui.button(RichText::new("Charger").strong()).clicked() {
                                if in_game {
                                    c.ui.slot_pending = Some(Pending::Load(i));
                                } else {
                                    action = Some(Pending::Load(i));
                                }
                            }
                            if slot.is_some() && ui.button(RichText::new("Supprimer").color(MUTED)).clicked() {
                                c.ui.slot_pending = Some(Pending::Delete(i));
                            }
                        });
                    }
                }
                if cancel {
                    c.ui.slot_pending = None;
                }
            });
            ui.add_space(3.0);
        }
        ui.add_space(6.0);
        if ghost_button(ui, "Fermer", 120.0).clicked() {
            close = true;
        }
    });
    if let Some(p) = action {
        c.ui.slot_pending = None;
        match p {
            Pending::Overwrite(i) => match c.session.save_slot(c.store, i) {
                Ok(()) => c.ui.toast(
                    format!("Partie sauvegardée dans l'emplacement {}.", i + 1),
                    true,
                ),
                Err(e) => c.ui.toast(format!("Sauvegarde impossible : {e}"), false),
            },
            Pending::Load(i) => match c.session.load_slot(c.store, i) {
                Ok(()) => {
                    c.ui.reset_for_load();
                    c.ui.screen = Screen::Playing;
                    c.ui.toast(format!("Partie de l'emplacement {} chargée.", i + 1), true);
                    close = true;
                }
                Err(e) => c.ui.toast(format!("Chargement impossible : {e}"), false),
            },
            Pending::Delete(i) => match Session::delete_slot(c.store, i) {
                Ok(()) => c.ui.toast(format!("Emplacement {} vidé.", i + 1), true),
                Err(e) => c.ui.toast(format!("Suppression impossible : {e}"), false),
            },
        }
        c.ui.slots = Session::slot_infos(c.store);
    }
    if close || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        c.ui.show_saves = false;
        c.ui.slot_pending = None;
    }
}
