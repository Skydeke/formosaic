//! Formosaic Game — game logic, entry point, and platform hosting.
//!
//! Depends on `formosaic-engine`. Nothing in `formosaic-engine` imports from here.
//!
//! # Module structure
//!
//! - `formosaic`   — Game logic, Application trait, Formosaic struct
//! - `game_engine` — winit/glutin event loop host
//! - `input`       — Input event types
//! - `level`       — Level storage and Poly Pizza API client
//! - `puzzle`      — Scrambler, entropy analysis, hint system

pub mod asset_loader;
pub mod formosaic;
pub mod rendering;
pub mod game_engine;
pub mod input;
pub mod level;
pub mod puzzle;

pub use formosaic::Formosaic;
pub use game_engine::GameEngine;
pub use input::{Event as EngineEvent, Key as EngineKey};
