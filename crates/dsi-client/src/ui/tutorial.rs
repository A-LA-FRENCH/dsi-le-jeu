//! Tutoriel : le mentor principal guide les premiers mois par des bulles qui
//! désignent l'élément d'interface à utiliser, et avancent quand l'action est faite.
//! Une fois le tutoriel terminé, une bulle de « prochaine action » accompagne encore
//! le joueur pendant le premier semestre.

use super::Ctx;
use super::style::*;
use super::widgets::*;
use crate::world::{self, WorldViewport};
use bevy_egui::egui::{self, Color32, RichText};
use dsi_core::Command;
use dsi_core::state::{Active, Metric, Room, TUTORIAL_DONE, TUTORIAL_OFF};

/// Ordre des étapes : la section « critères » (20 à 22) s'insère après les indicateurs.
const ORDER: [u32; 19] = [
    0, 1, 20, 21, 22, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
];
/// Nombre d'étapes affiché (« 3/19 »).
const STEPS: u32 = ORDER.len() as u32;

/// Étape suivante (bouton « Suivant »).
fn next_step(step: u32) -> u32 {
    match ORDER.iter().position(|&s| s == step) {
        Some(i) if i + 1 < ORDER.len() => ORDER[i + 1],
        _ => TUTORIAL_DONE,
    }
}

/// Rang affiché d'une étape (1 à STEPS).
fn display_index(step: u32) -> u32 {
    ORDER
        .iter()
        .position(|&s| s == step)
        .map_or(STEPS, |i| i as u32 + 1)
}
/// Les conseils de prochaine action restent affichés jusqu'à ce mois (absolu, exclu).
const TIPS_UNTIL: u32 = 6;

