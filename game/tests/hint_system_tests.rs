use cgmath::Vector3;
use formosaic::puzzle::hints::{HintSystem, HintTier};

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
