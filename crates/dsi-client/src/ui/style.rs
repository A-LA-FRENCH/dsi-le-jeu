//! Identité visuelle : « corporate dystopia » — bleu nuit, gris open space, jaune post-it.

use bevy_egui::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Stroke,
    TextStyle,
};
use std::sync::Arc;

pub const NEAR_BLACK: Color32 = Color32::from_rgb(13, 16, 23);
pub const PANEL: Color32 = Color32::from_rgb(22, 27, 38);
pub const CARD: Color32 = Color32::from_rgb(32, 38, 52);
pub const CARD_2: Color32 = Color32::from_rgb(44, 52, 70);
pub const LINE: Color32 = Color32::from_rgb(58, 68, 90);
pub const TEXT: Color32 = Color32::from_rgb(236, 236, 228);
pub const LIGHT: Color32 = Color32::from_rgb(196, 202, 214);
pub const MUTED: Color32 = Color32::from_rgb(140, 148, 166);
pub const YELLOW: Color32 = Color32::from_rgb(250, 214, 92);
pub const GREEN: Color32 = Color32::from_rgb(124, 230, 110);
pub const RED: Color32 = Color32::from_rgb(238, 98, 98);
pub const ORANGE: Color32 = Color32::from_rgb(246, 162, 60);
pub const BLUE: Color32 = Color32::from_rgb(110, 160, 240);
pub const PURPLE: Color32 = Color32::from_rgb(150, 120, 230);
pub const CYAN: Color32 = Color32::from_rgb(110, 220, 225);

const PIXEL: &[u8] = include_bytes!("../../../../assets/fonts/PixelifySans-SemiBold.ttf");
const BODY: &[u8] = include_bytes!("../../../../assets/fonts/IBMPlexSans-Regular.ttf");
const BODY_BOLD: &[u8] = include_bytes!("../../../../assets/fonts/IBMPlexSans-SemiBold.ttf");

/// Police pixel, calée sur les tailles où sa grille reste nette (sinon le « C » se referme).
pub fn pixel(size: f32) -> FontId {
    const CLEAN: [f32; 12] = [
        10.0, 14.0, 16.0, 20.0, 24.0, 28.0, 30.0, 36.0, 42.0, 48.0, 56.0, 64.0,
    ];
    let snapped = CLEAN
        .iter()
        .copied()
        .min_by(|a, b| (a - size).abs().total_cmp(&(b - size).abs()))
        .unwrap_or(size);
    FontId::new(snapped, FontFamily::Name("pixel".into()))
}

pub fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("bold".into()))
}

pub fn install(ctx: &egui::Context, high_contrast: bool) {
    let mut fonts = FontDefinitions::default();
    fonts
        .font_data
        .insert("pixel".into(), Arc::new(FontData::from_static(PIXEL)));
    fonts
        .font_data
        .insert("body".into(), Arc::new(FontData::from_static(BODY)));
    fonts.font_data.insert(
        "body_bold".into(),
        Arc::new(FontData::from_static(BODY_BOLD)),
    );
    // Secours : polices d'egui (émojis, icônes) puis Hack pour les symboles géométriques (▲▼■●→).
    let mut fallbacks: Vec<String> = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    if let Some(mono) = fonts.families.get(&FontFamily::Monospace) {
        for f in mono {
            if !fallbacks.contains(f) {
                fallbacks.push(f.clone());
            }
        }
    }
    let mut prop = vec!["body".to_string()];
    prop.extend(fallbacks.iter().cloned());
    fonts.families.insert(FontFamily::Proportional, prop);
    let mut px = vec!["pixel".to_string(), "body".to_string()];
    px.extend(fallbacks.iter().cloned());
    fonts.families.insert(FontFamily::Name("pixel".into()), px);
    let mut b = vec!["body_bold".to_string()];
    b.extend(fallbacks.iter().cloned());
    fonts.families.insert(FontFamily::Name("bold".into()), b);
    ctx.set_fonts(fonts);
    apply_visuals(ctx, high_contrast);
}

