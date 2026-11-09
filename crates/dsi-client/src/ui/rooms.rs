//! Panneau d'une salle du siège : actions (créneaux d'agenda), catalogue de projets,
//! recrutement, négociations anticipées.

use super::Ctx;
use super::style::*;
use super::widgets::*;
use bevy_egui::egui::{self, RichText};
use dsi_core::state::Room;
use dsi_core::{Command, projects};

fn blurb(room: Room) -> &'static str {
    match room {
        Room::Comex => "Là où se décident votre budget, votre avenir et la couleur des slides.",
        Room::Dsi => "Votre bureau. Une plante, un écran, 412 mails non lus.",
        Room::Finance => "Le DAF vous attend. Il a une question : « Pourquoi payons-nous ça ? »",
        Room::Rh => "La DRH, le canapé violet et la grille salariale de 2015.",
        Room::Achats => "Isabelle connaît tous les commerciaux. Ils la craignent.",
        Room::Archi => "Le tableau blanc où les boîtes et les flèches ont encore du sens.",
        Room::Devs => "L'open space des développeurs. Un canard en plastique veille.",
        Room::Projet => "Kanban, post-its, et le Gantt que personne ne met à jour.",
        Room::Data => "Des tableaux de bord, un lac de données, quelques marécages.",
        Room::Cloud => "Des écrans partout, et une facture qui grimpe en temps réel.",
        Room::Vendor => "Café, viennoiseries et partenariats stratégiques.",
        Room::Crise => "On espère ne jamais y passer la nuit. On y passe la nuit.",
        Room::Cyber => "La RSSI et son registre des risques, toujours à jour.",
        Room::Soc => "Yanis surveille 3 412 alertes. Il a besoin de sommeil.",
        Room::Datacenter => "Racks, clim de 2004, onduleur au bip suspect. Et GERARD.",
    }
}

pub fn show(ctx: &egui::Context, c: &mut Ctx) {
    let Some(room) = c.ui.room_panel else { return };
    let mut close = false;
    let mut cmd: Option<Command> = None;
    modal(ctx, "room", 680.0, YELLOW, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(room.label()).font(pixel(24.0)).color(YELLOW));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("×").on_hover_text("Fermer (Échap)").clicked() {
                    close = true;
                }
                let g = c.g();
                ui.label(
                    RichText::new(format!("Agenda : {}/{}", g.agenda, g.agenda_max))
                        .font(bold(13.5))
                        .color(if g.agenda > 0 { YELLOW } else { RED }),
                );
            });
        });
        muted(ui, blurb(room));
        ui.add_space(6.0);
        match room {
            Room::Projet => project_catalog(ui, c, &mut cmd),
            Room::Rh => recruit(ui, c, &mut cmd),
            Room::Achats => negotiations(ui, c, &mut cmd),
            _ => {}
        }
        let actions: Vec<_> = c
            .content
            .actions
            .iter()
            .filter(|a| a.room == room)
            .cloned()
            .collect();
        if !actions.is_empty() {
            section(ui, "ACTIONS");
        }
        for a in actions {
            let g = c.g();
            let cooldown_left = g
                .actions_done
                .get(&a.id)
                .map(|t| (t + a.cooldown) as i64 - g.turn() as i64)
                .filter(|l| *l > 0 && a.cooldown > 0);
            let req_ok = a.requires.met(g);
            let ok = req_ok && cooldown_left.is_none() && g.agenda >= a.agenda;
            card().show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&a.label).font(bold(14.5)).color(if ok {
                        TEXT
                    } else {
                        MUTED
                    }));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_enabled(
                                ok,
                                egui::Button::new(format!(
                                    "Faire ({} créneau{})",
                                    a.agenda,
                                    if a.agenda > 1 { "x" } else { "" }
                                )),
                            )
                            .clicked()
                        {
                            cmd = Some(Command::DoAction(a.id.clone()));
                        }
                    });
                });
                muted(ui, &a.desc);
                let ch = a.as_choice();
                if let Some(l) = cooldown_left {
                    ui.label(
                        RichText::new(format!("Disponible dans {l} mois"))
                            .size(12.0)
                            .color(ORANGE),
                    );
                } else if !req_ok {
                    ui.label(
                        RichText::new(format!(
                            "🔒 {}",
                            a.requires.explain(g, &|t| c.content.tech_name(t))
                        ))
                        .size(12.0)
                        .color(ORANGE),
                    );
                } else {
                    preview(ui, &ch.preview(), ch.gamble.is_some());
                }
            });
        }
    });
    if close {
        c.ui.room_panel = None;
    }
    if let Some(cmd) = cmd
        && c.run(cmd)
    {
        c.ui.room_panel = None;
    }
}

