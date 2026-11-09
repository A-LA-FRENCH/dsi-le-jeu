//! Interface (egui) : écran titre, création du ou de la DSI, tableau de bord, popups,
//! mini-jeux, réglages et mode debug. Le monde (le siège) est dessiné par Bevy dans
//! la zone centrale laissée libre.

mod ambition;
mod debug;
mod hud;
mod mentor;
mod menu;
mod minigames;
mod panel;
mod rooms;
mod saves;
mod scenes;
pub mod style;
mod tutorial;
pub mod widgets;

use crate::audio::Sfx;
use crate::plateau::{PlateauHover, PlateauStats};
use crate::sim::{GameContent, Session, Settings, Store};
use crate::world::{RoomPick, Sprites, WorldView, WorldViewport};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, EguiTextureHandle, egui};
use dsi_core::state::Room;
use dsi_core::{Command, Content, GameState};
use style::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Screen {
    #[default]
    Title,
    Create,
    Mentor,
    Playing,
}

/// Premier slot des avatars de DSI dans `ui/portraits.png`.
pub const AVATAR_SLOT: u32 = 16;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Tab {
    #[default]
    Inbox,
    Metrics,
    Contracts,
    Tech,
    Team,
}

#[derive(Resource)]
pub struct UiState {
    pub screen: Screen,
    pub tab: Tab,
    pub room_panel: Option<Room>,
    pub show_settings: bool,
    pub show_credits: bool,
    pub show_help: bool,
    pub confirm_reset: bool,
    pub show_debug: bool,
    pub toasts: Vec<(String, bool, f32)>,
    pub initialised: bool,
    pub fonts_ready: bool,
    pub applied_contrast: bool,
    // Création du ou de la DSI.
    pub name: String,
    pub female: bool,
    pub company: String,
    pub background: String,
    pub seed_text: String,
    pub avatar: u32,
    pub mentor: String,
    pub tutorial: bool,
    pub show_mentors: bool,
    /// Fenêtre de choix du cap de la saison 2.
    pub show_ambition: bool,
    /// Fenêtre des sauvegardes manuelles.
    pub show_saves: bool,
    pub slots: Vec<Option<Result<dsi_core::save::SlotInfo, String>>>,
    pub slot_pending: Option<saves::Pending>,
    /// Recrue dont le licenciement attend confirmation (onglet Équipe).
    pub fire_pending: Option<usize>,
    /// Solution choisie par projet dans la salle projet.
    pub project_option: std::collections::HashMap<String, String>,
    /// Destinataire choisi pour le partage du résultat.
    pub share_audience: Option<dsi_core::share::Audience>,
    /// Défi reçu par lien (`?defi=…`), tant qu'il n'a pas été écarté.
    pub incoming: Option<dsi_core::share::ShareCard>,
    pub incoming_checked: bool,
    /// Page du chapitre en cours (scènes de dialogue longues) : (chapitre, page).
    pub chapter_page: (usize, usize),
    pub ambition_choice: Option<String>,
    /// Rectangles d'éléments d'interface que le tutoriel peut désigner.
    pub marks: std::collections::HashMap<&'static str, egui::Rect>,
    /// Mois où le joueur a masqué la bulle de conseil.
    pub tip_hidden: Option<u32>,
    // Debug.
    pub debug_filter: String,
    pub debug_json: bool,
    pub project_filter: String,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            screen: Screen::Title,
            tab: Tab::Inbox,
            room_panel: None,
            show_settings: false,
            show_credits: false,
            show_help: false,
            confirm_reset: false,
            show_debug: false,
            toasts: Vec::new(),
            initialised: false,
            fonts_ready: false,
            applied_contrast: false,
            name: "Camille Durand".into(),
            female: true,
            company: "Groupe Duval-Montaigne".into(),
            background: "ex_dev".into(),
            seed_text: String::new(),
            avatar: 0,
            mentor: "mehdi".into(),
            tutorial: true,
            show_mentors: false,
            show_ambition: false,
            show_saves: false,
            slots: Vec::new(),
            slot_pending: None,
            fire_pending: None,
            project_option: std::collections::HashMap::new(),
            share_audience: None,
            incoming: None,
            incoming_checked: false,
            chapter_page: (usize::MAX, 0),
            ambition_choice: None,
            marks: Default::default(),
            tip_hidden: None,
            debug_filter: String::new(),
            debug_json: false,
            project_filter: String::new(),
        }
    }
}

