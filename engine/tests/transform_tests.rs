use cgmath::{
    InnerSpace, One, Quaternion, Rad, Rotation3, Vector3,
};
use formosaic_engine::architecture::scene::node::node::{Node, NodeBehavior};
use formosaic_engine::architecture::scene::node::transform::Transform;
use std::cell::RefCell;
use std::f32::consts::PI;
use std::rc::Rc;

fn assert_vec3_approx(a: Vector3<f32>, b: Vector3<f32>, eps: f32) {
    let diff = a - b;
    assert!(
        diff.x.abs() <= eps && diff.y.abs() <= eps && diff.z.abs() <= eps,
        "expected {b:?}, got {a:?}"
    );
}

fn assert_quat_approx(a: Quaternion<f32>, b: Quaternion<f32>, eps: f32) {
    let dot = a.s * b.s + a.v.dot(b.v);
    assert!(
        (dot.abs() - 1.0).abs() < eps || (a.s - b.s).abs() <= eps,
        "expected quaternion approx {b:?}, got {a:?}"
    );
}

#[test]
fn transform_default_is_identity() {
    let t = Transform::new();
    assert_vec3_approx(t.position, Vector3::new(0.0, 0.0, 0.0), 1e-6);
    assert_eq!(t.scale, Vector3::new(1.0, 1.0, 1.0));
    assert!(
        (t.rotation.s - 1.0).abs() < 1e-6,
        "identity quaternion should have s≈1"
    );
    assert!(t.parent.is_none());
}

#[test]
fn transform_from_position() {
    let t = Transform::from_position(Vector3::new(1.0, 2.0, 3.0));
    assert_vec3_approx(t.position, Vector3::new(1.0, 2.0, 3.0), 1e-6);
    assert_eq!(t.scale, Vector3::new(1.0, 1.0, 1.0));
}

#[test]
fn transform_setters_work() {
    let mut t = Transform::new();
    t.set_position(Vector3::new(10.0, 20.0, 30.0));
    t.set_scale(Vector3::new(2.0, 3.0, 4.0));
    t.set_rotation(Quaternion::from_angle_x(Rad(PI / 4.0)));

    assert_vec3_approx(t.position, Vector3::new(10.0, 20.0, 30.0), 1e-6);
    assert_vec3_approx(t.scale, Vector3::new(2.0, 3.0, 4.0), 1e-6);
    assert!((t.rotation.s - (PI / 8.0).cos()).abs() < 1e-5);
}

#[test]
fn transform_scale_by() {
    let mut t = Transform::new();
    t.set_scale(Vector3::new(2.0, 3.0, 4.0));
    t.scale_by(0.5);
    assert_vec3_approx(t.scale, Vector3::new(1.0, 1.5, 2.0), 1e-6);
}

#[test]
fn transform_add_transformation_combines() {
    let mut a = Transform::new();
    a.set_position(Vector3::new(1.0, 2.0, 3.0));
    a.set_scale(Vector3::new(2.0, 2.0, 2.0));

    let b = Transform::from_position(Vector3::new(10.0, 20.0, 30.0));

    a.add_transformation(&b);
    assert_vec3_approx(a.position, Vector3::new(11.0, 22.0, 33.0), 1e-6);
    assert_vec3_approx(a.scale, Vector3::new(2.0, 2.0, 2.0), 1e-6);
}

#[test]
fn transform_get_matrix_is_translation_rotation_scale() {
    let mut t = Transform::new();
    t.set_position(Vector3::new(1.0, 2.0, 3.0));
    let mat = t.get_matrix();
    let extracted_pos = Vector3::new(mat.w.x, mat.w.y, mat.w.z);
    assert_vec3_approx(extracted_pos, Vector3::new(1.0, 2.0, 3.0), 1e-6);
}

#[test]
fn transform_from_matrix_decomposes_correctly() {
    let mut t = Transform::new();
    t.set_position(Vector3::new(1.0, 2.0, 3.0));
    t.set_scale(Vector3::new(2.0, 3.0, 4.0));

    let mat = t.get_matrix();
    let decomposed = Transform::from_matrix(mat);

    assert_vec3_approx(decomposed.position, Vector3::new(1.0, 2.0, 3.0), 1e-4);
    assert_vec3_approx(decomposed.scale, Vector3::new(2.0, 3.0, 4.0), 1e-4);
}

#[test]
fn transform_look_at_points_to_target() {
    let mut t = Transform::new();
    t.set_position(Vector3::new(0.0, 0.0, 0.0));
    t.look_at(Vector3::new(0.0, 0.0, -1.0), Vector3::unit_y());

    let fwd = t.forward();
    let expected = Vector3::new(0.0, 0.0, -1.0);
    assert_vec3_approx(fwd, expected, 1e-5);
}

#[test]
fn transform_look_along_with_direction() {
    let mut t = Transform::new();
    t.look_along(Vector3::new(1.0, 0.0, 0.0), Vector3::unit_y());
    let fwd = t.forward();
    assert_vec3_approx(fwd, Vector3::new(1.0, 0.0, 0.0), 1e-5);
}

#[test]
fn transform_forward_and_up_with_rotation() {
    let mut t = Transform::new();
    t.look_at(Vector3::new(0.0, 0.0, -5.0), Vector3::unit_y());
    let fwd = t.forward();
    let up = t.up();
    assert!(fwd.magnitude() > 0.99);
    assert_vec3_approx(up, Vector3::unit_y(), 1e-5);
}

