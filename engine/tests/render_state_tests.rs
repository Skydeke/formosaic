use formosaic_engine::architecture::models::mesh::Mesh;
use formosaic_engine::architecture::models::model::Model;
use formosaic_engine::opengl::shaders::render_state::RenderState;
use formosaic_engine::rendering::abstracted::irenderer::IRenderer;
use formosaic_engine::rendering::abstracted::processable::Processable;
use formosaic_engine::rendering::instances::camera::camera::Camera;
use std::cell::Cell;

struct DummyRenderer;

impl IRenderer for DummyRenderer {
    fn render(&mut self, _context: &formosaic_engine::architecture::scene::scene_context::SceneContext) {}
    fn finish(&mut self) {}
}

struct DummyModel {
    meshes: Vec<Mesh>,
}

impl DummyModel {
    fn new() -> Self {
        Self { meshes: Vec::new() }
    }
}

impl Model for DummyModel {
    fn render<T: Processable>(&self, _instance_state: &RenderState<T>, _mesh_idx: usize) {}
    fn bind_and_configure(&self, _mesh_idx: usize) {}
    fn unbind(&self, _mesh_idx: usize) {}
    fn delete(&mut self) {}
    fn get_lowest(&self) -> f32 { 0.0 }
    fn get_meshes(&self) -> &[Mesh] { &self.meshes }
    fn centroid(&self) -> Option<cgmath::Vector3<f32>> { None }
}

struct DummyInstance {
    model: DummyModel,
    processed: Cell<u32>,
}

impl DummyInstance {
    fn new() -> Self {
        Self { model: DummyModel::new(), processed: Cell::new(0) }
    }
}

impl Processable for DummyInstance {
    fn process(&mut self) {
        self.processed.set(self.processed.get() + 1);
    }

    fn get_model(&self) -> &impl Model {
        &self.model
    }
}

#[test]
fn render_state_flags_are_consistent() {
    let renderer = DummyRenderer;
    let camera = Camera::new();
    let instance = DummyInstance::new();

    let state = RenderState::new_preresolved(
        &renderer,
        &instance,
        &camera,
        0,
        None,
        false,
    );

    assert!(state.has_instance());
    assert_eq!(state.instance_mesh_idx(), 0);
    assert!(!state.has_vertex_colors());
    assert!(state.mesh_material().is_none());
}

#[test]
fn render_state_screenspace_has_no_instance() {
    let renderer = DummyRenderer;
    let state = RenderState::<DummyInstance>::new_screenspace(&renderer);

    assert!(!state.has_instance());
    assert!(state.camera_opt().is_none());
    assert_eq!(state.instance_mesh_idx(), -1);
}
