//! Le « Plateau DSI » : l'open space de la DSI en 3D isométrique (pixel art).
//! On y voit le ou la DSI à son bureau, le mobilier, chaque recrue au travail
//! (frappe au clavier, pauses café, humeur), les prestataires ESN dans leur coin,
//! et les agents IA une fois débloqués. Tout est synchronisé avec la simulation.

use crate::sim::{GameContent, Session, Settings};
use crate::world::{WorldView, WorldViewport};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use dsi_core::state::{Inbox, Metric};

/// Position du plateau dans le monde (loin du siège).
pub const ORIGIN: Vec2 = Vec2::new(1400.0, 0.0);
pub const W: i32 = 11;
pub const D: i32 = 9;
/// Centre et taille du cadrage caméra (monde).
pub const FRAME_CENTER: Vec2 = Vec2::new(ORIGIN.x + 16.0, ORIGIN.y - 52.0);
pub const FRAME_SIZE: Vec2 = Vec2::new(352.0, 232.0);

const OBJ_CELL: f32 = 64.0;
const OBJ_COLS: u32 = 8;

// Index d'objets dans iso_objects.png (voir tools/art/gen_iso.py).
const DESK: u32 = 0;
const DESK_BIG: u32 = 1;
const COFFEE: u32 = 2;
const PLANT: u32 = 3;
const RACK: u32 = 4;
const TABLE: u32 = 5;
const TV: u32 = 6;
const ROBOT: u32 = 7;
const PRINTER: u32 = 8;
const SOFA: u32 = 9;
const CONTEXT: u32 = 10;
const WALL_L: u32 = 11;
const WALL_L_WINDOW: u32 = 12;
const WALL_L_DOOR: u32 = 13;
const WALL_L_POSTER: u32 = 14;
const WALL_R: u32 = 15;
const WALL_R_WINDOW: u32 = 16;
const WALL_R_KANBAN: u32 = 17;
const WALL_R_SCREEN: u32 = 18;
const WALL_R_SIGN: u32 = 19;

const TEAM_DESKS: [(i32, i32); 12] = [
    (3, 3),
    (4, 3),
    (5, 3),
    (6, 3),
    (3, 5),
    (4, 5),
    (5, 5),
    (6, 5),
    (3, 7),
    (4, 7),
    (5, 7),
    (6, 7),
];
const ESN_DESKS: [(i32, i32); 4] = [(8, 1), (9, 1), (8, 2), (9, 2)];
const DSI_DESK: (i32, i32) = (1, 1);
const COFFEE_TILE: (i32, i32) = (0, 7);
const COFFEE_SPOT: Vec2 = Vec2::new(1.1, 7.5);
const DOOR: Vec2 = Vec2::new(0.4, 4.5);

/// Grille (coordonnées continues) → monde.
pub fn iso(g: Vec2) -> Vec2 {
    ORIGIN + Vec2::new((g.x - g.y) * 16.0, -(g.x + g.y) * 8.0)
}

fn tile_center(i: i32, j: i32) -> Vec2 {
    Vec2::new(i as f32 + 0.5, j as f32 + 0.5)
}

/// Profondeur : plus on est proche du joueur (x + y grands), plus on est devant.
fn depth(g: Vec2) -> f32 {
    1.0 + (g.x + g.y) * 0.01
}

#[derive(Resource)]
pub struct IsoSprites {
    tiles: Handle<Image>,
    objects: Handle<Image>,
    people: Handle<Image>,
    fx: Handle<Image>,
}

/// Personne survolée sur le plateau (étiquette, position écran).
#[derive(Resource, Default)]
pub struct PlateauHover(pub Option<(String, Vec2)>);

/// Résumé affiché par l'interface.
#[derive(Resource, Default)]
pub struct PlateauStats {
    pub internes: usize,
    pub esn: usize,
    pub agents: usize,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Doing {
    Walking { to: Vec2, then_coffee: bool },
    Seated,
    Coffee,
}

#[derive(Component)]
struct Person {
    key: String,
    label: String,
    seat: Vec2,
    pos: Vec2,
    doing: Doing,
    timer: f32,
    anim: f32,
    row: u32,
    bubble: Option<u32>,
    bubble_t: f32,
    seed: u32,
    dsi: bool,
}

#[derive(Component)]
struct PersonBubble(Entity);

#[derive(Component)]
struct DynObject(String);

pub struct PlateauPlugin;

impl Plugin for PlateauPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlateauHover>()
            .init_resource::<PlateauStats>()
            .add_systems(Startup, setup)
            .add_systems(Update, (sync, behave, place, hover).chain());
    }
}

