use cgmath::Vector4;
use formosaic_engine::architecture::models::material::{AlphaMode, Material};
use formosaic_engine::architecture::models::mesh::Mesh;
use formosaic_engine::architecture::models::model::Model;
use formosaic_engine::opengl::shaders::render_state::RenderState;
use formosaic_engine::rendering::abstracted::irenderer::IRenderer;
use formosaic_engine::rendering::abstracted::processable::Processable;
use formosaic_engine::rendering::instances::camera::camera::Camera;

struct DummyRenderer;

impl IRenderer for DummyRenderer {
    fn render(&mut self, _context: &formosaic_engine::architecture::scene::scene_context::SceneContext) {}
    fn finish(&mut self) {}
}

struct DummyModel {
    meshes: Vec<Mesh>,
    materials: Vec<Option<Material>>,
    vertex_colors: Vec<bool>,
}

impl DummyModel {
    fn new() -> Self {
        Self {
            meshes: Vec::new(),
            materials: Vec::new(),
            vertex_colors: Vec::new(),
        }
    }

    fn with_material(mut self, mat: Option<Material>) -> Self {
        self.materials.push(mat);
        self.vertex_colors.push(false);
        self
    }

    fn with_vertex_color(mut self) -> Self {
        self.materials.push(None);
        self.vertex_colors.push(true);
        self
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

    fn get_material(&self, mesh_idx: usize) -> Option<&Material> {
        self.materials.get(mesh_idx).and_then(|m| m.as_ref())
    }

    fn has_vertex_colors(&self, mesh_idx: usize) -> bool {
        self.vertex_colors.get(mesh_idx).copied().unwrap_or(false)
    }
}

struct DummyInstance {
    model: DummyModel,
}

impl DummyInstance {
    fn new(model: DummyModel) -> Self {
        Self { model }
    }
}

impl Processable for DummyInstance {
    fn process(&mut self) {}

    fn get_model(&self) -> &impl Model {
        &self.model
    }
}

#[test]
fn render_state_with_material_reference() {
    let renderer = DummyRenderer;
    let camera = Camera::new();
    let model = DummyModel::new().with_material(Some(Material::new()));
    let instance = DummyInstance::new(model);

    let state = RenderState::new(
        &renderer,
        &instance,
        &camera,
        0,
    );

    assert!(state.mesh_material().is_some());
    assert!(!state.has_vertex_colors());
}

#[test]
fn render_state_with_vertex_colors() {
    let renderer = DummyRenderer;
    let camera = Camera::new();
    let model = DummyModel::new().with_vertex_color();
    let instance = DummyInstance::new(model);

    let state = RenderState::new(
        &renderer,
        &instance,
        &camera,
        0,
    );

    assert!(state.has_vertex_colors());
}

#[test]
fn render_state_material_none_for_empty_model() {
    let renderer = DummyRenderer;
    let camera = Camera::new();
    let model = DummyModel::new();
    let instance = DummyInstance::new(model);

    let state = RenderState::new(
        &renderer,
        &instance,
        &camera,
        0,
    );

    assert!(state.mesh_material().is_none());
}

#[test]
fn render_state_preresolved_material_lifetime() {
    let renderer = DummyRenderer;
    let camera = Camera::new();
    let model = DummyModel::new().with_material(Some(Material::new()));
    let instance = DummyInstance::new(model);

    let material = instance.model.get_material(0);

    let state = RenderState::new_preresolved(
        &renderer,
        &instance,
        &camera,
        0,
        material,
        false,
    );

    assert!(state.mesh_material().is_some());
    assert_eq!(state.instance_mesh_idx(), 0);
}

#[test]
fn render_state_preresolved_with_alpha_blend() {
    let renderer = DummyRenderer;
    let camera = Camera::new();
    let mat = Material::new()
        .with_diffuse_color(Vector4::new(1.0, 0.0, 0.0, 0.5))
        .with_emissive_color(Vector4::new(0.1, 0.1, 0.1, 1.0));
    let model = DummyModel::new().with_material(Some(mat));
    let instance = DummyInstance::new(model);

    let material = instance.model.get_material(0);
    let state = RenderState::new_preresolved(
        &renderer,
        &instance,
        &camera,
        0,
        material,
        false,
    );

    let mat_ref = state.mesh_material().unwrap();
    assert_eq!(mat_ref.alpha_mode, AlphaMode::Opaque);
    assert_eq!(mat_ref.diffuse_color.w, 0.5);
}

#[test]
fn render_state_preresolved_with_alpha_mask() {
    let renderer = DummyRenderer;
    let camera = Camera::new();

    let mut mat = Material::new();
    mat.alpha_mode = AlphaMode::Mask(0.3);

    let model = DummyModel::new().with_material(Some(mat));
    let instance = DummyInstance::new(model);

    let material = instance.model.get_material(0);
    let state = RenderState::new_preresolved(
        &renderer,
        &instance,
        &camera,
        0,
        material,
        true,
    );

    let mat_ref = state.mesh_material().unwrap();
    assert!(matches!(mat_ref.alpha_mode, AlphaMode::Mask(c) if (c - 0.3).abs() < 1e-5));
    assert!(state.has_vertex_colors());
}

#[test]
fn render_state_camera_accessor_returns_ref() {
    let renderer = DummyRenderer;
    let camera = Camera::new();
    let model = DummyModel::new();
    let instance = DummyInstance::new(model);

    let state = RenderState::new(
        &renderer,
        &instance,
        &camera,
        0,
    );

    let cam_ref = state.camera();
    assert_eq!(cam_ref.fov, 75.0_f32.to_radians());
}

#[test]
fn render_state_without_instance() {
    let renderer = DummyRenderer;
    let camera = Camera::new();

    let state = RenderState::<DummyInstance>::new_without_instance(&renderer, &camera);
    assert!(!state.has_instance());
    assert!(state.camera_opt().is_some());
    assert!(state.mesh_material().is_none());
    assert!(!state.has_vertex_colors());
}

#[test]
fn render_state_screenspace_no_camera() {
    let renderer = DummyRenderer;
    let state = RenderState::<DummyInstance>::new_screenspace(&renderer);

    assert!(!state.has_instance());
    assert!(state.camera_opt().is_none());
    assert_eq!(state.instance_mesh_idx(), -1);
}

#[test]
fn render_state_mesh_returns_none_by_default() {
    let renderer = DummyRenderer;
    let camera = Camera::new();
    let model = DummyModel::new();
    let instance = DummyInstance::new(model);

    let state = RenderState::new(
        &renderer,
        &instance,
        &camera,
        0,
    );

    assert!(state.mesh().is_none());
}
