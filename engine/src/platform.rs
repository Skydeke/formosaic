//! Platform abstraction layer.
//!
//! Centralises all platform-specific checks (`cfg!(target_os = "android")`) so
//! that adding new platforms (iOS, Web, etc.) requires touching only this file.

/// Read-only platform information, set once at startup.
///
/// Pass this struct instead of scattering `cfg!()` checks throughout the
/// codebase — adding a new platform is then a single-site change.
#[derive(Clone, Copy, Debug)]
pub struct PlatformInfo {
    /// True when the primary input is touch (Android, iOS, etc.).
    pub is_touch: bool,

    /// UI scale factor derived from screen DPI / size.
    pub ui_scale: f32,
}

impl PlatformInfo {
    /// Detect the current platform at compile-time.
    pub fn detect() -> Self {
        Self {
            is_touch: cfg!(target_os = "android"),
            ui_scale: 1.0,
        }
    }

    /// Override the UI scale (called after DPI info is available).
    pub fn with_ui_scale(mut self, scale: f32) -> Self {
        self.ui_scale = scale;
        self
    }

    /// Convenience: true when `is_touch` is set.
    pub fn is_touch(&self) -> bool {
        self.is_touch
    }
}