impl UiState {
    /// Ferme les fenêtres liées à la partie précédente après un chargement.
    pub fn reset_for_load(&mut self) {
        self.room_panel = None;
        self.show_settings = false;
        self.show_mentors = false;
        self.show_ambition = false;
        self.show_help = false;
        self.fire_pending = None;
        self.share_audience = None;
    }

    pub fn toast(&mut self, msg: impl Into<String>, good: bool) {
        let m = msg.into();
        let ttl = (3.5 + m.chars().count() as f32 / 30.0).min(9.0);
        self.toasts.push((m, good, ttl));
    }
}

#[derive(Resource, Default)]
pub struct UiTextures {
    pub portraits: Option<egui::TextureId>,
    pub mentors: Option<egui::TextureId>,
}

/// Tout ce dont les écrans ont besoin.
pub struct Ctx<'a> {
    pub session: &'a mut Session,
    pub content: &'a Content,
    pub ui: &'a mut UiState,
    pub sfx: &'a mut Sfx,
    pub settings: &'a mut Settings,
    pub store: &'a mut Store,
    pub tex: Option<egui::TextureId>,
    pub mentors_tex: Option<egui::TextureId>,
    pub view: &'a mut WorldView,
}

impl Ctx<'_> {
    /// Exécute une commande ; affiche l'erreur éventuelle dans une notification.
    pub fn run(&mut self, cmd: Command) -> bool {
        let team_before = self
            .session
            .game
            .as_ref()
            .map(|g| g.team.len())
            .unwrap_or(0);
        match self.session.run(self.content, self.sfx, cmd) {
            Ok(()) => {
                if let Some(g) = &self.session.game
                    && g.team.len() > team_before
                    && let Some(h) = g.team.last()
                {
                    let msg = format!(
                        "{} s'installe sur le plateau DSI (touche V pour le voir).",
                        h.name
                    );
                    self.ui.toast(msg, true);
                }
                true
            }
            Err(e) => {
                self.ui.toast(e, false);
                false
            }
        }
    }

    pub fn g(&self) -> &GameState {
        self.session.game.as_ref().expect("partie en cours")
    }

    pub fn click(&mut self) {
        self.sfx.push(crate::audio::Cue::Click);
    }

    /// Index de portrait d'un personnage (dialogues.json), ou du ou de la DSI.
    pub fn portrait_of(&self, speaker: &str) -> u32 {
        if speaker == "player" {
            let Some(g) = self.session.game.as_ref() else {
                return 14;
            };
            return match g.profile.avatar {
                Some(a) => AVATAR_SLOT + a,
                None if g.profile.female => 14,
                None => 15,
            };
        }
        self.content
            .character(speaker)
            .map(|c| c.portrait)
            .unwrap_or(13)
    }
}

impl Ctx<'_> {
    /// Identifiant du mentor désigné par `speaker` (« mentor » = mentor principal, « mentor:jb »).
    pub fn mentor_id(&self, speaker: &str) -> Option<String> {
        if speaker == "mentor" {
            return self
                .session
                .game
                .as_ref()
                .map(|g| g.profile.mentor.clone())
                .filter(|m| !m.is_empty());
        }
        speaker.strip_prefix("mentor:").map(|s| s.to_string())
    }

    pub fn mentor_index(&self, id: &str) -> u32 {
        self.content
            .mentors
            .iter()
            .position(|m| m.id == id)
            .unwrap_or(0) as u32
    }

    /// Dessine le portrait d'un personnage, d'un mentor ou du ou de la DSI.
    pub fn face(&self, ui: &mut egui::Ui, speaker: &str, size: f32) {
        if let Some(id) = self.mentor_id(speaker) {
            widgets::mentor_portrait(ui, self.mentors_tex, self.mentor_index(&id), size);
        } else {
            widgets::portrait(ui, self.tex, self.portrait_of(speaker), size);
        }
    }

    /// Nom et couleur d'un interlocuteur.
    pub fn speaker_name(&self, speaker: &str) -> Option<(String, egui::Color32)> {
        if let Some(id) = self.mentor_id(speaker) {
            return self
                .content
                .mentor(&id)
                .map(|m| (format!("{} — ton mentor", m.name), hex(&m.color)));
        }
        self.content
            .character(speaker)
            .map(|ch| (ch.name.clone(), hex(&ch.color)))
    }
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiState>()
            .init_resource::<UiTextures>()
            .add_systems(
                EguiPrimaryContextPass,
                (setup_style, register_textures, ui_root).chain(),
            );
    }
}

