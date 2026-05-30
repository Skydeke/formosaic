use cgmath::Vector3;
use formosaic::puzzle::hints::{HintSystem, HintTier};

// ── existing tests ────────────────────────────────────────────────────────────

#[test]
fn hint_tier_advances_and_caps() {
    let mut hints = HintSystem::new();
    assert_eq!(hints.tier(), HintTier::None);

    hints.advance();
    assert_eq!(hints.tier(), HintTier::WarmCold);
    hints.advance();
    assert_eq!(hints.tier(), HintTier::AxisPlane);
    hints.advance();
    assert_eq!(hints.tier(), HintTier::GhostSnap);
    hints.advance();
    assert_eq!(hints.tier(), HintTier::GhostSnap);
}

#[test]
fn hint_update_sets_warmth_and_disc_visibility() {
    let mut hints = HintSystem::new();
    let output = hints.update(0.1, Vector3::unit_z(), Vector3::unit_z());
    assert!(output.warmth > 0.99);
    assert!(!output.show_disc);

    hints.advance();
    hints.advance();
    let output = hints.update(0.1, Vector3::unit_z(), Vector3::unit_z());
    assert!(output.show_disc);
}

#[test]
fn ghost_snap_accumulates_lerp() {
    let mut hints = HintSystem::new();
    hints.advance();
    hints.advance();
    hints.advance();

    let first = hints.update(1.0, Vector3::unit_z(), Vector3::unit_z());
    let second = hints.update(1.0, Vector3::unit_z(), Vector3::unit_z());

    assert!(second.ghost_lerp > first.ghost_lerp);
}

// ── Bug fix: hint count shown as 1 even when no hint was used ────────────────

/// `reset_full()` (called at every level load) must zero hint_count so the HUD
/// does not carry over counts from a previous level.
#[test]
fn reset_full_clears_hint_count_across_levels() {
    let mut hints = HintSystem::new();

    // Simulate level 1: player uses two hints.
    hints.advance();
    hints.advance();
    assert_eq!(hints.hint_count(), 2, "count should be 2 after two advances");

    // Level 2 loads — should call reset_full(), not reset().
    hints.reset_full();
    assert_eq!(
        hints.hint_count(),
        0,
        "hint_count must be 0 at the start of a new level (reset_full was not called)"
    );
    assert_eq!(hints.tier(), HintTier::None, "tier must also reset to None");
}

/// Plain `reset()` preserves hint_count for solve-screen display — that is fine
/// for post-solve state, but must NOT be used at level load.
#[test]
fn reset_preserves_count_for_post_solve_display() {
    let mut hints = HintSystem::new();
    hints.advance();
    hints.advance();
    hints.advance();
    let count_before = hints.hint_count();

    hints.reset(); // called after solve animation (finish_restore)
    assert_eq!(
        hints.hint_count(),
        count_before,
        "reset() should preserve hint_count for post-solve score display"
    );
    assert_eq!(hints.tier(), HintTier::None, "tier should still reset");
}

/// Validates that a fresh HintSystem starts with zero count so the HUD never
/// shows "1 hint" before any hints are actually used.
#[test]
fn new_hint_system_starts_with_zero_count() {
    let hints = HintSystem::new();
    assert_eq!(hints.hint_count(), 0);
    assert_eq!(hints.tier(), HintTier::None);
}

// ── Bug fix: hint warmth precision near solution ──────────────────────────────

/// When the camera faces directly toward the solution axis the warmth must be
/// essentially 1.0 (HOT).
#[test]
fn warmth_is_max_when_camera_aligns_with_solution() {
    let mut hints = HintSystem::new();
    hints.advance(); // activate WarmCold tier
    let out = hints.update(0.016, Vector3::unit_z(), Vector3::unit_z());
    assert!(
        out.warmth > 0.99,
        "warmth should be ~1.0 when perfectly aligned, got {}",
        out.warmth
    );
}

/// When the camera is perpendicular to the solution axis the warmth should be
/// noticeably lower than when almost aligned, so the player feels a clear
/// gradient near the target.
#[test]
fn warmth_is_noticeably_lower_when_perpendicular_vs_aligned() {
    let mut hints = HintSystem::new();
    hints.advance();

    // Aligned
    let aligned = hints.update(0.016, Vector3::unit_z(), Vector3::unit_z());

    // ~15° off axis — still fairly close
    let close = hints.update(
        0.016,
        Vector3::new(15_f32.to_radians().sin(), 0.0, 15_f32.to_radians().cos()),
        Vector3::unit_z(),
    );

    // Perpendicular (90°)
    let perp = hints.update(0.016, Vector3::unit_x(), Vector3::unit_z());

    assert!(
        aligned.warmth > close.warmth,
        "aligned ({}) should be warmer than 15° off ({})",
        aligned.warmth,
        close.warmth
    );
    assert!(
        close.warmth > perp.warmth,
        "15° off ({}) should be warmer than perpendicular ({})",
        close.warmth,
        perp.warmth
    );
    // The near-solution gradient must be steep — aligned must be ≥ 10% warmer
    // than the 15° case so the player gets precise feedback close to the target.
    assert!(
        (aligned.warmth - close.warmth) >= 0.05,
        "warmth gradient near solution is too flat: aligned={}, 15°={}",
        aligned.warmth,
        close.warmth
    );
}

/// Both poles of the solution axis (+Z and −Z) should read as equally warm,
/// because the puzzle solution is symmetric about the axis.
#[test]
fn warmth_is_symmetric_for_both_poles() {
    let mut hints = HintSystem::new();
    hints.advance();

    let forward = hints.update(0.016, Vector3::unit_z(), Vector3::unit_z());
    let backward = hints.update(0.016, -Vector3::unit_z(), Vector3::unit_z());

    assert!(
        (forward.warmth - backward.warmth).abs() < 1e-4,
        "both poles should give identical warmth: forward={} backward={}",
        forward.warmth,
        backward.warmth
    );
}