fn obj_sprite(image: &Handle<Image>, idx: u32) -> Sprite {
    let x = (idx % OBJ_COLS) as f32 * OBJ_CELL;
    let y = (idx / OBJ_COLS) as f32 * OBJ_CELL;
    Sprite {
        image: image.clone(),
        rect: Some(Rect::new(x, y, x + OBJ_CELL, y + OBJ_CELL)),
        ..default()
    }
}

/// Transform d'un objet dont l'ancre (32, 48) tombe sur `g` (grille).
fn obj_transform(g: Vec2, z: f32) -> Transform {
    let a = iso(g);
    Transform::from_xyz(a.x, a.y + 16.0, z)
}

fn spawn_obj(commands: &mut Commands, s: &IsoSprites, idx: u32, g: Vec2) {
    commands.spawn((obj_sprite(&s.objects, idx), obj_transform(g, depth(g))));
}

fn setup(mut commands: Commands, server: Res<AssetServer>) {
    let s = IsoSprites {
        tiles: server.load("sprites/iso_tiles.png"),
        objects: server.load("sprites/iso_objects.png"),
        people: server.load("sprites/iso_people.png"),
        fx: server.load("sprites/fx.png"),
    };
    // Fond du plateau.
    commands.spawn((
        Sprite::from_color(Color::srgb_u8(18, 22, 32), Vec2::new(900.0, 600.0)),
        Transform::from_xyz(FRAME_CENTER.x, FRAME_CENTER.y, -1.0),
    ));
    // Sol.
    for i in 0..W {
        for j in 0..D {
            let kind = if i < 3 && j < 3 {
                2
            } else if i >= 8 && j < 3 {
                3
            } else if i < 2 && j >= 6 {
                4
            } else {
                ((i + j) % 2) as u32
            };
            let c = iso(tile_center(i, j));
            commands.spawn((
                Sprite {
                    image: s.tiles.clone(),
                    rect: Some(Rect::new(
                        kind as f32 * 32.0,
                        0.0,
                        kind as f32 * 32.0 + 32.0,
                        16.0,
                    )),
                    ..default()
                },
                Transform::from_xyz(c.x, c.y, 0.0),
            ));
        }
    }
    // Murs du fond.
    for j in 0..D {
        let idx = match j {
            2 | 7 => WALL_L_WINDOW,
            4 => WALL_L_DOOR,
            6 => WALL_L_POSTER,
            _ => WALL_L,
        };
        let g = tile_center(0, j);
        commands.spawn((
            obj_sprite(&s.objects, idx),
            obj_transform(g, 0.5 + j as f32 * 0.001),
        ));
    }
    for i in 0..W {
        let idx = match i {
            1 | 6 => WALL_R_WINDOW,
            3 => WALL_R_KANBAN,
            5 => WALL_R_SCREEN,
            9 => WALL_R_SIGN,
            _ => WALL_R,
        };
        let g = tile_center(i, 0);
        commands.spawn((
            obj_sprite(&s.objects, idx),
            obj_transform(g, 0.5 + i as f32 * 0.001),
        ));
    }
    // Mobilier.
    spawn_obj(
        &mut commands,
        &s,
        DESK_BIG,
        tile_center(DSI_DESK.0, DSI_DESK.1),
    );
    for (i, j) in TEAM_DESKS.iter().chain(ESN_DESKS.iter()) {
        spawn_obj(&mut commands, &s, DESK, tile_center(*i, *j));
    }
    spawn_obj(
        &mut commands,
        &s,
        COFFEE,
        tile_center(COFFEE_TILE.0, COFFEE_TILE.1),
    );
    for (i, j) in [(0, 0), (2, 0), (10, 8), (7, 4), (0, 3)] {
        spawn_obj(&mut commands, &s, PLANT, tile_center(i, j));
    }
    spawn_obj(&mut commands, &s, RACK, tile_center(7, 0));
    spawn_obj(&mut commands, &s, TABLE, Vec2::new(9.0, 6.5));
    spawn_obj(&mut commands, &s, TV, tile_center(10, 4));
    spawn_obj(&mut commands, &s, PRINTER, tile_center(2, 8));
    spawn_obj(&mut commands, &s, SOFA, tile_center(1, 5));
    commands.insert_resource(s);
}

