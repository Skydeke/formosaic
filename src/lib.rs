//! Shared library code for Formosaic
//! This is the core game and graphics logic used both by desktop binary and Android.

pub mod engine;
pub mod formosaic;
pub mod game_engine;
pub mod input;
pub mod level;
pub mod opengl;
pub mod puzzle;

pub use formosaic::Formosaic;
pub use input::{Event as EngineEvent, Key as EngineKey};

// src/imgui_renderer.rs was a dead duplicate of engine/rendering/instances/imgui_render.rs
// It has been removed. The canonical ImguiGlRenderer lives in:
//   crate::engine::rendering::instances::imgui_render
