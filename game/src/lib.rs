//! Formosaic Game — game logic, entry point, and platform hosting.
//!
//! Depends on `formosaic-engine`. Nothing in `formosaic-engine` imports from here.
//!
//! # Module structure
//!
//! - `formosaic`   — Formosaic game struct, implements Application
//! - `asset_loader`— platform-specific asset I/O
//! - `rendering`   — game-specific renderers (hint, shine, menu)
//! - `level`       — level storage and Poly Pizza API client
//! - `puzzle`      — scrambler, entropy analysis, hint system

pub mod asset_loader;
pub mod formosaic;
pub mod rendering;
pub mod input;
pub mod level;
pub mod puzzle;

pub use formosaic::Formosaic;
// Re-export engine types so examples don't need to depend on the engine crate directly.
pub use formosaic_engine::app::game_engine::GameEngine;
pub use formosaic_engine::app::application::Application;