fn hash(s: &str) -> u32 {
    s.bytes()
        .fold(2166136261u32, |h, b| (h ^ b as u32).wrapping_mul(16777619))
}

fn role_shirt(role: &str) -> u32 {
    match role {
        "junior" => 0,
        "senior" => 1,
        "staff" => 2,
        "architecte" => 3,
        "sre" => 4,
        "secu" => 5,
        "data_eng" => 6,
        "ml_eng" => 7,
        "platform" => 8,
        _ => 1,
    }
}

struct Wanted {
    key: String,
    label: String,
    seat: Vec2,
    row: u32,
    dsi: bool,
}

fn wanted(
    content: &dsi_core::Content,
    g: &dsi_core::GameState,
) -> (Vec<Wanted>, Vec<(String, u32, Vec2)>) {
    let mut v = Vec::new();
    let seat_of = |i: i32, j: i32| Vec2::new(i as f32 + 0.5 - 0.36, j as f32 + 0.5 - 0.36);
    v.push(Wanted {
        key: "dsi".into(),
        label: format!("{} — DSI (vous)", g.profile.name),
        seat: seat_of(DSI_DESK.0, DSI_DESK.1) - Vec2::new(0.1, 0.0),
        // Lignes 33+ : un sprite par avatar de DSI (voir tools/art/gen_iso.py).
        row: match g.profile.avatar {
            Some(a) => 33 + a.min(15),
            None => 10 * 3 + if g.profile.female { 1 } else { 0 },
        },
        dsi: true,
    });
    for (k, h) in g.team.iter().enumerate().take(TEAM_DESKS.len()) {
        let role = content
            .careers
            .roles
            .iter()
            .find(|r| r.id == h.role)
            .map(|r| r.label.clone())
            .unwrap_or_default();
        let (i, j) = TEAM_DESKS[k];
        v.push(Wanted {
            key: format!("hire:{}:{}", h.name, h.since),
            label: format!("{} — {role}", h.name),
            seat: seat_of(i, j),
            row: role_shirt(&h.role) * 3 + hash(&h.name) % 3,
            dsi: false,
        });
    }
    let esn = g
        .contracts
        .iter()
        .find(|c| c.id == "esn_pool" && c.active)
        .map(|c| (c.monthly_cost / 0.3).round() as usize)
        .unwrap_or(0)
        .min(ESN_DESKS.len());
    for k in 0..esn {
        let (i, j) = ESN_DESKS[k];
        v.push(Wanted {
            key: format!("esn:{k}"),
            label: "Prestataire ESN (régie) — « exactement le profil »".into(),
            seat: seat_of(i, j),
            row: 9 * 3 + (k as u32 % 3),
            dsi: false,
        });
    }
    let mut objs = Vec::new();
    if g.has_tech("agents") {
        objs.push(("robot1".to_string(), ROBOT, tile_center(8, 4)));
        objs.push(("robot2".to_string(), ROBOT, tile_center(9, 3)));
    }
    if g.has_tech("context_platform") {
        objs.push(("context".to_string(), CONTEXT, tile_center(10, 2)));
    }
    if g.has_tech("kubernetes") || g.has_tech("cloud_public") || g.has_tech("cloud_souverain") {
        objs.push(("rack2".to_string(), RACK, tile_center(8, 0)));
    }
    (v, objs)
}

