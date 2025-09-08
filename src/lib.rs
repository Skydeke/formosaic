//! Shared library code for Formosaic
//! This is the core game and graphics logic used both by desktop binary and Android.

pub mod engine;
pub mod formosaic;
pub mod game_engine;
pub mod input;
pub mod opengl;

pub use formosaic::Formosaic;
pub use input::{Event as EngineEvent, Key as EngineKey};
