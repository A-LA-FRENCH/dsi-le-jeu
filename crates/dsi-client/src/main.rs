//! DSI — LE JEU : couche de présentation Bevy.
//!
//! La simulation vit dans `dsi-core` (Rust pur, déterministe, testée). Cette crate
//! l'affiche : le siège en coupe en pixel art (15 salles cliquables, salariés qui
//! circulent, bulles d'alerte), un tableau de bord egui, les mini-jeux, l'audio
//! procédural et les sauvegardes.

#![allow(clippy::too_many_arguments, clippy::type_complexity)]

mod audio;
mod plateau;
mod share;
mod sim;
mod ui;
#[cfg(target_arch = "wasm32")]
mod web;
mod world;

use bevy::asset::AssetMetaCheck;
use bevy::prelude::*;
use bevy::window::WindowResolution;
use bevy_egui::EguiPlugin;

fn asset_root() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        "assets".to_string()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var("DSI_ASSETS")
            .unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets").to_string())
    }
}

fn main() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "DSI — Le Jeu".into(),
                    resolution: WindowResolution::from((1280u32, 720u32)),
                    canvas: Some("#game-canvas".into()),
                    fit_canvas_to_parent: true,
                    prevent_default_event_handling: true,
                    ..default()
                }),
                ..default()
            })
            .set(ImagePlugin::default_nearest())
            .set(AssetPlugin {
                file_path: asset_root(),
                meta_check: AssetMetaCheck::Never,
                ..default()
            }),
    )
    .add_plugins(EguiPlugin::default())
    .insert_resource(ClearColor(Color::srgb_u8(14, 17, 26)))
    .add_plugins((
        sim::SimPlugin,
        world::WorldPlugin,
        plateau::PlateauPlugin,
        ui::UiPlugin,
        audio::AudioPlugin,
    ));
    #[cfg(target_arch = "wasm32")]
    app.add_plugins(web::WebPlugin);
    app.run();
}
