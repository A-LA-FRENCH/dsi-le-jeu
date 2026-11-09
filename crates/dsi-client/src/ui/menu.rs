//! Écran titre, création du ou de la DSI, réglages, aide, crédits.

use super::style::*;
use super::widgets::*;
use super::{Ctx, Screen};
use bevy_egui::egui::{self, Color32, RichText};
use dsi_core::Profile;

pub fn title(ctx: &egui::Context, c: &mut Ctx) {
    dim(ctx, 120);
    egui::Area::new("title".into()).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0)).order(egui::Order::Foreground).show(ctx, |ui| {
        modal_frame(YELLOW).show(ui, |ui| {
            ui.set_width(560.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("LE JEU · DIRECTION DES SYSTÈMES D'INFORMATION").font(pixel(12.0)).color(MUTED));
                ui.label(RichText::new("DSI — LE JEU").font(pixel(58.0)).color(YELLOW));
                ui.label(RichText::new("« Votre SI date de 1998. Votre budget vient d'être coupé de 15 %. Bonne chance. »").size(16.0).italics().color(LIGHT));
                ui.add_space(14.0);
                if c.session.has_save && big_button(ui, "Continuer la partie", true, 300.0).clicked() {
                    match c.session.load(c.store) {
                        Ok(()) => c.ui.screen = Screen::Playing,
                        Err(e) => c.ui.toast(format!("Impossible de charger : {e}"), false),
                    }
                }
                ui.add_space(4.0);
                let first = !c.session.has_save;
                let b = if first { big_button(ui, "Nouvelle partie", true, 300.0) } else { ghost_button(ui, "Nouvelle partie", 300.0) };
                if b.clicked() {
                    c.ui.screen = Screen::Create;
                    c.click();
                }
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add_space(42.0);
                    if ghost_button(ui, "Charger une partie", 180.0).on_hover_text("3 emplacements de sauvegarde").clicked() {
                        super::saves::open(c);
                    }
                    if ghost_button(ui, "Réglages", 140.0).clicked() {
                        c.ui.show_settings = true;
                    }
                    if ghost_button(ui, "Crédits", 140.0).clicked() {
                        c.ui.show_credits = true;
                    }
                });
                ui.add_space(12.0);
                ui.label(RichText::new(&c.content.dialogues.disclaimer).size(11.5).color(MUTED));
                if c.session.params.debug {
                    ui.label(RichText::new("MODE DEBUG ACTIF").font(pixel(12.0)).color(RED));
                }
            });
        });
    });
}

/// Nombre d'avatars par genre (voir AVATARS dans tools/art/gen_art.py).
pub const AVATARS_PER_GENDER: u32 = 8;

pub fn create(ctx: &egui::Context, c: &mut Ctx) {
    let mut next = false;
    let mut back = false;
    // L'avatar doit correspondre au genre choisi.
    let range = if c.ui.female {
        0..AVATARS_PER_GENDER
    } else {
        AVATARS_PER_GENDER..2 * AVATARS_PER_GENDER
    };
    if !range.contains(&c.ui.avatar) {
        c.ui.avatar = range.start;
    }
    modal(ctx, "create", 680.0, YELLOW, |ui| {
        ui.label(
            RichText::new("PRISE DE FONCTIONS · 1/2")
                .font(pixel(12.0))
                .color(MUTED),
        );
        ui.label(
            RichText::new("Qui est le ou la nouvelle DSI ?")
                .font(pixel(26.0))
                .color(YELLOW),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            portrait(ui, c.tex, super::AVATAR_SLOT + c.ui.avatar, 80.0);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Nom :");
                    ui.add(
                        egui::TextEdit::singleline(&mut c.ui.name)
                            .id_salt("dsi_name")
                            .desired_width(260.0)
                            .char_limit(32),
                    );
                });
                ui.horizontal(|ui| {
                    ui.radio_value(&mut c.ui.female, true, "Directrice des SI");
                    ui.radio_value(&mut c.ui.female, false, "Directeur des SI");
                });
                ui.horizontal(|ui| {
                    ui.label("Entreprise :");
                    ui.add(
                        egui::TextEdit::singleline(&mut c.ui.company)
                            .id_salt("dsi_company")
                            .desired_width(230.0)
                            .char_limit(28),
                    );
                });
            });
        });
        section(ui, "APPARENCE");
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            for k in range.clone() {
                let sel = c.ui.avatar == k;
                let resp = egui::Frame::new()
                    .stroke(egui::Stroke::new(
                        if sel { 2.5 } else { 1.0 },
                        if sel { YELLOW } else { LINE },
                    ))
                    .corner_radius(3)
                    .inner_margin(egui::Margin::same(2))
                    .show(ui, |ui| portrait(ui, c.tex, super::AVATAR_SLOT + k, 52.0))
                    .response
                    .interact(egui::Sense::click());
                if resp
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    c.ui.avatar = k;
                }
            }
        });
        section(ui, "PARCOURS");
        let bgs = c.content.dialogues.backgrounds.clone();
        ui.columns(2, |cols| {
            for (i, b) in bgs.iter().enumerate() {
                let ui = &mut cols[i % 2];
                let sel = c.ui.background == b.id;
                let resp = egui::Frame::new()
                    .fill(if sel {
                        Color32::from_rgb(58, 54, 30)
                    } else {
                        CARD
                    })
                    .stroke(egui::Stroke::new(
                        if sel { 2.0 } else { 1.0 },
                        if sel { YELLOW } else { LINE },
                    ))
                    .corner_radius(3)
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new(&b.label).font(bold(14.5)));
                        muted(ui, &b.desc);
                        let list: Vec<_> = b.effects.iter().map(|(m, d)| (*m, *d)).collect();
                        deltas(ui, &list);
                    })
                    .response
                    .interact(egui::Sense::click());
                if resp
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    c.ui.background = b.id.clone();
                }
                ui.add_space(4.0);
            }
        });
        ui.horizontal(|ui| {
            ui.label("Graine (facultatif) :");
            ui.add(
                egui::TextEdit::singleline(&mut c.ui.seed_text)
                    .id_salt("dsi_seed")
                    .desired_width(120.0)
                    .hint_text("aléatoire"),
            );
            muted(ui, "Même graine = mêmes aléas. Aussi via ?seed=1234");
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if big_button(
                ui,
                "Choisir son mentor ▶",
                !c.ui.name.trim().is_empty(),
                300.0,
            )
            .clicked()
            {
                next = true;
            }
            if ghost_button(ui, "Retour", 100.0).clicked() {
                back = true;
            }
        });
    });
    if back {
        c.ui.screen = Screen::Title;
    }
    if next {
        c.ui.screen = Screen::Mentor;
        c.click();
    }
}

