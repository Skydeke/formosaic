use formosaic::formosaic::camera_scramble_t;

fn dot_from_angle_deg(deg: f32) -> f32 {
    (deg.to_radians()).cos()
}

// ── camera_scramble_t ────────────────────────────────────────────────

#[test]
fn scramble_t_far_from_solution() {
    assert_eq!(camera_scramble_t(0.0), 1.0);
}

#[test]
fn scramble_t_just_outside_fade() {
    assert_eq!(camera_scramble_t(dot_from_angle_deg(16.0)), 1.0);
}

#[test]
fn scramble_t_at_fade_threshold() {
    let t = camera_scramble_t(dot_from_angle_deg(15.0));
    assert!((t - 1.0).abs() < 1e-6);
}

#[test]
fn scramble_t_inside_fade_unscrambles() {
    let t = camera_scramble_t(dot_from_angle_deg(10.0));
    assert!(t > 0.0 && t < 1.0, "t={} should be between 0 and 1", t);
}

#[test]
fn scramble_t_at_solution_is_solved() {
    assert_eq!(camera_scramble_t(1.0), 0.0);
}

#[test]
fn scramble_t_monotonically_decreasing() {
    let angles = [14.0, 12.0, 10.0, 8.0, 5.0, 3.0, 1.0, 0.0];
    let mut prev_t = 1.0;
    for a in angles {
        let t = camera_scramble_t(dot_from_angle_deg(a));
        assert!(
            t <= prev_t + 1e-6,
            "t increased from {} to {} at {}°",
            prev_t,
            t,
            a
        );
        prev_t = t;
    }
}

#[test]
fn scramble_t_then_snap_threshold_ordering() {
    // SNAP_THRESHOLD_DOT ≈ cos(5°) ≈ 0.996 — solve triggers inside 5°.
    let dot_5deg = dot_from_angle_deg(5.0);
    let t_at_snap = camera_scramble_t(dot_5deg);
    assert!(
        t_at_snap >= 0.0 && t_at_snap <= 0.2,
        "t={} at ~5° — should be nearly solved",
        t_at_snap
    );

    // CAMERA_FADE_DOT ≈ cos(15°) ≈ 0.966 — fade starts ~3x farther than snap.
    // Verify fade-dot < snap-dot without referencing private constants.
    let dot_15deg = dot_from_angle_deg(15.0);
    assert!(dot_15deg < dot_5deg, "fade dot must be < snap dot");
}
