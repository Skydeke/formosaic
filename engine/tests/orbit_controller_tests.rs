use cgmath::{InnerSpace, Vector3};
use formosaic_engine::input::{Event, Key};
use formosaic_engine::rendering::instances::camera::camera_controller::CameraController;
use formosaic_engine::rendering::instances::camera::orbit_controller::OrbitController;

fn assert_vec3_approx(a: Vector3<f32>, b: Vector3<f32>, eps: f32) {
    let diff = a - b;
    assert!(
        diff.x.abs() <= eps && diff.y.abs() <= eps && diff.z.abs() <= eps,
        "expected {b:?}, got {a:?}"
    );
}

#[test]
fn orbit_controller_defaults() {
    let ctrl = OrbitController::new(Vector3::new(0.0, 0.0, 0.0), 10.0);
    assert_eq!(ctrl.sensitivity, 1.5);
    assert_vec3_approx(ctrl.target, Vector3::new(0.0, 0.0, 0.0), 1e-6);
    assert!((ctrl.distance - 10.0).abs() < 1e-6);
}

#[test]
fn orbit_controller_set_initial_position_calculates_distance() {
    let mut ctrl = OrbitController::new(Vector3::new(0.0, 0.0, 0.0), 1.0);
    let start = Vector3::new(0.0, 0.0, 5.0);
    ctrl.set_initial_position(start);
    assert!((ctrl.distance - 5.0).abs() < 1e-6);
}

#[test]
fn orbit_controller_set_initial_position_clamps_minimum_distance() {
    let mut ctrl = OrbitController::new(Vector3::new(0.0, 0.0, 0.0), 1.0);
    ctrl.set_initial_position(Vector3::new(0.0, 0.0, 0.0));
    assert!(ctrl.distance >= 0.001);
}

#[test]
fn orbit_controller_mouse_down_starts_dragging() {
    let mut ctrl = OrbitController::new(Vector3::new(0.0, 0.0, 0.0), 10.0);
    ctrl.handle_event(
        &Event::MouseDown {
            x: 100.0,
            y: 200.0,
            width: 800.0,
            height: 600.0,
        },
        800.0,
        600.0,
    );
}

#[test]
fn orbit_controller_mouse_move_accumulates_delta_while_dragging() {
    let mut ctrl = OrbitController::new(Vector3::new(0.0, 0.0, 0.0), 10.0);
    ctrl.handle_event(
        &Event::MouseDown {
            x: 400.0,
            y: 300.0,
            width: 800.0,
            height: 600.0,
        },
        800.0,
        600.0,
    );
    ctrl.handle_event(
        &Event::MouseMove {
            x: 500.0,
            y: 350.0,
            width: 800.0,
            height: 600.0,
        },
        800.0,
        600.0,
    );
}

#[test]
fn orbit_controller_mouse_up_stops_dragging() {
    let mut ctrl = OrbitController::new(Vector3::new(0.0, 0.0, 0.0), 10.0);
    ctrl.handle_event(
        &Event::MouseDown {
            x: 400.0,
            y: 300.0,
            width: 800.0,
            height: 600.0,
        },
        800.0,
        600.0,
    );
    ctrl.handle_event(
        &Event::MouseUp {
            x: 500.0,
            y: 350.0,
            width: 800.0,
            height: 600.0,
        },
        800.0,
        600.0,
    );
    ctrl.handle_event(
        &Event::MouseMove {
            x: 600.0,
            y: 400.0,
            width: 800.0,
            height: 600.0,
        },
        800.0,
        600.0,
    );
}

#[test]
fn orbit_controller_touch_events_work_like_mouse() {
    let mut ctrl = OrbitController::new(Vector3::new(0.0, 0.0, 0.0), 10.0);
    ctrl.handle_event(
        &Event::TouchDown {
            id: 0,
            x: 400.0,
            y: 300.0,
            width: 800.0,
            height: 600.0,
        },
        800.0,
        600.0,
    );
    ctrl.handle_event(
        &Event::TouchMove {
            id: 0,
            x: 500.0,
            y: 350.0,
            width: 800.0,
            height: 600.0,
        },
        800.0,
        600.0,
    );
    ctrl.handle_event(&Event::TouchUp { id: 0 }, 800.0, 600.0);
}

#[test]
fn orbit_controller_non_input_events_are_ignored() {
    let mut ctrl = OrbitController::new(Vector3::new(0.0, 0.0, 0.0), 10.0);
    ctrl.handle_event(&Event::KeyDown { key: Key::Escape }, 800.0, 600.0);
    ctrl.handle_event(&Event::Quit, 800.0, 600.0);
}

#[test]
fn orbit_controller_move_without_down_does_not_accumulate() {
    let mut ctrl = OrbitController::new(Vector3::new(0.0, 0.0, 0.0), 10.0);
    ctrl.handle_event(
        &Event::MouseMove {
            x: 500.0,
            y: 350.0,
            width: 800.0,
            height: 600.0,
        },
        800.0,
        600.0,
    );
}

#[test]
fn orbit_controller_snap_to_direction_places_camera_correctly() {
    let mut ctrl = OrbitController::new(Vector3::new(0.0, 0.0, 0.0), 5.0);
    let mut transform = formosaic_engine::architecture::scene::node::transform::Transform::new();
    transform.position = Vector3::new(0.0, 0.0, 5.0);

    let dir = Vector3::unit_z();
    ctrl.snap_to_direction(&mut transform, dir);

    let offset = transform.position - ctrl.target;
    assert!(
        (offset.magnitude() - 5.0).abs() < 1e-4,
        "camera should be at orbit distance from target"
    );

    let camera_fwd = transform.forward().normalize();
    assert!(
        (camera_fwd.dot(dir) - 1.0).abs() < 0.01,
        "camera should look at target along solution direction"
    );
}

