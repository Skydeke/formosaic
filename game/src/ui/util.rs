use imgui::WindowFlags;

pub fn hud_flags() -> WindowFlags {
    WindowFlags::NO_DECORATION
        | WindowFlags::NO_MOVE
        | WindowFlags::NO_BACKGROUND
        | WindowFlags::NO_INPUTS
        | WindowFlags::NO_SAVED_SETTINGS
}

pub fn diff_color(d: f32) -> [f32; 4] {
    match d {
        v if v < 0.25 => [0.18, 0.75, 0.50, 1.0],
        v if v < 0.50 => [0.75, 0.63, 0.18, 1.0],
        v if v < 0.75 => [0.75, 0.19, 0.29, 1.0],
        _             => [0.63, 0.18, 0.75, 1.0],
    }
}

pub fn diff_label_str(d: f32) -> &'static str {
    match d {
        v if v < 0.25 => "Easy",
        v if v < 0.50 => "Medium",
        v if v < 0.75 => "Hard",
        _             => "Expert",
    }
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.len() > max { format!("{}..", &s[..max]) } else { s.to_string() }
}
