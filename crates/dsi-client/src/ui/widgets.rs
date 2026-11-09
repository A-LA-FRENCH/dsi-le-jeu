//! Éléments d'interface réutilisables : jauges, variations, portraits, boutons.

use super::style::*;
use bevy_egui::egui::{self, Color32, RichText, Vec2};
use dsi_core::state::Metric;

/// Couleur d'une valeur de métrique (rouge = danger).
pub fn metric_color(m: Metric, v: f32) -> Color32 {
    let good = if m.inverted() { 100.0 - v } else { v };
    if good < 25.0 {
        RED
    } else if good < 45.0 {
        ORANGE
    } else if good < 70.0 {
        YELLOW
    } else {
        GREEN
    }
}

/// Jauge horizontale pixelisée.
pub fn gauge(
    ui: &mut egui::Ui,
    frac: f32,
    color: Color32,
    width: f32,
    height: f32,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 0.0, NEAR_BLACK);
    let segs = (width / 5.0).floor().max(1.0) as i32;
    let filled = (frac.clamp(0.0, 1.0) * segs as f32).round() as i32;
    let sw = width / segs as f32;
    for i in 0..segs {
        let r = egui::Rect::from_min_size(
            egui::pos2(rect.min.x + i as f32 * sw + 0.5, rect.min.y + 1.0),
            Vec2::new(sw - 1.0, height - 2.0),
        );
        p.rect_filled(
            r,
            0.0,
            if i < filled {
                color
            } else {
                Color32::from_rgb(34, 40, 54)
            },
        );
    }
    resp
}

/// « +3 COMEX » coloré selon que c'est une bonne nouvelle.
pub fn delta_text(m: Metric, d: f32) -> (String, Color32) {
    let good = if m.inverted() { d < 0.0 } else { d > 0.0 };
    let arrow = if d > 0.0 { "▲" } else { "▼" };
    (
        format!(
            "{arrow} {}{:.0} {}",
            if d > 0.0 { "+" } else { "" },
            d,
            m.short()
        ),
        if good { GREEN } else { RED },
    )
}

pub fn deltas(ui: &mut egui::Ui, list: &[(Metric, f32)]) {
    ui.horizontal_wrapped(|ui| {
        for (m, d) in list {
            if d.abs() < 0.5 {
                continue;
            }
            let (t, c) = delta_text(*m, *d);
            egui::Frame::new()
                .fill(CARD)
                .corner_radius(2)
                .inner_margin(egui::Margin::symmetric(6, 2))
                .show(ui, |ui| {
                    ui.label(RichText::new(t).font(bold(13.0)).color(c))
                        .on_hover_text(m.label());
                });
        }
    });
}

/// Indices visibles avant un choix : seulement le sens (▲▼), pas l'ampleur.
pub fn preview(ui: &mut egui::Ui, list: &[(Metric, f32)], hidden: bool) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        for (m, d) in list {
            if *d == 0.0 {
                ui.label(
                    RichText::new(format!("? {}", m.short()))
                        .size(11.5)
                        .color(MUTED),
                )
                .on_hover_text("Effet incertain");
                continue;
            }
            let good = if m.inverted() { *d < 0.0 } else { *d > 0.0 };
            let big = d.abs() >= 6.0;
            let arrow = match (*d > 0.0, big) {
                (true, true) => "▲▲",
                (true, false) => "▲",
                (false, true) => "▼▼",
                (false, false) => "▼",
            };
            ui.label(
                RichText::new(format!("{arrow} {}", m.short()))
                    .size(11.5)
                    .color(if good { GREEN } else { RED }),
            )
            .on_hover_text(m.label());
        }
        if hidden {
            ui.label(
                RichText::new("⏳ suites possibles")
                    .size(11.5)
                    .color(PURPLE),
            )
            .on_hover_text("Cette décision a des conséquences différées ou incertaines.");
        }
    });
}

