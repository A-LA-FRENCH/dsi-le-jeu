//! # dsi-core
//!
//! Simulation déterministe de « DSI — Le Jeu », indépendante de tout moteur de rendu.
//! Le client (Bevy) envoie des [`game::Command`] et affiche le [`state::GameState`].
//!
//! Tout le contenu (événements, fournisseurs, technos, projets, carrières, dialogues,
//! crises) vit dans `data/*.json` et est validé par les tests.

#![allow(clippy::type_complexity)]

pub mod ai;
pub mod cloud;
pub mod conditions;
pub mod content;
pub mod contracts;
pub mod economy;
pub mod effects;
pub mod events;
pub mod game;
pub mod mentors;
pub mod negotiation;
pub mod people;
pub mod projects;
pub mod save;
pub mod score;
pub mod security;
pub mod share;
pub mod skills;
pub mod state;
pub mod technology;
pub mod vendors;

pub use content::Content;
pub use game::{Command, GameError, apply, new_game};
pub use state::{GameState, Metric, Profile};
