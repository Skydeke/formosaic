//! Game-specific renderers.
//!
//! These renderers live in the game crate because they encode game logic
//! (what "solved" looks like, what hints look like, what the menu looks like).
//! The engine crate knows nothing about them.

pub mod hint_render;
pub mod menu_render;
pub mod shine_render;

// ─── Per-frame game render data ───────────────────────────────────────────────
//
// Written by the game layer into `SceneContext::game_render_data` each frame.
// Game-side renderers downcast `ctx.game_render_data` to this type.
// Keeps game-specific types out of the engine.

/// Hint overlay state for the current frame.
#[derive(Clone, Copy, Default)]
pub struct HintRenderState {
    pub warmth: f32,
    pub warmth_color: [f32; 3],
    pub tier: u8,
    pub time: f32,
}

/// All game-specific data passed from the game layer to game-side renderers
/// via `SceneContext::game_render_data`.
pub struct GameRenderData {
    pub hints: Option<HintRenderState>,
    /// Seconds since solve; `None` means not yet solved.
    pub solved_timer: Option<f32>,
}