fn sync(
    mut commands: Commands,
    sprites: Option<Res<IsoSprites>>,
    session: Res<Session>,
    content: Res<GameContent>,
    people: Query<(Entity, &Person)>,
    bubbles: Query<(Entity, &PersonBubble)>,
    objs: Query<(Entity, &DynObject)>,
    mut stats: ResMut<PlateauStats>,
    mut last_rev: Local<Option<u64>>,
    mut last_game: Local<Option<u64>>,
) {
    let Some(s) = sprites else { return };
    let game_id = session.game.as_ref().map(|g| g.seed);
    if *last_rev == Some(session.rev) && *last_game == game_id {
        return;
    }
    // Nouvelle partie : tout le monde est déjà assis, personne n'entre par la porte.
    let fresh = *last_game != game_id;
    *last_rev = Some(session.rev);
    *last_game = game_id;
    let (want, want_objs) = match &session.game {
        Some(g) => wanted(&content.0, g),
        None => (Vec::new(), Vec::new()),
    };
    stats.internes = want.iter().filter(|w| w.key.starts_with("hire:")).count();
    stats.esn = want.iter().filter(|w| w.key.starts_with("esn:")).count();
    stats.agents = want_objs.iter().filter(|o| o.1 == ROBOT).count();
    // Départs.
    for (e, p) in people.iter() {
        if fresh || !want.iter().any(|w| w.key == p.key) {
            commands.entity(e).despawn();
            for (b, owner) in bubbles.iter() {
                if owner.0 == e {
                    commands.entity(b).despawn();
                }
            }
        }
    }
    // Arrivées.
    for w in want {
        if !fresh && people.iter().any(|(_, p)| p.key == w.key) {
            continue;
        }
        let seed = hash(&w.key);
        let arriving = !fresh && !w.dsi;
        let start = if arriving { DOOR } else { w.seat };
        let e = commands
            .spawn((
                Sprite {
                    image: s.people.clone(),
                    rect: Some(Rect::new(0.0, 0.0, 16.0, 24.0)),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, 1.0),
                Person {
                    key: w.key,
                    label: w.label,
                    seat: w.seat,
                    pos: start,
                    doing: if arriving {
                        Doing::Walking {
                            to: w.seat,
                            then_coffee: false,
                        }
                    } else {
                        Doing::Seated
                    },
                    timer: 4.0 + (seed % 100) as f32 / 10.0,
                    anim: (seed % 7) as f32,
                    row: w.row,
                    bubble: if arriving { Some(8) } else { None },
                    bubble_t: if arriving { 4.0 } else { 0.0 },
                    seed,
                    dsi: w.dsi,
                },
            ))
            .id();
        commands.spawn((
            Sprite {
                image: s.fx.clone(),
                rect: Some(Rect::new(0.0, 0.0, 13.0, 14.0)),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, 9.0),
            Visibility::Hidden,
            PersonBubble(e),
        ));
    }
    for (e, o) in objs.iter() {
        if !want_objs.iter().any(|w| w.0 == o.0) {
            commands.entity(e).despawn();
        }
    }
    for (key, idx, g) in want_objs {
        if !objs.iter().any(|(_, o)| o.0 == key) {
            commands.spawn((
                obj_sprite(&s.objects, idx),
                obj_transform(g, depth(g)),
                DynObject(key),
            ));
        }
    }
}

fn behave(
    time: Res<Time>,
    settings: Res<Settings>,
    session: Res<Session>,
    content: Res<GameContent>,
    mut q: Query<&mut Person>,
) {
    let dt = if settings.reduce_motion {
        0.0
    } else {
        time.delta_secs()
    };
    let g = session.game.as_ref();
    let dev_sat = g.map(|g| g.m(Metric::DevSat)).unwrap_or(50.0);
    let incident = g.is_some_and(|g| {
        g.inbox.iter().any(|i| matches!(i, Inbox::Event { id } if content.0.event(id).is_some_and(|e| e.kind == dsi_core::events::EventKind::Incident)))
    });
    let pending = g.is_some_and(|g| !g.inbox.is_empty());
    for mut p in q.iter_mut() {
        p.anim += dt;
        p.timer -= dt;
        if p.bubble_t > 0.0 {
            p.bubble_t -= dt;
            if p.bubble_t <= 0.0 {
                p.bubble = None;
            }
        }
        match p.doing {
            Doing::Walking { to, then_coffee } => {
                let d = to - p.pos;
                let step = 1.6 * dt;
                if d.length() <= step {
                    p.pos = to;
                    if then_coffee {
                        p.doing = Doing::Coffee;
                        p.timer = 3.5;
                        p.bubble = Some(5);
                        p.bubble_t = 3.5;
                    } else {
                        p.doing = Doing::Seated;
                        p.timer = 8.0 + (p.seed % 13) as f32;
                    }
                } else {
                    p.pos += d.normalize() * step;
                }
            }
            Doing::Coffee => {
                if p.timer <= 0.0 {
                    let seat = p.seat;
                    p.doing = Doing::Walking {
                        to: seat,
                        then_coffee: false,
                    };
                }
            }
            Doing::Seated => {
                if p.timer <= 0.0 {
                    p.seed = p.seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                    let r = (p.seed >> 16) % 100;
                    p.timer = 7.0 + (r % 11) as f32;
                    if !p.dsi && r < 22 {
                        p.doing = Doing::Walking {
                            to: COFFEE_SPOT + Vec2::new((r % 3) as f32 * 0.3, 0.0),
                            then_coffee: true,
                        };
                    } else if !p.dsi && dev_sat < 32.0 && r < 55 {
                        p.bubble = Some(7);
                        p.bubble_t = 2.5;
                    } else if !p.dsi && r < 70 {
                        p.bubble = Some(6);
                        p.bubble_t = 2.0;
                    }
                }
            }
        }
        if p.dsi {
            p.bubble = if incident {
                Some(1)
            } else if pending {
                Some(0)
            } else {
                None
            };
            p.bubble_t = 1.0;
        }
    }
}

