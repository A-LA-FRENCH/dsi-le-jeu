//! Le monde : le siège en coupe (pixel art), les salariés qui circulent, les bulles
//! d'alerte au-dessus des salles, les voyants du datacenter, et la sélection des salles
//! à la souris. La caméra dessine uniquement dans la zone laissée libre par l'interface.

use crate::sim::{GameContent, Session, Settings};
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{CameraOutputMode, ClearColorConfig, Viewport};
use bevy::prelude::*;
use bevy::render::render_resource::BlendState;
use bevy::window::PrimaryWindow;
use dsi_core::events::EventKind;
use dsi_core::state::{Inbox, Room};

// Géométrie du sprite `building.png` (voir tools/art/gen_art.py).
pub const BW: f32 = 316.0;
pub const BH: f32 = 162.0;
pub const RW: f32 = 56.0;
pub const RH: f32 = 40.0;
const WALL: f32 = 2.0;
const MARGIN_X: f32 = 12.0;
const ROOF: f32 = 24.0;

/// Coin haut-gauche de l'intérieur d'une salle, en pixels de l'image.
pub fn room_px(room: Room) -> Vec2 {
    let (c, f) = room.grid();
    Vec2::new(
        MARGIN_X + WALL + c as f32 * (RW + WALL),
        ROOF + WALL + f as f32 * (RH + WALL),
    )
}

/// Pixel de l'image → monde (origine au centre de l'image, y vers le haut).
pub fn px_to_world(p: Vec2) -> Vec2 {
    Vec2::new(p.x - BW / 2.0, BH / 2.0 - p.y)
}

pub fn room_world_rect(room: Room) -> Rect {
    let p = room_px(room);
    let a = px_to_world(p);
    let b = px_to_world(p + Vec2::new(RW, RH));
    Rect::from_corners(a, b)
}

/// Position de l'enseigne sur le toit (centre, monde).
pub fn sign_world() -> Vec2 {
    px_to_world(Vec2::new(
        MARGIN_X + 70.0 + (BW - 2.0 * MARGIN_X - 140.0) / 2.0,
        11.0,
    ))
}

#[derive(Resource)]
pub struct Sprites {
    pub sky: Handle<Image>,
    pub building: Handle<Image>,
    pub people: Handle<Image>,
    pub fx: Handle<Image>,
    pub portraits: Handle<Image>,
    pub mentors: Handle<Image>,
}

#[derive(Component)]
pub struct MainCamera;

/// Zone centrale de l'écran laissée libre par l'interface (pixels logiques).
#[derive(Resource, Clone, Copy)]
pub struct WorldViewport {
    pub min: Vec2,
    pub max: Vec2,
    pub pointer_over_ui: bool,
    pub zoom: f32,
    /// Centre de la caméra dans le monde.
    pub cam: Vec2,
}

impl Default for WorldViewport {
    fn default() -> Self {
        Self {
            min: Vec2::ZERO,
            max: Vec2::new(1280.0, 720.0),
            pointer_over_ui: false,
            zoom: 3.0,
            cam: Vec2::new(0.0, 4.0),
        }
    }
}

/// Vue affichée : le siège en coupe, ou le plateau DSI isométrique.
#[derive(Resource, Default)]
pub struct WorldView {
    pub plateau: bool,
}

/// Salle survolée et salle cliquée (consommée par l'interface).
#[derive(Resource, Default)]
pub struct RoomPick {
    pub hover: Option<Room>,
    pub clicked: Option<Room>,
}

#[derive(Component)]
struct Walker {
    floor: u32,
    x: f32,
    target: f32,
    speed: f32,
    pause: f32,
    anim: f32,
    look: u32,
}

#[derive(Component)]
struct Bubble(Room);

#[derive(Component)]
struct Glow(Room);

#[derive(Component)]
struct HoverBox;

#[derive(Component)]
struct Led(f32);

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldViewport>()
            .init_resource::<WorldView>()
            .init_resource::<RoomPick>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (apply_camera, pick, walkers, bubbles, glows, leds).chain(),
            );
    }
}