fn setup_style(
    mut contexts: EguiContexts,
    mut ui: ResMut<UiState>,
    settings: Res<Settings>,
    session: Res<Session>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    if !ui.initialised {
        style::install(ctx, settings.high_contrast);
        ui.initialised = true;
        ui.applied_contrast = settings.high_contrast;
        ui.show_debug = session.params.debug;
        if let Some(seed) = session.params.seed {
            ui.seed_text = seed.to_string();
        }
    } else if !ui.fonts_ready {
        ui.fonts_ready = true;
    }
    if ui.applied_contrast != settings.high_contrast {
        style::apply_visuals(ctx, settings.high_contrast);
        ui.applied_contrast = settings.high_contrast;
    }
    let z = settings.ui_scale.clamp(0.8, 1.5);
    if (ctx.zoom_factor() - z).abs() > 0.001 {
        ctx.set_zoom_factor(z);
    }
    Ok(())
}

fn register_textures(
    mut contexts: EguiContexts,
    sprites: Option<Res<Sprites>>,
    mut tex: ResMut<UiTextures>,
) {
    let Some(s) = sprites else { return };
    if tex.portraits.is_none() {
        tex.portraits = Some(contexts.add_image(EguiTextureHandle::Strong(s.portraits.clone())));
        tex.mentors = Some(contexts.add_image(EguiTextureHandle::Strong(s.mentors.clone())));
    }
}

#[derive(bevy::ecs::system::SystemParam)]
pub struct UiRes<'w> {
    session: ResMut<'w, Session>,
    content: Res<'w, GameContent>,
    ui: ResMut<'w, UiState>,
    sfx: ResMut<'w, Sfx>,
    settings: ResMut<'w, Settings>,
    store: ResMut<'w, Store>,
    tex: Res<'w, UiTextures>,
    vp: ResMut<'w, WorldViewport>,
    pick: ResMut<'w, RoomPick>,
    time: Res<'w, Time>,
    view: ResMut<'w, WorldView>,
    person: Res<'w, PlateauHover>,
    stats: Res<'w, PlateauStats>,
}