#[test]
fn transform_set_rotation_euler() {
    let mut t = Transform::new();
    t.set_rotation_euler(90.0, 0.0, 0.0);
    let fwd = t.forward();
    let up = t.up();
    assert_vec3_approx(up, Vector3::new(0.0, 0.0, 1.0), 1e-5);
    assert_vec3_approx(fwd, Vector3::new(0.0, 1.0, 0.0), 1e-5);
}

#[test]
fn transform_add_rotation_euler_local() {
    let mut t = Transform::new();
    t.add_rotation_euler_local(90.0, 0.0, 0.0);
    let up = t.up();
    assert_vec3_approx(up, Vector3::new(0.0, 0.0, 1.0), 1e-5);
}

#[test]
fn transform_add_rotation_euler_world() {
    let mut t = Transform::new();
    t.add_rotation_euler_world(0.0, 90.0, 0.0);
    let fwd = t.forward();
    assert_vec3_approx(fwd, Vector3::new(-1.0, 0.0, 0.0), 1e-5);
}

#[test]
fn transform_world_position_with_parent() {
    let parent: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));
    let child: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));

    parent
        .borrow_mut()
        .transform_mut()
        .set_position(Vector3::new(10.0, 20.0, 30.0));
    child
        .borrow_mut()
        .transform_mut()
        .set_position(Vector3::new(1.0, 2.0, 3.0));

    parent
        .borrow_mut()
        .add_child_impl(Rc::clone(&parent), Rc::clone(&child));

    let world_pos = child.borrow().transform().get_world_position();
    assert_vec3_approx(world_pos, Vector3::new(11.0, 22.0, 33.0), 1e-6);
}

#[test]
fn transform_world_rotation_combines_parent() {
    let parent: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));
    let child: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));

    let parent_rot = Quaternion::from_angle_y(Rad(PI / 2.0));
    parent.borrow_mut().transform_mut().set_rotation(parent_rot);

    parent
        .borrow_mut()
        .add_child_impl(Rc::clone(&parent), Rc::clone(&child));

    let world_rot = child.borrow().transform().get_world_rotation();
    let expected = parent_rot * Quaternion::one();
    assert_quat_approx(world_rot, expected, 1e-5);
}

#[test]
fn transform_world_scale_multiplies_parent() {
    let parent: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));
    let child: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));

    parent
        .borrow_mut()
        .transform_mut()
        .set_scale(Vector3::new(2.0, 3.0, 4.0));
    child
        .borrow_mut()
        .transform_mut()
        .set_scale(Vector3::new(0.5, 0.5, 0.5));

    parent
        .borrow_mut()
        .add_child_impl(Rc::clone(&parent), Rc::clone(&child));

    let world_scale = child.borrow().transform().get_world_scale();
    assert_vec3_approx(world_scale, Vector3::new(1.0, 1.5, 2.0), 1e-6);
}

#[test]
fn transform_rotate_around_world_changes_position() {
    let mut t = Transform::new();
    t.set_position(Vector3::new(1.0, 0.0, 0.0));

    let rot = Quaternion::from_angle_y(Rad(PI));
    t.rotate_around_world(Vector3::new(0.0, 0.0, 0.0), rot);

    assert_vec3_approx(t.position, Vector3::new(-1.0, 0.0, 0.0), 1e-5);
}

#[test]
fn transform_get_world_position_without_parent_is_local() {
    let mut t = Transform::new();
    t.set_position(Vector3::new(5.0, 10.0, 15.0));
    let world = t.get_world_position();
    assert_vec3_approx(world, Vector3::new(5.0, 10.0, 15.0), 1e-6);
}

#[test]
fn transform_parent_weak_ref_does_not_leak() {
    let parent: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));
    let child: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));

    parent
        .borrow_mut()
        .add_child_impl(Rc::clone(&parent), Rc::clone(&child));

    let parent_weak = child.borrow().transform().parent.clone();
    assert!(
        parent_weak.is_some(),
        "child should have a weak parent reference"
    );
    if let Some(weak) = &parent_weak {
        assert!(weak.upgrade().is_some(), "parent should be alive");
    }

    drop(parent);
    if let Some(weak) = &parent_weak {
        assert!(
            weak.upgrade().is_none(),
            "parent should be dropped, weak ref should fail"
        );
    }
}

#[test]
fn transform_matrix_decomposition_preserves_rotation() {
    let mut t = Transform::new();
    let rot = Quaternion::from_angle_y(Rad(PI / 3.0))
        * Quaternion::from_angle_x(Rad(PI / 6.0));
    t.set_rotation(rot);
    t.set_position(Vector3::new(1.0, 2.0, 3.0));
    t.set_scale(Vector3::new(2.0, 3.0, 4.0));

    let mat = t.get_matrix();
    let decomposed = Transform::from_matrix(mat);

    assert_vec3_approx(decomposed.position, Vector3::new(1.0, 2.0, 3.0), 1e-4);
    assert_vec3_approx(decomposed.scale, Vector3::new(2.0, 3.0, 4.0), 1e-4);

    let fwd_orig = t.forward();
    let fwd_dec = decomposed.forward();
    assert_vec3_approx(fwd_orig, fwd_dec, 1e-4);
}

#[test]
fn transform_add_transformation_rotation_ordering() {
    let mut t = Transform::new();
    t.set_rotation_euler(45.0, 0.0, 0.0);

    let other = Transform::new();
    t.add_transformation(&other);

    let fwd = t.forward();
    assert!(fwd.y > 0.0 && fwd.y < 1.0, "fwd.y should be positive but < 1 after 45° x-rotation, got {fwd:?}");
}
