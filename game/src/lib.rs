//! Formosaic Game — game logic, entry point, and platform hosting.
//!
//! Depends on `formosaic-engine`. Nothing in `formosaic-engine` imports from here.
//!
//! # Module structure
//!
//! - `formosaic`   — Formosaic game struct, implements Application
//! - `game_engine` — re-export of engine's GameEngine
//! - `asset_loader`— platform-specific asset I/O
//! - `rendering`   — game-specific renderers (hint, shine, menu)
//! - `level`       — level storage and Poly Pizza API client
//! - `puzzle`      — scrambler, entropy analysis, hint system

pub mod asset_loader;
pub mod formosaic;
pub mod rendering;
pub mod game_engine;
pub mod input;
pub mod level;
pub mod puzzle;

pub use formosaic::Formosaic;
pub use game_engine::GameEngine;
// Application trait lives in the engine — re-export for convenience
pub use formosaic_engine::app::application::Application;