fn setup(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut egui_settings: ResMut<bevy_egui::EguiGlobalSettings>,
) {
    let sprites = Sprites {
        sky: server.load("sprites/sky.png"),
        building: server.load("sprites/building.png"),
        people: server.load("sprites/people.png"),
        fx: server.load("sprites/fx.png"),
        portraits: server.load("ui/portraits.png"),
        mentors: server.load("ui/mentors.png"),
    };
    // Caméra du monde.
    commands.spawn((Camera2d, MainCamera, Transform::from_xyz(0.0, 0.0, 100.0)));
    // Caméra d'interface : toute la fenêtre, egui seulement, par-dessus.
    egui_settings.auto_create_primary_context = false;
    commands.spawn((
        bevy_egui::PrimaryEguiContext,
        Camera2d,
        RenderLayers::none(),
        Camera {
            order: 1,
            output_mode: CameraOutputMode::Write {
                blend_state: Some(BlendState::ALPHA_BLENDING),
                clear_color: ClearColorConfig::None,
            },
            clear_color: ClearColorConfig::Custom(Color::NONE),
            ..default()
        },
    ));
    // Ciel : la skyline s'appuie sur le trottoir, derrière le siège.
    commands.spawn((
        Sprite {
            image: sprites.sky.clone(),
            ..default()
        },
        Transform::from_xyz(0.0, -BH / 2.0 + 4.0 + 270.0 * 1.5 / 2.0, 0.0)
            .with_scale(Vec3::splat(1.5)),
    ));
    // Sol sous la rue.
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(30, 32, 38), Vec2::new(2000.0, 400.0)),
        Transform::from_xyz(0.0, -BH / 2.0 - 200.0 + 1.0, 0.5),
    ));
    commands.spawn((
        Sprite {
            image: sprites.building.clone(),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 1.0),
    ));
    // Survol d'une salle.
    commands.spawn((
        Sprite::from_color(Color::srgba(1.0, 1.0, 0.8, 0.12), Vec2::new(RW, RH)),
        Transform::from_xyz(0.0, 0.0, 3.0),
        Visibility::Hidden,
        HoverBox,
    ));
    for room in Room::ALL {
        let r = room_world_rect(room);
        commands.spawn((
            Sprite::from_color(Color::srgba(1.0, 0.2, 0.2, 0.0), Vec2::new(RW, RH)),
            Transform::from_translation(r.center().extend(2.0)),
            Glow(room),
        ));
        commands.spawn((
            Sprite {
                image: sprites.fx.clone(),
                rect: Some(Rect::new(0.0, 0.0, 13.0, 14.0)),
                ..default()
            },
            Transform::from_translation((r.center() + Vec2::new(0.0, RH / 2.0 - 2.0)).extend(6.0)),
            Visibility::Hidden,
            Bubble(room),
        ));
    }
    // Salariés.
    let mut seed = 7u32;
    let mut rnd = move || {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        (seed >> 8) as f32 / (1u32 << 24) as f32
    };
    for i in 0..14u32 {
        let floor = i % 3;
        let x = MARGIN_X + 6.0 + rnd() * (BW - 2.0 * MARGIN_X - 12.0);
        commands.spawn((
            Sprite {
                image: sprites.people.clone(),
                rect: Some(Rect::new(0.0, 0.0, 10.0, 16.0)),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, 4.0 + i as f32 * 0.01),
            Walker {
                floor,
                x,
                target: x,
                speed: 10.0 + rnd() * 10.0,
                pause: rnd() * 3.0,
                anim: 0.0,
                look: i % 8,
            },
        ));
    }
    // Voyants du datacenter.
    let dc = room_px(Room::Datacenter);
    for rack in 0..4 {
        for row in 0..9 {
            let p = dc
                + Vec2::new(
                    3.0 + rack as f32 * 12.0 + 2.0 + (row % 2) as f32 * 2.0,
                    4.0 + 2.0 + row as f32 * 3.0,
                );
            let w = px_to_world(p) + Vec2::new(0.5, -0.5);
            let col = if (rack + row) % 3 == 0 {
                Color::srgb(0.95, 0.6, 0.2)
            } else {
                Color::srgb(0.5, 1.0, 0.35)
            };
            commands.spawn((
                Sprite::from_color(col, Vec2::ONE),
                Transform::from_translation(w.extend(2.5)),
                Led(rnd() * 3.0),
            ));
        }
    }
    commands.insert_resource(sprites);
}