#[test]
fn orbit_controller_control_applies_rotation() {
    let mut ctrl = OrbitController::new(Vector3::new(0.0, 0.0, 0.0), 10.0);
    let mut transform = formosaic_engine::architecture::scene::node::transform::Transform::new();
    transform.position = Vector3::new(0.0, 0.0, 10.0);

    ctrl.set_initial_position(transform.position);

    ctrl.handle_event(
        &Event::MouseDown {
            x: 400.0,
            y: 300.0,
            width: 800.0,
            height: 600.0,
        },
        800.0,
        600.0,
    );
    ctrl.handle_event(
        &Event::MouseMove {
            x: 500.0,
            y: 300.0,
            width: 800.0,
            height: 600.0,
        },
        800.0,
        600.0,
    );

    CameraController::control(&mut ctrl, &mut transform);

    assert_ne!(transform.position, Vector3::new(0.0, 0.0, 10.0));
}

#[test]
fn orbit_controller_clamps_pitch_to_avoid_flip() {
    let mut ctrl = OrbitController::new(Vector3::new(0.0, 0.0, 0.0), 10.0);
    let mut transform = formosaic_engine::architecture::scene::node::transform::Transform::new();
    transform.position = Vector3::new(0.0, 0.0, 10.0);

    ctrl.set_initial_position(transform.position);

    ctrl.handle_event(
        &Event::MouseDown {
            x: 400.0,
            y: 300.0,
            width: 800.0,
            height: 600.0,
        },
        800.0,
        600.0,
    );
    ctrl.handle_event(
        &Event::MouseMove {
            x: 400.0,
            y: -1000.0,
            width: 800.0,
            height: 600.0,
        },
        800.0,
        600.0,
    );

    CameraController::control(&mut ctrl, &mut transform);

    let up = transform.up();
    assert!(
        up.y > -1.0 && up.y < 1.0,
        "pitch should be clamped, got up vector {up:?}"
    );
}

#[test]
fn orbit_controller_distance_preserved_after_rotation() {
    let mut ctrl = OrbitController::new(Vector3::new(0.0, 0.0, 0.0), 7.5);
    let mut transform = formosaic_engine::architecture::scene::node::transform::Transform::new();
    transform.position = Vector3::new(0.0, 0.0, 7.5);
    ctrl.set_initial_position(transform.position);

    ctrl.handle_event(
        &Event::MouseDown {
            x: 400.0,
            y: 300.0,
            width: 800.0,
            height: 600.0,
        },
        800.0,
        600.0,
    );
    ctrl.handle_event(
        &Event::MouseMove {
            x: 600.0,
            y: 400.0,
            width: 800.0,
            height: 600.0,
        },
        800.0,
        600.0,
    );
    CameraController::control(&mut ctrl, &mut transform);

    let dist = (transform.position - ctrl.target).magnitude();
    assert!(
        (dist - 7.5).abs() < 0.01,
        "orbit radius should be preserved, got {dist}"
    );
}

#[test]
fn orbit_controller_clears_delta_after_control() {
    let mut ctrl = OrbitController::new(Vector3::new(0.0, 0.0, 0.0), 10.0);
    let mut transform = formosaic_engine::architecture::scene::node::transform::Transform::new();
    transform.position = Vector3::new(0.0, 0.0, 10.0);

    ctrl.handle_event(
        &Event::MouseDown {
            x: 400.0,
            y: 300.0,
            width: 800.0,
            height: 600.0,
        },
        800.0,
        600.0,
    );
    ctrl.handle_event(
        &Event::MouseMove {
            x: 500.0,
            y: 350.0,
            width: 800.0,
            height: 600.0,
        },
        800.0,
        600.0,
    );
    CameraController::control(&mut ctrl, &mut transform);

    let pos_after_first = transform.position;

    CameraController::control(&mut ctrl, &mut transform);
    let pos_after_second = transform.position;

    assert_vec3_approx(pos_after_first, pos_after_second, 1e-5);
}

#[test]
fn orbit_controller_normalized_axis_is_unit_length() {
    let mut ctrl = OrbitController::new(Vector3::new(1.0, 2.0, 3.0), 10.0);
    let dir = Vector3::new(1.0, 1.0, 1.0).normalize();
    let mut transform = formosaic_engine::architecture::scene::node::transform::Transform::new();
    transform.position = Vector3::new(1.0, 2.0, 13.0);

    ctrl.snap_to_direction(&mut transform, dir);
    let offset = (transform.position - ctrl.target).normalize();
    assert!((offset.magnitude() - 1.0).abs() < 1e-5);
}

#[test]
fn orbit_controller_retarget_preserves_camera_offset() {
    let mut ctrl = OrbitController::new(Vector3::new(1.0, 2.0, 3.0), 5.0);
    let mut transform = formosaic_engine::architecture::scene::node::transform::Transform::new();
    transform.position = Vector3::new(1.0, 2.0, 8.0);

    ctrl.set_target_preserve_offset(&mut transform, Vector3::new(4.0, 5.0, 6.0));

    assert_vec3_approx(ctrl.target, Vector3::new(4.0, 5.0, 6.0), 1e-6);
    assert_vec3_approx(transform.position, Vector3::new(4.0, 5.0, 11.0), 1e-6);
    assert!((ctrl.distance - 5.0).abs() < 1e-6);
}