fn ui_root(mut contexts: EguiContexts, mut r: UiRes) -> Result {
    if !r.ui.fonts_ready {
        return Ok(());
    }
    let ctx = contexts.ctx_mut()?.clone();
    let dt = r.time.delta_secs();
    let UiRes {
        session,
        content,
        ui,
        sfx,
        settings,
        store,
        tex,
        vp,
        pick,
        view,
        person,
        stats,
        ..
    } = &mut r;
    let mut c = Ctx {
        session,
        content: &content.0,
        ui,
        sfx,
        settings,
        store,
        tex: tex.portraits,
        mentors_tex: tex.mentors,
        view,
    };
    c.ui.marks.clear();

    if c.session.game.is_none() && c.ui.screen == Screen::Playing {
        c.ui.screen = Screen::Title;
    }
    if c.ui.screen != Screen::Playing {
        c.view.plateau = false;
    }
    let mut root = egui::Ui::new(
        ctx.clone(),
        "root".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.content_rect()),
    );

    let central = match c.ui.screen {
        Screen::Title | Screen::Create | Screen::Mentor => {
            match c.ui.screen {
                Screen::Title => {
                    if !c.ui.show_saves {
                        menu::title(&ctx, &mut c)
                    }
                }
                Screen::Create => menu::create(&ctx, &mut c),
                _ => menu::mentor_choice(&ctx, &mut c),
            }
            ctx.content_rect()
        }
        Screen::Playing => {
            hud::top_bar(&mut root, &mut c);
            hud::bottom_bar(&mut root, &mut c);
            let narrow = ctx.content_rect().width() < 1150.0;
            egui::Panel::right("right_panel")
                .resizable(true)
                .default_size(if narrow { 320.0 } else { 360.0 })
                .min_size(280.0)
                .max_size(560.0)
                .frame(style::panel_frame())
                .show(&mut root, |ui| panel::show(ui, &mut c));
            let central = root.available_rect_before_wrap();
            // Clic sur une salle du siège.
            if let Some(room) = pick.clicked.take()
                && c.session.game.as_ref().is_some_and(|g| g.active.is_none())
            {
                c.click();
                let idx = c
                    .g()
                    .inbox
                    .iter()
                    .position(|i| dsi_core::game::inbox_room(c.content, i) == room);
                match idx {
                    Some(i) => {
                        c.run(Command::Open(i));
                    }
                    None => c.ui.room_panel = Some(room),
                }
            }
            hud::world_overlay(
                &ctx,
                &mut c,
                central,
                vp,
                pick.hover,
                person.0.as_ref(),
                stats,
            );
            mentor::widget(&ctx, &mut c, central);
            hud::next_button(&ctx, &mut c, central);
            scenes::show(&ctx, &mut c);
            if c.ui.show_ambition {
                ambition::picker(&ctx, &mut c);
            }
            if c.session
                .game
                .as_ref()
                .is_some_and(|g| g.active.is_none() && !g.finished())
            {
                rooms::show(&ctx, &mut c);
                if c.ui.show_mentors {
                    mentor::panel(&ctx, &mut c);
                }
            }
            hud::shortcuts(&ctx, &mut c);
            tutorial::show(&ctx, &mut c, central, vp);
            central
        }
    };
    pick.clicked = None;
    if c.ui.show_settings {
        menu::settings(&ctx, &mut c);
    }
    saves::show(&ctx, &mut c);
    if c.ui.show_credits {
        menu::credits(&ctx, &mut c);
    }
    if c.ui.show_help {
        menu::help(&ctx, &mut c);
    }
    if c.ui.show_debug && c.session.game.is_some() {
        debug::window(&ctx, &mut c);
    }
    toasts(&ctx, c.ui, dt);
    if let Some(e) = c.session.last_error.take() {
        c.ui.toast(e, false);
    }

    vp.min = Vec2::new(central.min.x, central.min.y);
    vp.max = Vec2::new(central.max.x, central.max.y);
    let pointer = ctx.pointer_hover_pos();
    let modal_open = c
        .session
        .game
        .as_ref()
        .is_some_and(|g| g.active.is_some() || g.finished())
        || c.ui.room_panel.is_some()
        || c.ui.show_mentors
        || c.ui.show_settings;
    vp.pointer_over_ui = modal_open
        || ctx.egui_wants_pointer_input()
        || pointer.is_some_and(|p| !central.contains(p));
    Ok(())
}

fn toasts(ctx: &egui::Context, ui: &mut UiState, dt: f32) {
    for t in &mut ui.toasts {
        t.2 -= dt;
    }
    ui.toasts.retain(|t| t.2 > 0.0);
    if ui.toasts.len() > 4 {
        let n = ui.toasts.len() - 4;
        ui.toasts.drain(0..n);
    }
    if ui.toasts.is_empty() {
        return;
    }
    egui::Area::new("toasts".into())
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(-60.0, 70.0))
        .order(egui::Order::Tooltip)
        .interactable(false)
        .show(ctx, |area| {
            for (msg, good, ttl) in &ui.toasts {
                let a = (ttl.min(1.0) * 240.0) as u8;
                egui::Frame::new()
                    .fill(egui::Color32::from_rgba_unmultiplied(20, 25, 35, a))
                    .stroke(egui::Stroke::new(1.5, if *good { GREEN } else { RED }))
                    .inner_margin(egui::Margin::symmetric(12, 6))
                    .corner_radius(3)
                    .show(area, |ui| {
                        ui.set_max_width(460.0);
                        ui.add(
                            egui::Label::new(egui::RichText::new(msg).size(14.0).color(TEXT))
                                .wrap(),
                        );
                    });
                area.add_space(4.0);
            }
        });
}