/// Portrait n° `idx` de l'atlas `ui/portraits.png` (16 × 32 px).
pub fn portrait(ui: &mut egui::Ui, tex: Option<egui::TextureId>, idx: u32, size: f32) {
    let Some(id) = tex else {
        ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
        return;
    };
    let n = 32.0;
    let uv = egui::Rect::from_min_max(
        egui::pos2(idx as f32 / n, 0.0),
        egui::pos2((idx as f32 + 1.0) / n, 1.0),
    );
    ui.add(
        egui::Image::from_texture(egui::load::SizedTexture::new(id, Vec2::splat(size)))
            .uv(uv)
            .texture_options(egui::TextureOptions::NEAREST),
    );
}

pub fn big_button(ui: &mut egui::Ui, text: &str, enabled: bool, width: f32) -> egui::Response {
    let b = egui::Button::new(RichText::new(text).font(pixel(20.0)).color(if enabled {
        NEAR_BLACK
    } else {
        MUTED
    }))
    .min_size(egui::vec2(width, 38.0))
    .fill(if enabled { YELLOW } else { CARD });
    ui.add_enabled(enabled, b)
}

pub fn ghost_button(ui: &mut egui::Ui, text: &str, width: f32) -> egui::Response {
    ui.add(egui::Button::new(RichText::new(text).size(14.5)).min_size(egui::vec2(width, 30.0)))
}

pub fn section(ui: &mut egui::Ui, text: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(text).font(pixel(13.0)).color(MUTED));
}

pub fn muted(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.label(RichText::new(text.into()).size(12.5).color(MUTED));
}

/// Assombrit tout l'écran derrière une fenêtre modale.
pub fn dim(ctx: &egui::Context, alpha: u8) {
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Middle,
        egui::Id::new("dim"),
    ));
    painter.rect_filled(ctx.content_rect(), 0.0, Color32::from_black_alpha(alpha));
}

/// Fenêtre modale centrée, avec défilement si besoin.
pub fn modal(
    ctx: &egui::Context,
    id: &str,
    width: f32,
    accent: Color32,
    add: impl FnOnce(&mut egui::Ui),
) {
    dim(ctx, 150);
    egui::Area::new(egui::Id::new(id))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            modal_frame(accent).show(ui, |ui| {
                let cr = ctx.content_rect();
                ui.set_width(width.min(cr.width() - 60.0).max(260.0));
                let limit = (cr.height() - 90.0).max(200.0);
                ui.set_max_height(limit);
                egui::ScrollArea::vertical().max_height(limit).show(ui, add);
            });
        });
}

/// Bandeau de type de popup (EVENT, INCIDENT, VENDOR MEETING…).
pub fn kind_banner(ui: &mut egui::Ui, text: &str, color: Color32) {
    egui::Frame::new()
        .fill(color)
        .corner_radius(2)
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).font(pixel(12.0)).color(NEAR_BLACK));
        });
}

pub fn wrap(ui: &mut egui::Ui, text: impl Into<RichText>) {
    let t: RichText = text.into();
    ui.add(egui::Label::new(t).wrap());
}

/// Portrait d'un mentor (`ui/mentors.png`, 3 × 40 px).
pub fn mentor_portrait(
    ui: &mut egui::Ui,
    tex: Option<egui::TextureId>,
    idx: u32,
    size: f32,
) -> egui::Response {
    let Some(id) = tex else {
        return ui
            .allocate_exact_size(Vec2::splat(size), egui::Sense::click())
            .1;
    };
    let n = 3.0;
    let uv = egui::Rect::from_min_max(
        egui::pos2(idx as f32 / n, 0.0),
        egui::pos2((idx as f32 + 1.0) / n, 1.0),
    );
    ui.add(
        egui::Image::from_texture(egui::load::SizedTexture::new(id, Vec2::splat(size)))
            .uv(uv)
            .texture_options(egui::TextureOptions::NEAREST)
            .sense(egui::Sense::click()),
    )
}

