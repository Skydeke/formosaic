//! Per-frame render state written by the game layer into `SceneContext`
//! each frame.  These types live in the engine because `SceneContext` is engine
//! infrastructure — but they contain only plain render parameters, no game
//! logic.  The game fills them; renderers read them.

/// Scene lighting parameters.
#[derive(Clone, Copy, Debug)]
pub struct LightConfig {
    /// Background / sky clear colour (linear RGB).
    pub clear_color: [f32; 3],
    /// Sun direction in world space (need not be normalised).
    pub sun_dir:     [f32; 3],
    /// Sun colour multiplier (HDR range allowed).
    pub sun_color:   [f32; 3],
    /// Hemisphere ambient sky colour for upward-facing surfaces.
    pub sky_color:   [f32; 3],
    /// Minimum ambient level for downward-facing surfaces.
    pub ambient_min: f32,
}

impl Default for LightConfig {
    fn default() -> Self {
        Self {
            // Near-black neutral — barely any hue so it doesn't clash
            clear_color: [0.02, 0.02, 0.03],
            // Sun from upper-right-front
            sun_dir:     [0.6,  0.9,  0.5],
            // Warm sun, pulled back — tone-map handles highlights
            sun_color:   [0.85, 0.78, 0.62],
            // Subtle cool sky ambient
            sky_color:   [0.25, 0.35, 0.60],
            // Low ambient — let shadows be shadows
            ambient_min: 0.14,
        }
    }
}

/// Hint overlay state for the current frame.
#[derive(Clone, Copy, Default)]
pub struct HintRenderState {
    pub warmth:       f32,
    pub warmth_color: [f32; 3],
    pub tier:         u8,
    pub time:         f32,
}
