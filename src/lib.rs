//! Shared library code for Formosaic
//! This is the core game and graphics logic used both by desktop binary and Android.

pub mod game;
pub mod input;
pub mod renderer;
pub mod shaders;
pub mod shared;

pub use game::Game;
pub use input::{Event as EngineEvent, Key as EngineKey};
