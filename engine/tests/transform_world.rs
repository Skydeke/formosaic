use cgmath::Vector3;
use formosaic_engine::architecture::scene::node::node::{Node, NodeBehavior};
use std::cell::RefCell;
use std::rc::Rc;

fn assert_vec3_approx(a: Vector3<f32>, b: Vector3<f32>, eps: f32) {
    let diff = a - b;
    assert!(
        diff.x.abs() <= eps && diff.y.abs() <= eps && diff.z.abs() <= eps,
        "expected {b:?}, got {a:?}"
    );
}

#[test]
fn child_world_position_includes_parent() {
    let parent: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));
    let child: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));

    parent
        .borrow_mut()
        .transform_mut()
        .set_position(Vector3::new(1.0, 2.0, 3.0));
    child
        .borrow_mut()
        .transform_mut()
        .set_position(Vector3::new(4.0, 5.0, 6.0));

    parent
        .borrow_mut()
        .add_child_impl(Rc::clone(&parent), Rc::clone(&child));

    let world_pos = child.borrow().transform().get_world_position();
    assert_vec3_approx(world_pos, Vector3::new(5.0, 7.0, 9.0), 1e-6);
}

#[test]
fn child_world_scale_multiplies_parent() {
    let parent: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));
    let child: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));

    parent
        .borrow_mut()
        .transform_mut()
        .set_scale(Vector3::new(2.0, 3.0, 4.0));
    child
        .borrow_mut()
        .transform_mut()
        .set_scale(Vector3::new(0.5, 0.25, 2.0));

    parent
        .borrow_mut()
        .add_child_impl(Rc::clone(&parent), Rc::clone(&child));

    let world_scale = child.borrow().transform().get_world_scale();
    assert_vec3_approx(world_scale, Vector3::new(1.0, 0.75, 8.0), 1e-6);
}