fn place(
    time: Res<Time>,
    settings: Res<Settings>,
    mut q: Query<(Entity, &Person, &mut Transform, &mut Sprite), Without<PersonBubble>>,
    mut b: Query<(&PersonBubble, &mut Transform, &mut Visibility, &mut Sprite), Without<Person>>,
) {
    let t = time.elapsed_secs();
    let mut heads: Vec<(Entity, Vec3, Option<u32>)> = Vec::new();
    for (e, p, mut tf, mut sprite) in q.iter_mut() {
        let w = iso(p.pos);
        let (frame, flip) = match p.doing {
            Doing::Seated => {
                let typing = !settings.reduce_motion && ((p.anim * 0.4).sin() > -0.2);
                (
                    if typing && (p.anim * 7.0) as i32 % 2 == 0 {
                        0
                    } else {
                        1
                    },
                    false,
                )
            }
            Doing::Walking { to, .. } => {
                let dx = iso(to).x - w.x;
                (2 + ((p.anim * 6.0) as u32 % 2), dx < 0.0)
            }
            Doing::Coffee => (4, false),
        };
        let row = p.row as f32 * 24.0;
        sprite.rect = Some(Rect::new(
            frame as f32 * 16.0,
            row,
            frame as f32 * 16.0 + 16.0,
            row + 24.0,
        ));
        sprite.flip_x = flip;
        tf.translation = Vec3::new(w.x.round(), (w.y + 12.0).round(), depth(p.pos) - 0.003);
        let bob = if settings.reduce_motion {
            0.0
        } else {
            ((t * 3.0 + p.seed as f32).sin() * 1.0).round()
        };
        heads.push((
            e,
            Vec3::new(tf.translation.x, tf.translation.y + 19.0 + bob, 9.0),
            p.bubble,
        ));
    }
    for (pb, mut tf, mut vis, mut sprite) in b.iter_mut() {
        match heads.iter().find(|h| h.0 == pb.0) {
            Some((_, pos, Some(k))) => {
                tf.translation = *pos;
                sprite.rect = Some(Rect::new(
                    *k as f32 * 13.0,
                    0.0,
                    *k as f32 * 13.0 + 13.0,
                    14.0,
                ));
                *vis = Visibility::Visible;
            }
            _ => *vis = Visibility::Hidden,
        }
    }
}

fn hover(
    view: Res<WorldView>,
    vp: Res<WorldViewport>,
    window: Single<&Window, With<PrimaryWindow>>,
    q: Query<(&Person, &Transform)>,
    mut out: ResMut<PlateauHover>,
) {
    out.0 = None;
    if !view.plateau || vp.pointer_over_ui {
        return;
    }
    let Some(c) = window.cursor_position() else {
        return;
    };
    let mut best: Option<(f32, String, Vec2)> = None;
    for (p, tf) in q.iter() {
        let s = crate::world::world_to_screen(&vp, tf.translation.truncate());
        let d = (s - c).length();
        if d < 9.0 * vp.zoom && best.as_ref().is_none_or(|b| d < b.0) {
            best = Some((d, p.label.clone(), s));
        }
    }
    out.0 = best.map(|b| (b.1, b.2));
}
