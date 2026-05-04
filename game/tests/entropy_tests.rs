use cgmath::Vector3;
use formosaic::puzzle::entropy::{analyse_axis, difficulty_label};

#[test]
fn analyse_axis_uniform_offsets_has_max_entropy() {
    let offsets = vec![0.0f32; 9 * 12];
    let report = analyse_axis(&offsets, Vector3::unit_y());

    let expected = (64.0f32).log2();
    assert!((report.entropy_bits - expected).abs() < 1e-3);
    assert!(report.peak_score > 0.99);
}

#[test]
fn difficulty_label_maps_thresholds() {
    assert_eq!(difficulty_label(0.0), "Easy");
    assert_eq!(difficulty_label(0.24), "Easy");
    assert_eq!(difficulty_label(0.25), "Medium");
    assert_eq!(difficulty_label(0.49), "Medium");
    assert_eq!(difficulty_label(0.50), "Hard");
    assert_eq!(difficulty_label(0.74), "Hard");
    assert_eq!(difficulty_label(0.75), "Expert");
}