/// Mémorise le rectangle d'un élément pour que le tutoriel puisse le désigner.
pub fn mark(
    marks: &mut std::collections::HashMap<&'static str, egui::Rect>,
    key: &'static str,
    r: egui::Rect,
) {
    marks.insert(key, r);
}

/// Postes du budget mensuel : (libellé, couleur, montant).
pub fn budget_items(b: &dsi_core::economy::Breakdown) -> [(&'static str, Color32, f32); 5] {
    [
        ("Salaires", BLUE, b.salaries),
        ("Logiciels", PURPLE, b.software),
        ("ESN", ORANGE, b.esn),
        ("Cloud / IA", CYAN, b.cloud_ai),
        ("Projets", YELLOW, b.projects),
    ]
}

/// Détail texte du budget mensuel (infobulle).
pub fn budget_tooltip(b: &dsi_core::economy::Breakdown) -> String {
    let mut t = format!(
        "Budget mensuel alloué par le DAF : {:.2} pt\n",
        b.allocation
    );
    let pct = |v: f32| {
        if b.allocation > 0.0 {
            v / b.allocation * 100.0
        } else {
            0.0
        }
    };
    for (k, _, v) in budget_items(b) {
        let detail = match k {
            "Salaires" => " (équipe historique + recrues)",
            "Logiciels" => " (licences, abonnements, contrats fournisseurs)",
            "ESN" => " (régie + renforts ESN sur les projets)",
            "Cloud / IA" => " (facture cloud + tokens / GPU)",
            _ => " (projets en cours)",
        };
        t.push_str(&format!("\n{k} : {v:.2} pt — {:.0} %{detail}", pct(v)));
    }
    t.push_str(&format!(
        "\n\nSolde du mois : {:+.2} pt, ajouté à la jauge BUDGET au passage du mois.\nPour réduire les salaires : onglet Équipe → Licencier.",
        b.balance
    ));
    t
}

/// Barre empilée : chaque poste en proportion du budget mensuel ; le reste est le solde.
pub fn budget_bar(
    ui: &mut egui::Ui,
    b: &dsi_core::economy::Breakdown,
    width: f32,
    height: f32,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 0.0, Color32::from_rgb(34, 40, 54));
    let total = b.allocation.max(b.spent()).max(0.01);
    let mut x = rect.min.x;
    for (_, col, v) in budget_items(b) {
        let w = v.max(0.0) / total * width;
        if w > 0.5 {
            p.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(x, rect.min.y),
                    egui::pos2((x + w - 1.0).max(x + 0.5), rect.max.y),
                ),
                0.0,
                col,
            );
        }
        x += w;
    }
    // Trait de l'allocation : au-delà, on dépense plus que ce qu'on reçoit.
    let ax = rect.min.x + b.allocation / total * width;
    if b.balance < 0.0 {
        p.rect_filled(
            egui::Rect::from_min_max(egui::pos2(ax, rect.min.y), rect.max),
            0.0,
            RED.gamma_multiply(0.35),
        );
    }
    p.line_segment(
        [
            egui::pos2(ax, rect.min.y - 2.0),
            egui::pos2(ax, rect.max.y + 2.0),
        ],
        egui::Stroke::new(2.0, TEXT),
    );
    resp
}

/// Légende compacte d'un poste du budget.
pub fn budget_chip(ui: &mut egui::Ui, label: &str, col: Color32, v: f32) {
    ui.spacing_mut().item_spacing.x = 3.0;
    let (r, _) = ui.allocate_exact_size(Vec2::new(8.0, 8.0), egui::Sense::hover());
    ui.painter().rect_filled(r, 0.0, col);
    ui.label(
        RichText::new(format!("{label} {v:.1}"))
            .size(12.0)
            .color(LIGHT),
    );
    ui.add_space(4.0);
}
