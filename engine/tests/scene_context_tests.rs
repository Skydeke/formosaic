use formosaic_engine::architecture::scene::node::node::{Node, NodeBehavior};
use formosaic_engine::architecture::scene::node::scenegraph::Scenegraph;
use formosaic_engine::opengl::objects::clip_plane::ClipPlane;
use formosaic_engine::rendering::render_state::LightConfig;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn light_config_defaults_are_reasonable() {
    let lc = LightConfig::default();
    assert_eq!(lc.clear_color, [0.02, 0.02, 0.03]);
    assert_eq!(lc.sun_dir, [0.6, 0.9, 0.5]);
    assert_eq!(lc.sun_color, [0.85, 0.78, 0.62]);
    assert_eq!(lc.sky_color, [0.25, 0.35, 0.60]);
    assert!((lc.ambient_min - 0.14).abs() < 1e-6);
}

#[test]
fn light_config_copy_works() {
    let lc = LightConfig::default();
    let lc2 = lc;
    assert_eq!(lc.clear_color, lc2.clear_color);
    assert_eq!(lc.sun_color, lc2.sun_color);
}

#[test]
fn light_config_clone_works() {
    let lc = LightConfig::default();
    let lc2 = lc.clone();
    assert_eq!(lc.sun_dir, lc2.sun_dir);
}

#[test]
fn light_config_debug_repr() {
    let lc = LightConfig::default();
    let repr = format!("{:?}", lc);
    assert!(repr.contains("LightConfig"));
    assert!(repr.contains("clear_color"));
}

#[test]
fn clip_plane_none_is_zero() {
    let cp = ClipPlane::NONE;
    let v = cp.get_value();
    assert_eq!(v.x, 0.0);
    assert_eq!(v.y, 0.0);
    assert_eq!(v.z, 0.0);
    assert_eq!(v.w, 0.0);
}

#[test]
fn clip_plane_of_sets_coefficients() {
    let cp = ClipPlane::of(1.0, 2.0, 3.0, 4.0);
    let v = cp.get_value();
    assert_eq!(v.x, 1.0);
    assert_eq!(v.y, 2.0);
    assert_eq!(v.z, 3.0);
    assert_eq!(v.w, 4.0);
}

#[test]
fn clip_plane_of_above_creates_y_positive_plane() {
    let cp = ClipPlane::of_above(5.0);
    let v = cp.get_value();
    assert_eq!(v.x, 0.0);
    assert_eq!(v.y, 1.0);
    assert_eq!(v.z, 0.0);
    assert_eq!(v.w, -5.0);
}

#[test]
fn clip_plane_of_below_creates_y_negative_plane() {
    let cp = ClipPlane::of_below(3.0);
    let v = cp.get_value();
    assert_eq!(v.x, 0.0);
    assert_eq!(v.y, -1.0);
    assert_eq!(v.z, 0.0);
    assert_eq!(v.w, 3.0);
}

#[test]
fn clip_plane_copy_and_clone_work() {
    let cp = ClipPlane::of(1.0, 0.0, 0.0, -2.0);
    let cp2 = cp;
    let cp3 = cp.clone();
    assert_eq!(cp.get_value(), cp2.get_value());
    assert_eq!(cp.get_value(), cp3.get_value());
}

#[test]
fn clip_plane_debug_repr() {
    let cp = ClipPlane::of_above(10.0);
    let repr = format!("{:?}", cp);
    assert!(repr.contains("ClipPlane"));
}

#[test]
fn scene_context_default_impl() {
    let ctx = formosaic_engine::architecture::scene::scene_context::SceneContext::default();
    assert!(ctx.scene().is_some());
}

#[test]
fn scene_context_clip_plane_getter_setter() {
    let mut ctx = formosaic_engine::architecture::scene::scene_context::SceneContext::new();
    assert_eq!(
        ctx.get_clip_plane().get_value(),
        ClipPlane::NONE.get_value()
    );

    let cp = ClipPlane::of_above(10.0);
    ctx.set_clip_plane(cp);
    assert_eq!(ctx.clip_plane().get_value(), cp.get_value());
}

#[test]
fn scene_context_light_config_is_default() {
    let ctx = formosaic_engine::architecture::scene::scene_context::SceneContext::new();
    let default_lc = LightConfig::default();
    assert_eq!(ctx.lights.clear_color, default_lc.clear_color);
    assert_eq!(ctx.lights.sun_dir, default_lc.sun_dir);
}

#[test]
fn scene_context_camera_is_always_available() {
    let ctx = formosaic_engine::architecture::scene::scene_context::SceneContext::new();
    let _cam = ctx.get_camera();
    let _cam_ref: &std::cell::RefCell<
        formosaic_engine::rendering::instances::camera::camera::Camera,
    > = ctx.camera();
}

#[test]
fn scene_context_set_resolution() {
    use cgmath::Vector2;
    let mut ctx = formosaic_engine::architecture::scene::scene_context::SceneContext::new();
    ctx.set_resolution(Vector2::new(1920, 1080));
}

#[test]
fn scene_context_update_does_not_panic() {
    let mut ctx = formosaic_engine::architecture::scene::scene_context::SceneContext::new();
    ctx.update();
}

#[test]
fn scenegraph_add_and_clear() {
    let graph = Scenegraph::new();
    let node: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));
    graph.add_node(node);

    let count_before = graph.collect_nodes_of_type::<Node>().len();
    assert!(count_before >= 2);

    graph.clear();
    let count_after = graph.collect_nodes_of_type::<Node>().len();
    assert_eq!(count_after, 1, "clear should leave only root");
}

#[test]
fn scenegraph_clear_preserves_new_root() {
    let graph = Scenegraph::new();
    let node: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));
    graph.add_node(node);
    graph.clear();

    let node2: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));
    graph.add_node(node2);
    assert_eq!(graph.collect_nodes_of_type::<Node>().len(), 2);
}

#[test]
fn scene_context_set_and_read_deltas() {
    let mut ctx = formosaic_engine::architecture::scene::scene_context::SceneContext::new();
    ctx.delta_time = 1.0 / 60.0;
    assert!((ctx.delta_time - 1.0 / 60.0).abs() < 1e-6);
}

#[test]
fn scene_context_platform_detected() {
    let ctx = formosaic_engine::architecture::scene::scene_context::SceneContext::new();
    let detected = cfg!(target_os = "android");
    assert_eq!(ctx.platform.is_touch(), detected);
}

#[test]
fn node_children_slice_reference() {
    let node: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));
    let child: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));

    node.borrow_mut()
        .add_child_impl(Rc::clone(&node), Rc::clone(&child));

    let binding = node.borrow();
    let slice = binding.children();
    assert_eq!(slice.len(), 1);
}

#[test]
fn node_debug_name_format() {
    let node = Node::new();
    let name = node.get_name();
    assert!(name.starts_with("Node#"));
}