pub fn apply_visuals(ctx: &egui::Context, high_contrast: bool) {
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Heading, pixel(22.0)),
            (TextStyle::Body, FontId::proportional(15.0)),
            (TextStyle::Button, FontId::proportional(15.0)),
            (TextStyle::Small, FontId::proportional(12.5)),
            (TextStyle::Monospace, FontId::monospace(13.0)),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 5.0);
        style.spacing.interact_size.y = 26.0;
        let v = &mut style.visuals;
        v.dark_mode = true;
        v.override_text_color = Some(if high_contrast { Color32::WHITE } else { TEXT });
        v.panel_fill = PANEL;
        v.window_fill = Color32::from_rgb(20, 25, 35);
        v.extreme_bg_color = NEAR_BLACK;
        v.faint_bg_color = CARD;
        v.code_bg_color = NEAR_BLACK;
        v.hyperlink_color = YELLOW;
        v.selection.bg_fill = Color32::from_rgb(70, 64, 30);
        v.selection.stroke = Stroke::new(1.0, YELLOW);
        v.window_stroke = Stroke::new(2.0, LINE);
        v.window_corner_radius = CornerRadius::same(3);
        v.menu_corner_radius = CornerRadius::same(3);
        v.warn_fg_color = ORANGE;
        v.error_fg_color = RED;
        let r = CornerRadius::same(2);
        let hc = if high_contrast { 1.5 } else { 1.0 };
        v.widgets.noninteractive.bg_fill = PANEL;
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, LIGHT);
        v.widgets.noninteractive.corner_radius = r;
        v.widgets.inactive.bg_fill = CARD_2;
        v.widgets.inactive.weak_bg_fill = CARD_2;
        v.widgets.inactive.bg_stroke = Stroke::new(1.0 * hc, LINE);
        v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
        v.widgets.inactive.corner_radius = r;
        v.widgets.hovered.bg_fill = Color32::from_rgb(70, 64, 30);
        v.widgets.hovered.weak_bg_fill = Color32::from_rgb(66, 62, 36);
        v.widgets.hovered.bg_stroke = Stroke::new(1.0 * hc, YELLOW);
        v.widgets.hovered.fg_stroke = Stroke::new(1.5, TEXT);
        v.widgets.hovered.corner_radius = r;
        v.widgets.active.bg_fill = Color32::from_rgb(110, 96, 30);
        v.widgets.active.weak_bg_fill = Color32::from_rgb(90, 80, 30);
        v.widgets.active.bg_stroke = Stroke::new(1.0, YELLOW);
        v.widgets.active.fg_stroke = Stroke::new(2.0, TEXT);
        v.widgets.active.corner_radius = r;
        v.widgets.open.bg_fill = CARD_2;
        v.widgets.open.weak_bg_fill = CARD_2;
        v.widgets.open.corner_radius = r;
    });
}

pub fn panel_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(PANEL)
        .inner_margin(Margin::same(10))
        .stroke(Stroke::new(1.0, Color32::from_rgb(30, 36, 50)))
}

pub fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(CARD)
        .inner_margin(Margin::same(8))
        .corner_radius(CornerRadius::same(3))
        .stroke(Stroke::new(1.0, LINE))
}

pub fn modal_frame(accent: Color32) -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_rgb(20, 25, 35))
        .inner_margin(Margin::same(18))
        .corner_radius(CornerRadius::same(4))
        .stroke(Stroke::new(2.0, accent))
        .shadow(egui::Shadow {
            offset: [0, 6],
            blur: 24,
            spread: 2,
            color: Color32::from_black_alpha(170),
        })
}

pub fn tone_color(t: dsi_core::state::Tone) -> Color32 {
    use dsi_core::state::Tone;
    match t {
        Tone::Info => LIGHT,
        Tone::Good => GREEN,
        Tone::Bad => RED,
        Tone::Money => YELLOW,
        Tone::Story => CYAN,
    }
}

pub fn kind_color(k: dsi_core::events::EventKind) -> Color32 {
    use dsi_core::events::EventKind::*;
    match k {
        Event => YELLOW,
        Vendor => BLUE,
        Comex => ORANGE,
        Incident => RED,
        Project => CYAN,
        Recruitment => PURPLE,
        Contract => BLUE,
        Architecture => GREEN,
    }
}

pub fn hex(s: &str) -> Color32 {
    let s = s.trim_start_matches('#');
    if s.len() != 6 {
        return YELLOW;
    }
    let v = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).unwrap_or(200);
    Color32::from_rgb(v(0), v(2), v(4))
}