fn apply_camera(
    mut vp: ResMut<WorldViewport>,
    view: Res<WorldView>,
    session: Res<Session>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut cam: Query<(&mut Transform, &mut Projection, &mut Camera), With<MainCamera>>,
) {
    let Ok((mut tf, mut proj, mut camera)) = cam.single_mut() else {
        return;
    };
    let avail = (vp.max - vp.min).max(Vec2::splat(100.0));
    let plateau = view.plateau && session.game.is_some();
    let (center, size) = if plateau {
        (crate::plateau::FRAME_CENTER, crate::plateau::FRAME_SIZE)
    } else {
        (Vec2::new(0.0, 4.0), Vec2::new(BW + 8.0, BH + 24.0))
    };
    let fit = (avail.x / size.x).min(avail.y / size.y);
    let zoom = if fit >= 3.0 {
        fit.floor().min(6.0)
    } else {
        ((fit * 4.0).floor() / 4.0).max(1.0)
    };
    vp.zoom = zoom;
    vp.cam = center;
    if let Projection::Orthographic(o) = proj.as_mut() {
        o.scale = 1.0 / zoom;
    }
    tf.translation.x = center.x;
    tf.translation.y = center.y;
    let scale = window.scale_factor();
    let w = window.physical_width();
    let h = window.physical_height();
    let min = (vp.min * scale)
        .as_uvec2()
        .min(UVec2::new(w.saturating_sub(1), h.saturating_sub(1)));
    let max = (vp.max * scale).as_uvec2().min(UVec2::new(w, h));
    let size = max.saturating_sub(min).max(UVec2::ONE);
    camera.viewport = Some(Viewport {
        physical_position: min,
        physical_size: size,
        ..default()
    });
}

/// Monde → position écran (pixels logiques de la fenêtre).
pub fn world_to_screen(vp: &WorldViewport, world: Vec2) -> Vec2 {
    let center = (vp.min + vp.max) / 2.0;
    let d = (world - vp.cam) * vp.zoom;
    Vec2::new(center.x + d.x, center.y - d.y)
}

fn pick(
    vp: Res<WorldViewport>,
    window: Single<&Window, With<PrimaryWindow>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut pickr: ResMut<RoomPick>,
    session: Res<Session>,
    view: Res<WorldView>,
    mut hover_box: Query<(&mut Transform, &mut Visibility), With<HoverBox>>,
) {
    pickr.hover = None;
    let playing = !view.plateau
        && session
            .game
            .as_ref()
            .is_some_and(|g| g.active.is_none() && !g.finished());
    if let Some(c) = window.cursor_position()
        && playing
        && !vp.pointer_over_ui
        && c.x >= vp.min.x
        && c.y >= vp.min.y
        && c.x <= vp.max.x
        && c.y <= vp.max.y
    {
        let center = (vp.min + vp.max) / 2.0;
        let d = c - center;
        let world = Vec2::new(d.x, -d.y) / vp.zoom + vp.cam;
        pickr.hover = Room::ALL
            .iter()
            .copied()
            .find(|r| room_world_rect(*r).contains(world));
        if mouse.just_pressed(MouseButton::Left) && pickr.hover.is_some() {
            pickr.clicked = pickr.hover;
        }
    }
    if let Ok((mut tf, mut vis)) = hover_box.single_mut() {
        match pickr.hover {
            Some(r) => {
                tf.translation = room_world_rect(r).center().extend(3.0);
                *vis = Visibility::Visible;
            }
            None => *vis = Visibility::Hidden,
        }
    }
}

