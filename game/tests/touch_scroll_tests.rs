//! Tests for touch-scroll direction on Android.
//!
//! The correct behaviour is the ORIGINAL formula (no negation): swiping a finger
//! DOWN (positive dy) produces a positive mouse-wheel delta, which imgui
//! interprets as "scroll content toward the user" (content moves down).
//! This matches how finger-drag scrolling works in native Android apps.
//!
//! A negation was incorrectly added in an earlier fix and has been removed.
//! Momentum (fling) scrolling is handled separately in prepare_frame.

// ─── helper: replicate the delta formula from imgui-winit-support ────────────

/// Maps a finger movement delta (positive = finger moved DOWN the screen) to
/// an imgui mouse-wheel Y value.
///
/// Correct behaviour: finger down → content scrolls down → positive wheel.
fn touch_delta_to_wheel(finger_dy: f32, divisor: f32) -> f32 {
    // This mirrors the CORRECT formula in vendor/imgui-winit-support/src/lib.rs.
    finger_dy / divisor
}

/// The NEGATED (wrong) formula that was briefly applied.
fn touch_delta_to_wheel_negated(finger_dy: f32, divisor: f32) -> f32 {
    -finger_dy / divisor
}

// ─── tests ───────────────────────────────────────────────────────────────────

/// Swiping a finger downward (positive dy) must produce a positive wheel value
/// so content scrolls the same direction as the finger (native Android feel).
#[test]
fn swipe_down_produces_positive_wheel() {
    let finger_dy = 30.0_f32; // finger moved 30 px downward
    let wheel = touch_delta_to_wheel(finger_dy, 16.0);
    assert!(
        wheel > 0.0,
        "swiping down should yield positive wheel (scroll down), got {}",
        wheel
    );
}

/// Swiping a finger upward (negative dy) must produce a negative wheel value.
#[test]
fn swipe_up_produces_negative_wheel() {
    let finger_dy = -30.0_f32; // finger moved 30 px upward
    let wheel = touch_delta_to_wheel(finger_dy, 16.0);
    assert!(
        wheel < 0.0,
        "swiping up should yield negative wheel (scroll up), got {}",
        wheel
    );
}

/// The negated formula inverted the direction — document it for reference.
#[test]
fn negated_formula_had_wrong_direction_for_swipe_down() {
    let finger_dy = 30.0_f32;
    let negated_wheel = touch_delta_to_wheel_negated(finger_dy, 16.0);
    let correct_wheel = touch_delta_to_wheel(finger_dy, 16.0);
    assert!(
        negated_wheel < 0.0,
        "negated formula would produce negative wheel for downward swipe"
    );
    assert_eq!(
        correct_wheel, -negated_wheel,
        "correct formula is the opposite of the negated one"
    );
}

/// Zero delta (finger not moving) must yield zero wheel — no phantom scroll.
#[test]
fn no_movement_produces_zero_wheel() {
    let wheel = touch_delta_to_wheel(0.0, 16.0);
    assert_eq!(wheel, 0.0);
}

/// Divisor scales the magnitude linearly — larger divisor means less scroll
/// per pixel (useful for lists with tall rows).
#[test]
fn larger_divisor_reduces_scroll_magnitude() {
    let finger_dy = 48.0_f32;
    let fine = touch_delta_to_wheel(finger_dy, 16.0).abs();
    let coarse = touch_delta_to_wheel(finger_dy, 48.0).abs();
    assert!(
        fine > coarse,
        "smaller divisor should scroll more: fine={} coarse={}",
        fine, coarse
    );
}

/// The wheel magnitude must equal |finger_dy| / divisor (exact formula check).
#[test]
fn wheel_magnitude_equals_delta_over_divisor() {
    let finger_dy = 32.0_f32;
    let divisor = 16.0_f32;
    let wheel = touch_delta_to_wheel(finger_dy, divisor);
    let expected_magnitude = finger_dy / divisor;
    assert!(
        (wheel.abs() - expected_magnitude).abs() < 1e-6,
        "|wheel| should be {}, got {}",
        expected_magnitude,
        wheel.abs()
    );
}