/// Deuxième écran de création : le mentor principal et le tutoriel.
pub fn mentor_choice(ctx: &egui::Context, c: &mut Ctx) {
    let mut start = false;
    let mut back = false;
    if c.content.mentor(&c.ui.mentor).is_none() {
        c.ui.mentor = "mehdi".into();
    }
    modal(ctx, "mentor_choice", 900.0, YELLOW, |ui| {
        ui.label(
            RichText::new("PRISE DE FONCTIONS · 2/2")
                .font(pixel(12.0))
                .color(MUTED),
        );
        ui.label(
            RichText::new("Choisis ton mentor")
                .font(pixel(26.0))
                .color(YELLOW),
        );
        muted(
            ui,
            "Les trois animateurs d'A la French répondront quand tu les appelleras. Ton mentor principal apporte un bonus permanent, te guide dans le tutoriel et revient deux fois plus vite (tous les 2 mois au lieu de 4).",
        );
        ui.add_space(6.0);
        let mentors = c.content.mentors.clone();
        ui.columns(3, |cols| {
            for (i, m) in mentors.iter().enumerate() {
                let ui = &mut cols[i];
                let sel = c.ui.mentor == m.id;
                let accent = hex(&m.color);
                let resp = egui::Frame::new()
                    .fill(if sel {
                        Color32::from_rgb(40, 46, 34)
                    } else {
                        CARD
                    })
                    .stroke(egui::Stroke::new(
                        if sel { 2.5 } else { 1.0 },
                        if sel { accent } else { LINE },
                    ))
                    .corner_radius(3)
                    .inner_margin(egui::Margin::same(10))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.vertical_centered(|ui| {
                            super::widgets::mentor_portrait(ui, c.mentors_tex, i as u32, 96.0)
                        });
                        ui.label(RichText::new(&m.name).font(pixel(20.0)).color(accent));
                        ui.add(
                            egui::Label::new(RichText::new(&m.title).size(12.5).color(LIGHT))
                                .wrap(),
                        );
                        ui.add_space(2.0);
                        ui.add(
                            egui::Label::new(RichText::new(&m.bio).size(12.5).color(MUTED)).wrap(),
                        );
                        ui.add_space(2.0);
                        for sk in &m.skills {
                            ui.label(RichText::new(format!("• {sk}")).size(12.5).color(TEXT));
                        }
                        ui.add_space(4.0);
                        ui.label(RichText::new("Bonus permanent").size(11.5).color(MUTED));
                        ui.add(
                            egui::Label::new(
                                RichText::new(&m.passive_label)
                                    .font(bold(13.0))
                                    .color(GREEN),
                            )
                            .wrap(),
                        );
                        if sel {
                            ui.label(
                                RichText::new("✔ Mentor principal")
                                    .font(bold(13.0))
                                    .color(accent),
                            );
                        }
                    })
                    .response
                    .interact(egui::Sense::click());
                if resp
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    c.ui.mentor = m.id.clone();
                    c.click();
                }
            }
        });
        ui.add_space(6.0);
        ui.checkbox(
            &mut c.ui.tutorial,
            "Tutoriel : ton mentor te guide pendant les premiers mois (recommandé)",
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if big_button(ui, "Prendre ses fonctions", true, 300.0).clicked() {
                start = true;
            }
            if ghost_button(ui, "◀ Retour", 100.0).clicked() {
                back = true;
            }
        });
    });
    if back {
        c.ui.screen = Screen::Create;
    }
    if start {
        let seed =
            c.ui.seed_text
                .trim()
                .parse::<u64>()
                .ok()
                .unwrap_or_else(|| platform::clock::now_millis() % 1_000_000);
        let company = if c.ui.company.trim().is_empty() {
            "Groupe Duval-Montaigne".to_string()
        } else {
            c.ui.company.trim().to_string()
        };
        let profile = Profile {
            name: c.ui.name.trim().to_string(),
            female: c.ui.female,
            company,
            background: c.ui.background.clone(),
            avatar: Some(c.ui.avatar),
            mentor: c.ui.mentor.clone(),
        };
        c.session.new_game(c.content, seed, profile);
        if c.ui.tutorial {
            c.run(dsi_core::Command::SetTutorial(0));
        }
        c.ui.screen = Screen::Playing;
        c.ui.tab = super::Tab::Inbox;
        c.ui.seed_text.clear();
    }
}

