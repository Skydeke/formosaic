//! Shared library code for Formosaic
//! This is the core game and graphics logic used both by desktop binary and Android.

pub mod game;
pub mod graphics;
pub mod input;
pub mod shared;

pub use game::Game;
pub use graphics::{Renderer, Shader};
pub use input::{Event as EngineEvent, Key as EngineKey};