#[derive(Clone, Copy)]
enum Target {
    None,
    Mark(&'static str),
    Room(Room),
    /// À gauche de l'écran, à côté d'une fenêtre modale centrée.
    Side,
}

struct Bubble {
    text: String,
    target: Target,
    /// Bouton « Suivant » (sinon l'étape avance quand l'action est faite).
    next: bool,
}

fn set_step(c: &mut Ctx, step: u32) {
    if let Some(g) = c.session.game.as_mut()
        && g.tutorial != step
    {
        g.tutorial = step;
        c.session.rev += 1;
    }
}

/// Fait avancer les étapes dont l'action est accomplie, puis décrit la bulle à afficher.
fn step_bubble(c: &mut Ctx) -> Option<Bubble> {
    loop {
        let g = c.g();
        let step = g.tutorial;
        let active = g.active.is_some();
        let turn = g.turn();
        let next = match step {
            // Le joueur est déjà passé au mois suivant : on saute à l'étape des contrats.
            0..=9 | 20..=22 if turn >= 1 && !active => Some(10),
            0..=3
                if matches!(
                    g.active,
                    Some(Active::Event { .. } | Active::Chapter { .. } | Active::Renewal { .. })
                ) =>
            {
                Some(4)
            }
            4 if !active && g.decisions.iter().any(|d| !d.starts_with("mentor:")) => Some(5),
            5 if !active && g.inbox.is_empty() => Some(6),
            6 if g.agenda == 0 || g.agenda < g.agenda_max && c.ui.room_panel.is_none() => Some(8),
            6 if c.ui.room_panel.is_some() => Some(7),
            7 if c.ui.room_panel.is_none() => Some(8),
            8 if g.projects.is_empty() => Some(9),
            9 if turn >= 1 => Some(10),
            10 if c.ui.tab == super::Tab::Contracts => Some(11),
            12 if c.view.plateau => Some(13),
            14 if g.mentors.consults > 0 && !active => Some(15),
            _ => None,
        };
        match next {
            Some(n) => set_step(c, n),
            None => break,
        }
    }
    let g = c.g();
    let step = g.tutorial;
    let modal_open =
        g.active.is_some() || c.ui.show_mentors || c.ui.show_settings || c.ui.show_help;
    let agenda = g.agenda;
    let pending = g.inbox.len();
    let b = |text: &str, target: Target, next: bool| {
        Some(Bubble {
            text: g.tx(text),
            target,
            next,
        })
    };
    match step {
        0 if !modal_open && c.ui.room_panel.is_none() => {
            let hello = c
                .content
                .mentor(&g.profile.mentor)
                .map(|m| m.hello.clone())
                .unwrap_or_default();
            Some(Bubble {
                text: format!("{} Tu peux passer ce tutoriel à tout moment.", g.tx(&hello)),
                target: Target::None,
                next: true,
            })
        }
        1 if !modal_open => b(
            "Voici tes 18 indicateurs. La première ligne est vitale : si le COMEX, le budget, la sécurité ou le MAKE tombent à 0, ou si la dette, le lock-in ou la complexité atteignent 100, c'est fini. Leur nom clignote en rouge quand ça devient dangereux. Survole une jauge pour le détail.",
            Target::Mark("gauges"),
            true,
        ),
        20 if !modal_open => {
            let parts: Vec<&str> = dsi_core::score::SCORE_PARTS
                .iter()
                .map(|(m, _)| m.label())
                .collect();
            Some(Bubble {
                text: format!(
                    "Parlons critères. Ton score sur 100 est la moyenne de 10 composantes : {}. Pour la dette, moins c'est mieux. Ton rang va de « Directeur du PowerPoint » à « Grand architecte ». Au-delà de 95, on vérifie que tu n'as pas triché.",
                    parts.join(", ")
                ),
                target: Target::Mark("tab_metrics"),
                next: true,
            })
        }
        21 if !modal_open => {
            let year = c
                .content
                .dialogues
                .years
                .iter()
                .find(|y| y.year == g.cal.year.max(1));
            let objs = year
                .map(|y| {
                    y.objectives
                        .iter()
                        .map(|o| format!("« {} »", o.label))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            Some(Bubble {
                text: format!(
                    "Chaque décembre, le COMEX annuel te juge sur les objectifs de l'année. Cette année : {objs}. Objectif tenu : la confiance du COMEX monte ; raté : elle chute. C'est la jauge qui te fait virer. Tu les retrouves dans l'onglet Indicateurs."
                ),
                target: Target::Mark("tab_metrics"),
                next: true,
            })
        }
        22 if !modal_open => b(
            "Et à la fin ? En décembre de l'année 3, ton profil décide : Software Company, DSI souveraine, Cloud Native, Engineering Culture, Agentic Enterprise ou Pragmatic CIO, la plus dure, où il faut être bon partout. Sinon : « Toujours en poste ». Les conditions exactes sont dans Indicateurs › Objectifs de fin. Choisis tôt où tu veux aller : on ne devient pas DSI souveraine en décembre.",
            Target::Mark("tab_metrics"),
            true,
        ),
        2 if !modal_open => b(
            "Ta vraie ressource, c'est le temps : 3 créneaux d'agenda par mois. Lancer un projet, recruter ou négocier dans une salle en consomme un.",
            Target::Mark("agenda"),
            true,
        ),
        3 if !modal_open => b(
            "Chaque mois, des sujets tombent : une bulle « ! » au-dessus d'une salle du siège, et une carte dans « À traiter ». Clique sur le premier sujet, ou sur le gros bouton orange en bas à droite.",
            Target::Mark("inbox_first"),
            false,
        ),
        4 => match &g.active {
            Some(Active::Chapter { .. }) => b(
                "Un chapitre commence : les personnages posent le décor. Lis chaque page (« Suite » ou Entrée), puis « C'est parti ».",
                Target::Side,
                false,
            ),
            Some(Active::Event { .. }) => b(
                "Chaque décision fait mal quelque part. Les flèches ▲▼ montrent le sens des effets, jamais leur ampleur, et ⏳ annonce des suites dans les mois à venir. Choisis avec la souris ou les touches 1 à 4.",
                Target::Side,
                false,
            ),
            Some(Active::Result(_)) => b(
                "Voici les effets réels de ta décision. Lis les notes : certaines conséquences arriveront plus tard. « Continuer » (ou Entrée).",
                Target::Side,
                false,
            ),
            Some(_) => b("Prends ton temps : lis, puis décide.", Target::Side, false),
            None => b(
                "Ouvre maintenant un événement : clique sur une carte « À traiter » ou sur une bulle du siège.",
                Target::Mark("inbox_first"),
                false,
            ),
        },
        5 if !modal_open && pending > 0 => {
            let t = format!(
                "Bien joué. Il te reste {pending} sujet(s) ce mois-ci : traite-les tous avant de passer au mois suivant."
            );
            Some(Bubble {
                text: t,
                target: Target::Mark("inbox_first"),
                next: false,
            })
        }
        6 if !modal_open => {
            let t = format!(
                "Tu as encore {agenda} créneau(x). Clique sur la SALLE PROJET du siège : la cartographie du SI est un objectif de l'année. Chaque salle propose ses propres actions."
            );
            Some(Bubble {
                text: t,
                target: if c.view.plateau {
                    Target::Mark("view_switch")
                } else {
                    Target::Room(Room::Projet)
                },
                next: false,
            })
        }
        7 => b(
            "Chaque action coûte un créneau. Regarde les effets à la livraison et le coût mensuel d'un projet : cinq projets en parallèle au maximum. Ferme la fenêtre (Échap) quand tu as fini.",
            Target::Side,
            false,
        ),
        8 if !modal_open => b(
            "Tes projets avancent ici, mois après mois, et peuvent glisser. « ⚡ ESN » accélère un projet mais coûte plus cher et crée de la dépendance.",
            Target::Mark("projects_bar"),
            true,
        ),
        9 if !modal_open => b(
            "Quand tu as fini, passe au mois suivant avec ce bouton (ou la barre d'espace). Le budget tourne : en bas à droite, le budget mensuel se répartit entre salaires, logiciels, ESN, cloud / IA et projets. Si le solde est négatif, la jauge BUDGET baisse chaque mois.",
            Target::Mark("month_btn"),
            false,
        ),
        10 if !modal_open => b(
            "Nouveau mois, nouveaux sujets. Avant tout : ouvre l'onglet Contrats. Les renouvellements arrivent tout seuls, et VMware, c'est en juin.",
            Target::Mark("tab_contracts"),
            false,
        ),
        11 if !modal_open => b(
            "Négocier 2 à 9 mois avant l'échéance (bouton ici, ou salle Achats) te donne du levier. Attendre la date du renouvellement, c'est −40 de rapport de force.",
            Target::Mark("contracts_list"),
            true,
        ),
        12 if !modal_open => b(
            "Touche V ou ce bouton : le plateau DSI, l'open space où tu vois ton équipe travailler. Va jeter un œil.",
            Target::Mark("view_switch"),
            true,
        ),
        13 if !modal_open => b(
            "Pour l'instant, ce sont surtout des prestataires ESN. Chaque recrue (salle RH du siège) s'installe ici, augmente ta capacité interne… et coûte chaque mois.",
            Target::None,
            true,
        ),
        14 if !modal_open => b(
            "Et moi ? Clique sur mon portrait quand tu veux un conseil : c'est gratuit en agenda, avec un petit bonus adapté à ta situation. Je reviens tous les 2 mois.",
            Target::Mark("mentor"),
            true,
        ),
        15 if !modal_open => b(
            "Tu as l'essentiel. Je te glisserai encore une piste de temps en temps pendant le premier semestre. Bonne chance : ton SI date de 1998.",
            Target::None,
            true,
        ),
        _ => None,
    }
}

/// Conseil de « prochaine action » après le tutoriel.
fn next_action(c: &Ctx) -> Option<Bubble> {
    let g = c.g();
    if g.active.is_some() || c.ui.room_panel.is_some() || c.ui.show_mentors || c.ui.show_settings {
        return None;
    }
    let now = g.turn();
    let mut prefix = String::new();
    if g.m(Metric::Budget) < 20.0 {
        prefix = format!(
            "Attention au budget ({:.0}) : évite les gros projets. ",
            g.m(Metric::Budget)
        );
    }
    let (text, target) = if !g.inbox.is_empty() {
        (
            format!(
                "Il te reste {} sujet(s) à traiter : clique sur les bulles du siège ou sur « À traiter ».",
                g.inbox.len()
            ),
            Target::Mark("inbox_first"),
        )
    } else if g.agenda > 0 {
        let main = c.content.mentor(&g.profile.mentor);
        let due = g
            .contracts
            .iter()
            .filter(|x| x.active && x.pre_deal.is_none() && (2..=9).contains(&x.months_left(now)))
            .min_by_key(|x| x.months_left(now));
        let can_project =
            g.projects.len() < dsi_core::projects::MAX_PARALLEL && g.m(Metric::Budget) > 20.0;
        if let Some(m) =
            main.filter(|m| g.mentors.consults == 0 && dsi_core::mentors::wait(g, m) == 0)
        {
            (
                format!(
                    "Demande-moi un conseil : c'est gratuit, et {} a toujours un bonus en poche.",
                    m.name
                ),
                Target::Mark("mentor"),
            )
        } else if let Some(x) = due {
            let label = c
                .content
                .contract(&x.id)
                .map(|d| d.label.clone())
                .unwrap_or_default();
            (
                format!(
                    "{label} arrive à échéance dans {} mois : négocie maintenant aux Achats, c'est là que tu as le plus de levier.",
                    x.months_left(now)
                ),
                Target::Room(Room::Achats),
            )
        } else if can_project
            && !g.done_projects.contains("cartographie")
            && !g.projects.iter().any(|p| p.id == "cartographie")
        {
            (
                "Lance la cartographie du SI en salle projet : c'est un objectif de l'année."
                    .into(),
                Target::Room(Room::Projet),
            )
        } else if g.team.is_empty() {
            ("Ton plateau est vide : recrute un premier développeur à la RH pour augmenter ta capacité interne (MAKE).".into(), Target::Room(Room::Rh))
        } else if g.m(Metric::Security) < 40.0 {
            ("Ta sécurité est basse : passe au SOC ou en cybersécurité (correctifs, campagne anti-phishing).".into(), Target::Room(Room::Soc))
        } else if g.m(Metric::Debt) > 70.0 {
            ("La dette grimpe : un sprint de remboursement dans l'open space devs ferait du bien.".into(), Target::Room(Room::Devs))
        } else if can_project {
            (
                format!(
                    "Tu as {} projet(s) sur {} : la salle projet en propose d'autres.",
                    g.projects.len(),
                    dsi_core::projects::MAX_PARALLEL
                ),
                Target::Room(Room::Projet),
            )
        } else {
            (
                format!(
                    "Il te reste {} créneau(x) : utilise-les dans une salle, ou passe au mois suivant.",
                    g.agenda
                ),
                Target::Mark("month_btn"),
            )
        }
    } else {
        (
            "Plus de créneau ce mois-ci : passe au mois suivant (barre d'espace).".into(),
            Target::Mark("month_btn"),
        )
    };
    Some(Bubble {
        text: format!("{prefix}{text}"),
        target,
        next: false,
    })
}

fn target_rect(c: &Ctx, t: Target, vp: &WorldViewport) -> Option<egui::Rect> {
    match t {
        Target::Mark(k) => c.ui.marks.get(k).copied(),
        Target::Room(r) if !c.view.plateau => {
            let w = world::room_world_rect(r);
            let a = world::world_to_screen(vp, w.min);
            let b = world::world_to_screen(vp, w.max);
            Some(egui::Rect::from_two_pos(
                egui::pos2(a.x, a.y),
                egui::pos2(b.x, b.y),
            ))
        }
        _ => None,
    }
}

pub fn show(ctx: &egui::Context, c: &mut Ctx, central: egui::Rect, vp: &WorldViewport) {
    let Some(g) = c.session.game.as_ref() else {
        return;
    };
    if g.finished() {
        return;
    }
    let step = g.tutorial;
    let turn = g.turn();
    let (bubble, tips) = if step == TUTORIAL_OFF {
        return;
    } else if step >= TUTORIAL_DONE {
        if turn >= TIPS_UNTIL || c.ui.tip_hidden == Some(turn) {
            return;
        }
        (next_action(c), true)
    } else {
        (step_bubble(c), false)
    };
    let Some(bubble) = bubble else { return };
    let g = c.g();
    let Some(def) = c.content.mentor(&g.profile.mentor).cloned() else {
        return;
    };
    let accent = hex(&def.color);
    let idx = c.mentor_index(&def.id);
    let screen = ctx.content_rect();
    let rect = target_rect(c, bubble.target, vp);
    let t = ctx.input(|i| i.time) as f32;

    // Surbrillance de la cible.
    if let Some(r) = rect {
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("tuto_hl"),
        ));
        let a = if c.settings.reduce_motion {
            230
        } else {
            (150.0 + 100.0 * (t * 4.0).sin()) as u8
        };
        let col = Color32::from_rgba_unmultiplied(250, 214, 92, a);
        painter.rect_stroke(
            r.expand(4.0),
            4.0,
            egui::Stroke::new(3.0, col),
            egui::StrokeKind::Outside,
        );
    }

    // Position de la bulle.
    let side = matches!(bubble.target, Target::Side);
    let width = if side {
        226.0
    } else if tips {
        330.0
    } else {
        360.0
    };
    let (pivot, pos) = match (bubble.target, rect) {
        // À côté d'une fenêtre centrée : en bas à gauche, au-dessus du portrait du mentor.
        (Target::Side, _) => (
            egui::Align2::LEFT_BOTTOM,
            egui::pos2(8.0, central.max.y - 96.0),
        ),
        (_, Some(r)) => {
            // Cible très large (barre des projets) : la bulle s'aligne sur son bord gauche.
            let x0 = if r.width() > width * 1.5 {
                r.min.x + 16.0
            } else {
                r.center().x - width / 2.0
            };
            let x = x0.clamp(8.0, screen.width() - width - 8.0);
            if r.center().y < screen.height() * 0.45 {
                (egui::Align2::LEFT_TOP, egui::pos2(x, r.max.y + 14.0))
            } else {
                (egui::Align2::LEFT_BOTTOM, egui::pos2(x, r.min.y - 14.0))
            }
        }
        _ if tips => (
            egui::Align2::RIGHT_BOTTOM,
            egui::pos2(central.max.x - 12.0, central.max.y - 12.0),
        ),
        _ => (egui::Align2::CENTER_CENTER, central.center()),
    };

    let mut go_next = false;
    let mut skip = false;
    let mut hide = false;
    let mut disable = false;
    egui::Area::new(egui::Id::new("tutorial_bubble"))
        .pivot(pivot)
        .fixed_pos(pos)
        .order(egui::Order::Tooltip)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(Color32::from_rgb(24, 28, 40))
                .stroke(egui::Stroke::new(2.0, accent))
                .corner_radius(6)
                .inner_margin(egui::Margin::same(10))
                .shadow(egui::Shadow {
                    offset: [0, 4],
                    blur: 16,
                    spread: 1,
                    color: Color32::from_black_alpha(160),
                })
                .show(ui, |ui| {
                    ui.set_width(width);
                    ui.horizontal_top(|ui| {
                        mentor_portrait(
                            ui,
                            c.mentors_tex,
                            idx,
                            if tips || side { 40.0 } else { 56.0 },
                        );
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&def.name).font(pixel(14.0)).color(accent));
                                let tag = if tips {
                                    "conseil".to_string()
                                } else {
                                    format!("tutoriel {}/{STEPS}", display_index(step))
                                };
                                muted(ui, tag);
                            });
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&bubble.text).size(14.0).color(TEXT),
                                )
                                .wrap(),
                            );
                            ui.add_space(2.0);
                            ui.horizontal(|ui| {
                                if tips {
                                    if ui.small_button("Masquer ce mois-ci").clicked() {
                                        hide = true;
                                    }
                                    if ui.small_button("Ne plus afficher").clicked() {
                                        disable = true;
                                    }
                                } else {
                                    if bubble.next
                                        && ui
                                            .add(
                                                egui::Button::new(
                                                    RichText::new("Suivant ▸")
                                                        .font(bold(13.0))
                                                        .color(NEAR_BLACK),
                                                )
                                                .fill(YELLOW),
                                            )
                                            .clicked()
                                    {
                                        go_next = true;
                                    }
                                    if ui.small_button("Passer le tutoriel").clicked() {
                                        skip = true;
                                    }
                                }
                            });
                        });
                    });
                });
        });
    let modal_open = c.g().active.is_some()
        || c.ui.room_panel.is_some()
        || c.ui.show_mentors
        || c.ui.show_settings;
    if !tips
        && bubble.next
        && !modal_open
        && !ctx.egui_wants_keyboard_input()
        && ctx.input(|i| i.key_pressed(egui::Key::Enter))
    {
        go_next = true;
    }
    if go_next {
        let n = next_step(c.g().tutorial);
        set_step(c, n);
        c.click();
    }
    if skip {
        set_step(c, TUTORIAL_DONE);
    }
    if hide {
        c.ui.tip_hidden = Some(turn);
    }
    if disable {
        set_step(c, TUTORIAL_OFF);
    }
    let _ = Command::SetTutorial;
}
