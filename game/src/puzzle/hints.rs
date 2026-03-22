//! Progressive 3-tier hint system for Formosaic.
//!
//! # Hint tiers
//!
//! | Tier | What the player sees | Trigger            |
//! |------|----------------------|--------------------|
//! | 0    | Nothing (default)    | –                  |
//! | 1    | Warm/cold compass    | H pressed once     |
//! | 2    | Axis-plane indicator | H pressed twice    |
//! | 3    | Ghost model overlay  | H pressed 3 times  |
//!
//! ## Tier 1 – Warm/Cold
//!
//! The HUD shows a directional arrow that gets "warmer" (red → orange → yellow)
//! as the camera approaches the solution hemisphere, and "cooler" (blue → teal)
//! as it moves away.  No axis is revealed — just a colour temperature.
//!
//! ## Tier 2 – Axis-Plane Indicator
//!
//! A translucent disc is rendered in world space, perpendicular to the solution
//! axis.  The camera must look roughly toward the centre of this disc.  The disc
//! is always visible but gives away neither which of the two valid directions
//! (±axis) to look from.
//!
//! ## Tier 3 – Ghost Snap
//!
//! The scramble lerp is driven toward `t = 0` by a fraction each frame, so the
//! model slowly "un-scrambles" over ~5 s.  This gives away the solution
//! immediately — it is a last resort.

use cgmath::{InnerSpace, Vector3};

// ─── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HintTier {
    #[default]
    None  = 0,
    WarmCold = 1,
    AxisPlane = 2,
    GhostSnap = 3,
}

impl HintTier {
    pub fn next(self) -> Self {
        match self {
            HintTier::None     => HintTier::WarmCold,
            HintTier::WarmCold => HintTier::AxisPlane,
            HintTier::AxisPlane => HintTier::GhostSnap,
            HintTier::GhostSnap => HintTier::GhostSnap, // stays at max
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Output of `HintSystem::update` — everything the renderer needs each frame.
#[derive(Debug, Clone, Copy)]
pub struct HintOutput {
    /// Current hint tier.
    pub tier: HintTier,
    /// [0,1] warmth of the warm/cold indicator.  1 = facing solution, 0 = facing away.
    pub warmth: f32,
    /// RGB colour for the warm/cold arrow.
    pub warmth_color: [f32; 3],
    /// Whether to draw the axis-plane disc.
    pub show_disc: bool,
    /// Normal of the disc (= solution axis).
    pub disc_normal: Vector3<f32>,
    /// How much to un-scramble the model (0 = fully scrambled, 1 = solved).
    /// Only >0 for GhostSnap tier.
    pub ghost_lerp: f32,
    /// Hint penalty to add to the score (increments each time a hint is used).
    pub hint_count: u32,
}

pub struct HintSystem {
    tier: HintTier,
    ghost_lerp: f32,
    hint_count: u32,
}

impl HintSystem {
    pub fn new() -> Self {
        Self {
            tier: HintTier::None,
            ghost_lerp: 0.0,
            hint_count: 0,
        }
    }

    /// Called when the player presses the hint key.
    pub fn advance(&mut self) {
        self.tier = self.tier.next();
        self.hint_count += 1;
        log::info!("[Hint] advanced to tier {:?}", self.tier);
    }

    /// Reset hints (e.g. new level / rescramble).
    pub fn reset(&mut self) {
        self.tier = HintTier::None;
        self.ghost_lerp = 0.0;
        // hint_count intentionally NOT reset so score penalty persists per session.
    }

    pub fn reset_full(&mut self) {
        self.tier = HintTier::None;
        self.ghost_lerp = 0.0;
        self.hint_count = 0;
    }

    /// Update each frame.  `delta_time` in seconds, `camera_fwd` and `solution_dir` in world space.
    pub fn update(
        &mut self,
        delta_time: f32,
        camera_fwd: Vector3<f32>,
        solution_dir: Vector3<f32>,
    ) -> HintOutput {
        let dot    = camera_fwd.normalize().dot(solution_dir.normalize());
        let warmth = (dot.abs() + 1.0) * 0.5; // 0.5 = perpendicular, 1 = on-axis

        // Ghost snap: slowly un-scramble over ~5 s.
        if self.tier == HintTier::GhostSnap {
            self.ghost_lerp = (self.ghost_lerp + delta_time / 5.0).min(1.0);
        }

        let warmth_color = warmth_to_rgb(warmth);

        HintOutput {
            tier: self.tier,
            warmth,
            warmth_color,
            show_disc:    self.tier as u8 >= HintTier::AxisPlane as u8,
            disc_normal:  solution_dir,
            ghost_lerp:   if self.tier == HintTier::GhostSnap { self.ghost_lerp } else { 0.0 },
            hint_count:   self.hint_count,
        }
    }

    pub fn tier(&self) -> HintTier {
        self.tier
    }

    pub fn hint_count(&self) -> u32 {
        self.hint_count
    }
}

impl Default for HintSystem {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Colour helpers ───────────────────────────────────────────────────────────

/// Map a [0,1] warmth value to an RGB colour.
/// 0 = cold blue, 0.5 = neutral white, 1 = hot red.
fn warmth_to_rgb(w: f32) -> [f32; 3] {
    let w = w.clamp(0.0, 1.0);
    if w < 0.5 {
        // cold: blue → white
        let t = w * 2.0;
        [t, t, 1.0]
    } else {
        // warm: white → red/orange/yellow
        let t = (w - 0.5) * 2.0;
        [1.0, 1.0 - t * 0.6, 1.0 - t]
    }
}
