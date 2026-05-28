use crate::puzzle::hints::HintTier;
use imgui::{Ui, WindowFlags};

pub struct Scale {
    pub su: f32,
    pub is_touch: bool,
}

impl Scale {
    pub fn from_screen(w: f32, h: f32, is_touch: bool) -> Self {
        let su = if is_touch {
            let area = (w * h).sqrt();
            let ref_area = (1920.0_f32 * 1080.0_f32).sqrt();
            (area / ref_area).clamp(0.45, 3.0) * 1.4
        } else {
            1.0
        };
        Scale { su, is_touch }
    }

    /// Convert design units to pixels.
    #[inline]
    pub fn su(&self, n: f32) -> f32 {
        n * self.su
    }

    // ── Spacing tokens ────────────────────────────────────────────────
    #[inline]
    pub fn gap_xxs(&self) -> f32 {
        self.su(2.0)
    }
    #[inline]
    pub fn gap_xs(&self) -> f32 {
        self.su(4.0)
    }
    #[inline]
    pub fn gap_sm(&self) -> f32 {
        self.su(8.0)
    }
    #[inline]
    pub fn gap_md(&self) -> f32 {
        self.su(12.0)
    }
    #[inline]
    pub fn gap_lg(&self) -> f32 {
        self.su(16.0)
    }
    #[inline]
    pub fn gap_xl(&self) -> f32 {
        self.su(24.0)
    }

    /// Standard window padding.
    #[inline]
    pub fn pad_w(&self) -> f32 {
        self.su(16.0)
    }

    /// Standard button height.
    #[inline]
    pub fn btn_h(&self) -> f32 {
        self.su(40.0)
    }

    /// Progress bar height.
    #[inline]
    pub fn bar_h(&self) -> f32 {
        self.su(24.0)
    }

    /// Full-width button filling the content area.
    pub fn btn_full(&self, ui: &Ui, label: &str, inner_w: f32, action: &mut bool) {
        if ui.button_with_size(label, [inner_w, self.btn_h()]) {
            *action = true;
        }
    }
}

// ── Layout helpers ─────────────────────────────────────────────────────────

pub fn hud_flags() -> WindowFlags {
    WindowFlags::NO_DECORATION
        | WindowFlags::NO_MOVE
        | WindowFlags::NO_BACKGROUND
        | WindowFlags::NO_INPUTS
        | WindowFlags::NO_SAVED_SETTINGS
}

pub fn popup_flags() -> WindowFlags {
    WindowFlags::NO_DECORATION
        | WindowFlags::NO_MOVE
        | WindowFlags::NO_SAVED_SETTINGS
        | WindowFlags::NO_SCROLLBAR
        | WindowFlags::NO_SCROLL_WITH_MOUSE
}

/// Gold section title + separator.
pub fn title(ui: &Ui, text: &str) {
    ui.text_colored([0.85, 0.62, 0.18, 1.0], text);
    let _s = ui.push_style_color(imgui::StyleColor::Separator, [0.68, 0.48, 0.12, 0.45]);
    ui.separator();
    drop(_s);
}

/// Bright body text.
#[inline]
pub fn text_body(ui: &Ui, text: &str) {
    ui.text_colored([0.88, 0.90, 0.96, 1.0], text);
}
/// Muted secondary text.
#[inline]
pub fn text_muted(ui: &Ui, text: &str) {
    ui.text_colored([0.50, 0.56, 0.68, 0.70], text);
}
/// Dim label text.
#[inline]
pub fn text_dim(ui: &Ui, text: &str) {
    ui.text_colored([0.36, 0.42, 0.56, 0.7], text);
}

/// Subtle separator line.
pub fn separator_dim(ui: &Ui) {
    let _s = ui.push_style_color(imgui::StyleColor::Separator, [0.20, 0.30, 0.45, 0.30]);
    ui.separator();
    drop(_s);
}

/// Vertical gap.
#[inline]
pub fn gap(ui: &Ui, h: f32) {
    ui.dummy([0.0, h]);
}

// ── Color helpers ─────────────────────────────────────────────────────────

pub fn diff_color(d: f32) -> [f32; 4] {
    match d {
        v if v < 0.25 => [0.18, 0.75, 0.50, 1.0],
        v if v < 0.50 => [0.75, 0.63, 0.18, 1.0],
        v if v < 0.75 => [0.75, 0.19, 0.29, 1.0],
        _ => [0.63, 0.18, 0.75, 1.0],
    }
}

pub fn diff_label_str(d: f32) -> &'static str {
    match d {
        v if v < 0.25 => "Easy",
        v if v < 0.50 => "Medium",
        v if v < 0.75 => "Hard",
        _ => "Expert",
    }
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}..", &s[..max])
    } else {
        s.to_string()
    }
}

pub fn hint_tier_label(tier: HintTier) -> &'static str {
    match tier {
        HintTier::None => "None",
        HintTier::WarmCold => "Warm/Cold",
        HintTier::AxisPlane => "Axis Plane",
        HintTier::GhostSnap => "Ghost Snap",
    }
}
