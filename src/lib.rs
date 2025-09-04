//! Shared library code for Formosaic
//! This is the core game and graphics logic used both by desktop binary and Android.

pub mod game;
pub mod input;
pub mod mesh;
pub mod opengl;
pub mod renderable;
pub mod renderer;
pub mod shared;
pub mod simple_model;

pub use game::Game;
pub use input::{Event as EngineEvent, Key as EngineKey};