pub fn settings(ctx: &egui::Context, c: &mut Ctx) {
    let mut close = false;
    modal(ctx, "settings", 460.0, LINE, |ui| {
        ui.label(RichText::new("Réglages").font(pixel(24.0)).color(YELLOW));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Taille du texte");
            ui.add(egui::Slider::new(&mut c.settings.ui_scale, 0.8..=1.4).step_by(0.05));
        });
        ui.checkbox(
            &mut c.settings.music,
            "Musique (thème d'A la French en 16 bits)",
        );
        ui.add_enabled_ui(c.settings.music, |ui| {
            ui.horizontal(|ui| {
                ui.label("Volume de la musique");
                ui.add(
                    egui::Slider::new(&mut c.settings.music_volume, 0.0..=1.0).show_value(false),
                );
            });
        });
        ui.checkbox(&mut c.settings.sound, "Effets sonores");
        ui.checkbox(&mut c.settings.reduce_motion, "Réduire les animations");
        if let Some(g) = c.session.game.as_mut() {
            let mut on = g.tutorial != dsi_core::state::TUTORIAL_OFF;
            if ui
                .checkbox(
                    &mut on,
                    "Bulles du mentor (tutoriel et conseils de prochaine action)",
                )
                .changed()
            {
                g.tutorial = if on { 0 } else { dsi_core::state::TUTORIAL_OFF };
                c.session.rev += 1;
            }
        }
        ui.checkbox(&mut c.settings.high_contrast, "Contraste renforcé");
        ui.add_space(6.0);
        muted(
            ui,
            "Clavier : 1–5 choisir · Entrée continuer · Espace mois suivant · A/I/C/T/E onglets · S sauvegardes · Échap réglages · Tab pour naviguer.",
        );
        ui.add_space(8.0);
        if c.session.game.is_some() {
            ui.horizontal(|ui| {
                if ghost_button(ui, "Retour à l'écran titre", 200.0)
                    .on_hover_text("La partie est sauvegardée automatiquement.")
                    .clicked()
                {
                    c.session.game = None;
                    c.ui.screen = Screen::Title;
                    close = true;
                }
                if ghost_button(ui, "Aide", 80.0).clicked() {
                    c.ui.show_help = true;
                    close = true;
                }
            });
            if ghost_button(ui, "Sauvegarder / charger (S)", 300.0).clicked() {
                super::saves::open(c);
                close = true;
            }
        }
        ui.add_space(6.0);
        if c.session.has_save {
            if !c.ui.confirm_reset {
                if ui
                    .add(egui::Button::new(RichText::new("RESET GAME").color(RED)))
                    .on_hover_text("Efface la sauvegarde automatique (les 3 emplacements manuels sont conservés)")
                    .clicked()
                {
                    c.ui.confirm_reset = true;
                }
            } else {
                ui.label(
                    RichText::new("Effacer définitivement la partie sauvegardée ?").color(RED),
                );
                ui.horizontal(|ui| {
                    if ui
                        .button(RichText::new("Oui, tout effacer").color(RED))
                        .clicked()
                    {
                        let _ = c.store.0.remove(dsi_core::save::AUTOSAVE_KEY);
                        c.session.has_save = false;
                        c.session.game = None;
                        c.ui.screen = Screen::Title;
                        c.ui.confirm_reset = false;
                        c.ui.toast("Partie effacée. Le SI de 1998 vous attend à nouveau.", true);
                        close = true;
                    }
                    if ui.button("Annuler").clicked() {
                        c.ui.confirm_reset = false;
                    }
                });
            }
        }
        ui.add_space(8.0);
        if big_button(ui, "Fermer", true, 140.0).clicked() {
            close = true;
        }
    });
    if close {
        c.ui.show_settings = false;
    }
}