fn walkers(
    time: Res<Time>,
    settings: Res<Settings>,
    mut q: Query<(&mut Walker, &mut Transform, &mut Sprite)>,
    mut tick: Local<u32>,
) {
    let dt = if settings.reduce_motion {
        0.0
    } else {
        time.delta_secs()
    };
    *tick = tick.wrapping_add(1);
    for (i, (mut w, mut tf, mut sprite)) in q.iter_mut().enumerate() {
        let mut frame = 2.0;
        if w.pause > 0.0 {
            w.pause -= dt;
        } else {
            let dir = (w.target - w.x).signum();
            w.x += dir * w.speed * dt;
            w.anim += dt * 6.0;
            frame = (w.anim as u32 % 2) as f32;
            sprite.flip_x = dir < 0.0;
            if (w.target - w.x).abs() < 1.0 {
                // Nouvelle destination : une autre salle du même étage.
                let h = (*tick as usize).wrapping_mul(31).wrapping_add(i * 97);
                let col = (h % 5) as f32;
                w.target = MARGIN_X + WALL + col * (RW + WALL) + 8.0 + ((h / 5) % 36) as f32;
                w.pause = 1.0 + ((h / 7) % 40) as f32 / 10.0;
                if (h / 3).is_multiple_of(4) {
                    w.floor = ((h / 11) % 3) as u32;
                    w.x = w.target;
                }
            }
        }
        let feet_px = ROOF + WALL + w.floor as f32 * (RH + WALL) + RH - 2.0;
        let p = px_to_world(Vec2::new(w.x.round(), feet_px));
        tf.translation.x = p.x;
        tf.translation.y = p.y + 8.0;
        let row = w.look as f32 * 16.0;
        sprite.rect = Some(Rect::new(
            frame * 10.0,
            row,
            frame * 10.0 + 10.0,
            row + 16.0,
        ));
    }
}

/// Type d'alerte affichée au-dessus d'une salle : 0 normal, 1 incident, 2 fournisseur.
pub fn pending_kind(
    content: &dsi_core::Content,
    g: &dsi_core::GameState,
    room: Room,
) -> Option<u32> {
    let mut best: Option<u32> = None;
    for item in &g.inbox {
        if dsi_core::game::inbox_room(content, item) != room {
            continue;
        }
        let k = match item {
            Inbox::Event { id } => match content.event(id).map(|e| e.kind) {
                Some(EventKind::Incident) => 1,
                Some(EventKind::Vendor | EventKind::Contract) => 2,
                _ => 0,
            },
            Inbox::Renewal { .. } => 2,
            Inbox::Consequence { .. } => 1,
            _ => 0,
        };
        best = Some(best.map_or(k, |b: u32| if k == 1 || b == 1 { 1 } else { b.max(k) }));
    }
    best
}

fn bubbles(
    time: Res<Time>,
    settings: Res<Settings>,
    session: Res<Session>,
    content: Res<GameContent>,
    mut q: Query<(&Bubble, &mut Transform, &mut Visibility, &mut Sprite)>,
) {
    let t = time.elapsed_secs();
    for (b, mut tf, mut vis, mut sprite) in q.iter_mut() {
        let kind = session
            .game
            .as_ref()
            .and_then(|g| pending_kind(&content.0, g, b.0));
        match kind {
            Some(k) => {
                *vis = Visibility::Visible;
                sprite.rect = Some(Rect::new(
                    k as f32 * 13.0,
                    0.0,
                    k as f32 * 13.0 + 13.0,
                    14.0,
                ));
                let r = room_world_rect(b.0);
                let bob = if settings.reduce_motion {
                    0.0
                } else {
                    ((t * 4.0).sin() * 1.5).round()
                };
                tf.translation.y = r.max.y - 9.0 + bob;
            }
            None => *vis = Visibility::Hidden,
        }
    }
}

fn glows(
    time: Res<Time>,
    settings: Res<Settings>,
    session: Res<Session>,
    content: Res<GameContent>,
    mut q: Query<(&Glow, &mut Sprite)>,
) {
    let t = time.elapsed_secs();
    for (g, mut sprite) in q.iter_mut() {
        let kind = session
            .game
            .as_ref()
            .and_then(|s| pending_kind(&content.0, s, g.0));
        let a = match kind {
            Some(1) if !settings.reduce_motion => 0.10 + 0.10 * (t * 5.0).sin().abs(),
            Some(1) => 0.15,
            _ => 0.0,
        };
        sprite.color = Color::srgba(1.0, 0.15, 0.1, a);
    }
}

fn leds(time: Res<Time>, settings: Res<Settings>, mut q: Query<(&mut Led, &mut Visibility)>) {
    if settings.reduce_motion {
        return;
    }
    let dt = time.delta_secs();
    for (mut l, mut vis) in q.iter_mut() {
        l.0 -= dt;
        if l.0 <= 0.0 {
            let on = *vis != Visibility::Hidden;
            *vis = if on {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
            l.0 = 0.15 + ((l.0.abs() * 7919.0) % 1.0) * 1.2;
        }
    }
}