fn project_catalog(ui: &mut egui::Ui, c: &mut Ctx, cmd: &mut Option<Command>) {
    section(ui, "CATALOGUE DES PROJETS");
    muted(
        ui,
        format!(
            "Lancer un projet coûte 1 créneau d'agenda et son coût de lancement. {} projets en parallèle au maximum.",
            projects::MAX_PARALLEL
        ),
    );
    let mut cats: Vec<String> = c
        .content
        .projects
        .iter()
        .filter(|p| p.catalog)
        .map(|p| p.category.clone())
        .collect();
    cats.dedup();
    let mut seen = std::collections::BTreeSet::new();
    cats.retain(|x| seen.insert(x.clone()));
    ui.horizontal_wrapped(|ui| {
        if ui
            .selectable_label(c.ui.project_filter.is_empty(), "Disponibles")
            .clicked()
        {
            c.ui.project_filter.clear();
        }
        for cat in &cats {
            if ui
                .selectable_label(&c.ui.project_filter == cat, cat)
                .clicked()
            {
                c.ui.project_filter = cat.clone();
            }
        }
    });
    let filter = c.ui.project_filter.clone();
    for base in c.content.projects.iter().filter(|p| p.catalog) {
        let g = c.g();
        if g.done_projects.contains(&base.id) || g.projects.iter().any(|x| x.id == base.id) {
            continue;
        }
        if !filter.is_empty() && base.category != filter {
            continue;
        }
        // Solution retenue (projets à plusieurs prestataires).
        let opt: Option<String> = if base.options.is_empty() {
            None
        } else {
            Some(
                c.ui.project_option
                    .get(&base.id)
                    .filter(|o| base.option(o).is_some())
                    .cloned()
                    .unwrap_or_else(|| base.options[0].id.clone()),
            )
        };
        let p = base.with_option(opt.as_deref());
        let check = projects::can_start_with(c.content, g, &base.id, opt.as_deref());
        if filter.is_empty() && check.is_err() {
            continue;
        }
        let ok = check.is_ok() && g.agenda > 0;
        card().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(&base.name).font(bold(14.5)).color(if ok {
                    TEXT
                } else {
                    MUTED
                }));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add_enabled(ok, egui::Button::new("Lancer")).clicked() {
                        *cmd = Some(match &opt {
                            Some(o) => Command::StartProjectWith(base.id.clone(), o.clone()),
                            None => Command::StartProject(base.id.clone()),
                        });
                    }
                });
            });
            muted(ui, &base.desc);
            if !base.options.is_empty() {
                ui.label(
                    RichText::new(format!("Solution ({}) :", base.options.len()))
                        .font(bold(12.5))
                        .color(CYAN),
                );
                ui.horizontal_wrapped(|ui| {
                    for o in &base.options {
                        let sel = opt.as_deref() == Some(o.id.as_str());
                        if ui
                            .selectable_label(
                                sel,
                                RichText::new(&o.name).size(13.0).color(if sel {
                                    YELLOW
                                } else {
                                    LIGHT
                                }),
                            )
                            .clicked()
                        {
                            c.ui.project_option.insert(base.id.clone(), o.id.clone());
                        }
                    }
                });
                if let Some(o) = opt.as_deref().and_then(|o| base.option(o)) {
                    ui.add(
                        egui::Label::new(RichText::new(&o.desc).size(12.5).italics().color(LIGHT))
                            .wrap(),
                    );
                }
            }
            ui.label(
                RichText::new(format!(
                    "Lancement {:.0} · {:.2}/mois · {} mois · complexité {:.0} · risque {:.0} %",
                    p.cost,
                    p.burn,
                    p.duration,
                    p.complexity,
                    p.risk * 100.0
                ))
                .size(12.0)
                .color(LIGHT),
            );
            let mut fx: Vec<_> = p.on_done.iter().map(|(m, d)| (*m, *d)).collect();
            fx.retain(|(_, d)| d.abs() >= 0.5);
            if !fx.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("À la livraison :").size(12.0).color(MUTED));
                    preview(ui, &fx, false);
                });
            }
            if !p.add_contract.is_empty() || !p.end_contract.is_empty() {
                let label = |id: &String| {
                    c.content
                        .contract(id)
                        .map(|k| k.label.clone())
                        .unwrap_or_default()
                };
                let mut parts = Vec::new();
                if !p.end_contract.is_empty() {
                    parts.push(format!(
                        "Résilie : {}",
                        p.end_contract
                            .iter()
                            .map(label)
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                if !p.add_contract.is_empty() {
                    let cost: f32 = p
                        .add_contract
                        .iter()
                        .filter_map(|id| c.content.contract(id))
                        .map(|k| k.monthly_cost)
                        .sum();
                    parts.push(format!(
                        "Souscrit : {} ({cost:.2}/mois)",
                        p.add_contract
                            .iter()
                            .map(label)
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                ui.add(
                    egui::Label::new(RichText::new(parts.join(" · ")).size(12.0).color(MUTED))
                        .wrap(),
                );
            }
            if !p.add_tech.is_empty() {
                let names: Vec<String> =
                    p.add_tech.iter().map(|t| c.content.tech_name(t)).collect();
                ui.label(
                    RichText::new(format!("Débloque : {}", names.join(", ")))
                        .size(12.0)
                        .color(CYAN),
                );
            }
            if let Err(e) = check {
                ui.label(RichText::new(format!("🔒 {e}")).size(12.0).color(ORANGE));
            }
        });
    }
}

fn recruit(ui: &mut egui::Ui, c: &mut Ctx, cmd: &mut Option<Command>) {
    section(ui, "RECRUTER (ENTRETIEN — 1 CRÉNEAU)");
    muted(
        ui,
        "Chaque recrue augmente la capacité interne (MAKE) et une compétence, coûte chaque mois, et peut partir si l'ambiance se dégrade.",
    );
    for r in &c.content.careers.roles {
        let g = c.g();
        let ok = r.requires.met(g) && g.agenda > 0;
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    ok,
                    egui::Button::new(RichText::new(&r.label).size(14.0))
                        .min_size(egui::vec2(200.0, 28.0)),
                )
                .on_hover_text(&r.pitch)
                .clicked()
            {
                *cmd = Some(Command::Recruit(r.id.clone()));
            }
            muted(ui, format!("{:.2} pt/mois", r.salary));
            if !r.requires.met(g) {
                ui.label(
                    RichText::new(format!(
                        "🔒 {}",
                        r.requires.explain(g, &|t| c.content.tech_name(t))
                    ))
                    .size(12.0)
                    .color(ORANGE),
                );
            }
        });
    }
}

fn negotiations(ui: &mut egui::Ui, c: &mut Ctx, cmd: &mut Option<Command>) {
    section(ui, "NÉGOCIER EN AVANCE (1 CRÉNEAU)");
    muted(
        ui,
        "Plus vous négociez tôt (2 à 9 mois avant l'échéance), plus vous avez de levier. Le pré-accord s'applique à l'échéance.",
    );
    let now = c.g().turn();
    for st in c.g().contracts.clone().iter().filter(|x| x.active) {
        let Some(def) = c.content.contract(&st.id) else {
            continue;
        };
        let left = st.months_left(now);
        let ok = st.pre_deal.is_none() && (1..=9).contains(&left) && c.g().agenda > 0;
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    ok,
                    egui::Button::new(RichText::new(&def.label).size(13.5))
                        .min_size(egui::vec2(260.0, 26.0)),
                )
                .clicked()
            {
                *cmd = Some(Command::Negotiate(st.id.clone()));
            }
            let note = if st.pre_deal.is_some() {
                "pré-accord signé".to_string()
            } else if left > 9 {
                format!("échéance dans {left} mois : trop tôt")
            } else {
                format!("échéance dans {} mois", left.max(0))
            };
            muted(ui, note);
        });
    }
}