pub fn help(ctx: &egui::Context, c: &mut Ctx) {
    let mut close = false;
    modal(ctx, "help", 620.0, LINE, |ui| {
        ui.label(
            RichText::new("Comment jouer")
                .font(pixel(24.0))
                .color(YELLOW),
        );
        for (t, d) in [
            (
                "1 tour = 1 mois",
                "Chaque mois apporte des sujets (bulles au-dessus des salles, liste « À traiter »). Traitez-les tous, puis passez au mois suivant.",
            ),
            (
                "Le temps est la ressource rare",
                "Vous avez 3 créneaux d'agenda par mois. Visiter une salle pour agir (projet, recrutement, négociation, action) en consomme un.",
            ),
            (
                "Chaque décision fait mal quelque part",
                "Les flèches ▲▼ indiquent le sens des effets, pas leur ampleur. ⏳ signale des conséquences différées.",
            ),
            (
                "Équipe et salaires",
                "Recrutez depuis la salle RH ; chaque recrue coûte chaque mois. Onglet Équipe : « Licencier… » arrête un salaire (1 créneau ; sans indemnités pendant la période d'essai de 3 mois, puis 3 mois de salaire), au prix d'un coup au moral et à l'attractivité.",
            ),
            (
                "Projets",
                "Lancés depuis la salle projet : coût de lancement, coût mensuel, durée, risque de glissement. 5 en parallèle au maximum.",
            ),
            (
                "Contrats",
                "Les renouvellements arrivent tout seuls. Négocier tôt (Achats) donne du levier ; attendre J-2 coûte cher.",
            ),
            (
                "Défaites",
                "COMEX à 0, sécurité à 0, budget à 0, dette à 100, dépendance à 100, MAKE à 0, complexité à 100.",
            ),
            (
                "Objectif",
                "Survivre trois ans et transformer le SI : BUY → MAKE, Waterfall → Agentic. La fin « Pragmatic CIO » est la plus difficile. Une fois la fin atteinte, vous pouvez prolonger de trois ans (saison 2, années 4 à 6) en choisissant votre cap : la fin visée, avec ses conditions chiffrées (onglet Indicateurs). Cap tenu : +5 points.",
            ),
        ] {
            ui.label(RichText::new(t).font(bold(15.0)).color(TEXT));
            muted(ui, d);
            ui.add_space(2.0);
        }
        ui.add_space(6.0);
        if big_button(ui, "Compris", true, 140.0).clicked() {
            close = true;
        }
    });
    if close {
        c.ui.show_help = false;
    }
}

pub fn credits(ctx: &egui::Context, c: &mut Ctx) {
    let mut close = false;
    modal(ctx, "credits", 560.0, LINE, |ui| {
        ui.label(
            RichText::new("Crédits & mentions")
                .font(pixel(24.0))
                .color(YELLOW),
        );
        muted(
            ui,
            "Un jeu de gestion satirique sur la vie d'une Direction des Systèmes d'Information, dans l'esprit d'« A la French — le jeu ».",
        );
        ui.add_space(4.0);
        ui.label("Rust · Bevy · egui · WebAssembly · WebGPU / WebGL2. Simulation déterministe et testée (crate dsi-core).");
        ui.label("Pixel art, effets sonores et textes originaux, générés pour le projet.");
        ui.label("Musique et portraits des mentors (Mehdi, Steeve, Jean-Baptiste) repris d'« A la French — le jeu » : thème du podcast en 16 bits, « La Marseillaise » en 8-bit, gigue celtique.");
        ui.label("Polices : Pixelify Sans et IBM Plex Sans — SIL Open Font License 1.1.");
        ui.add_space(6.0);
        ui.label(RichText::new(&c.content.dialogues.disclaimer).color(ORANGE));
        muted(
            ui,
            "Les noms d'entreprises sont cités en texte comme repères du marché, sans logo. Les répliques des commerciaux et les situations sont des caricatures fictives et génériques.",
        );
        ui.add_space(8.0);
        if big_button(ui, "Fermer", true, 140.0).clicked() {
            close = true;
        }
    });
    if close {
        c.ui.show_credits = false;
    }
}
