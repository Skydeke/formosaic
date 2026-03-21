//! Game-specific renderers.
//!
//! These renderers live in the game crate because they encode game logic
//! (what "solved" looks like, what hints look like, what the menu looks like).
//! The engine crate knows nothing about them.

pub mod hint_render;
pub mod menu_render;
pub mod shine_render;
