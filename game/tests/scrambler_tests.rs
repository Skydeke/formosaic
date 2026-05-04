use cgmath::{InnerSpace, Vector3};
use formosaic::puzzle::scrambler::make_scrambled_orbit;

fn angle_between(a: Vector3<f32>, b: Vector3<f32>) -> f32 {
    let dot = a.normalize().dot(b.normalize()).clamp(-1.0, 1.0);
    dot.acos()
}

#[test]
fn scrambled_orbit_starts_far_from_solution_axis() {
    let target = Vector3::new(0.0, 0.0, 0.0);
    let distance = 10.0;
    let solution_dir = Vector3::unit_y();

    let (_ctrl, camera_pos) = make_scrambled_orbit(target, distance, solution_dir);
    let camera_dir = (camera_pos - target).normalize();

    let angle = angle_between(camera_dir, solution_dir);
    assert!(angle >= std::f32::consts::FRAC_PI_3);
}
