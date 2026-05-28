use cgmath::Matrix4;

use crate::{
    architecture::models::{material::Material, mesh::Mesh},
    rendering::{
        abstracted::{irenderer::IRenderer, processable::Processable},
        instances::camera::camera::Camera,
    },
};

/// Pre-resolved model data per draw call — populated at draw-call build time
/// from a safe `RefCell::borrow()` scope, then read by uniform extractors
/// during rendering. This avoids the need for `Processable::get_model()`
/// (which previously required unsafe pointer escape from behind RefCell).
pub struct ModelRenderData {
    pub mesh_transform: Matrix4<f32>,
    pub is_skinned: bool,
    pub bone_matrices: Vec<Matrix4<f32>>,
    pub bone_count: i32,
}

pub struct RenderState<'a, T: Processable> {
    renderer: &'a dyn IRenderer,
    instance: Option<&'a T>,
    camera: Option<&'a Camera>,
    #[allow(dead_code)]
    instance_mesh_idx: i32,
    material: Option<&'a Material>,
    has_vertex_colors: bool,
    /// Pre-resolved model data from a safe borrow scope — read by uniform
    /// extractors instead of calling `instance.get_model()`.
    model_data: Option<&'a ModelRenderData>,
}

impl<'a, T: Processable> RenderState<'a, T> {
    /// Primary constructor used by EntityRenderer.
    /// All model data is pre-resolved from a shared borrow of the model,
    /// before `borrow_mut()` is taken for rendering. This avoids the
    /// aliased-borrow problem that required `SimpleEntity`'s unsafe pattern.
    pub fn new_preresolved(
        renderer: &'a dyn IRenderer,
        instance: &'a T,
        camera: &'a Camera,
        instance_mesh_idx: usize,
        material: Option<&'a Material>,
        has_vertex_colors: bool,
        model_data: &'a ModelRenderData,
    ) -> Self {
        Self {
            renderer,
            instance: Some(instance),
            camera: Some(camera),
            instance_mesh_idx: instance_mesh_idx as i32,
            material,
            has_vertex_colors,
            model_data: Some(model_data),
        }
    }

    /// Resolve material and vertex-color from the instance by borrowing its
    /// model through `RefCell`. Prefer `new_preresolved` for optimal safety.
    pub fn new(
        renderer: &'a dyn IRenderer,
        instance: &'a T,
        camera: &'a Camera,
        instance_mesh_idx: usize,
    ) -> Self {
        Self {
            renderer,
            instance: Some(instance),
            camera: Some(camera),
            instance_mesh_idx: instance_mesh_idx as i32,
            material: None,
            has_vertex_colors: false,
            model_data: None,
        }
    }

    pub fn new_without_instance(renderer: &'a dyn IRenderer, camera: &'a Camera) -> Self {
        Self {
            renderer,
            instance: None,
            camera: Some(camera),
            instance_mesh_idx: -1,
            material: None,
            has_vertex_colors: false,
            model_data: None,
        }
    }

    pub fn new_screenspace(renderer: &'a dyn IRenderer) -> Self {
        Self {
            renderer,
            instance: None,
            camera: None,
            instance_mesh_idx: -1,
            material: None,
            has_vertex_colors: false,
            model_data: None,
        }
    }

    pub fn renderer(&self) -> &dyn IRenderer {
        self.renderer
    }
    pub fn camera(&self) -> &Camera {
        self.camera
            .expect("RenderState::camera() on screenspace state")
    }
    pub fn camera_opt(&self) -> Option<&Camera> {
        self.camera
    }
    pub fn instance(&self) -> Option<&T> {
        self.instance
    }
    pub fn mesh(&self) -> Option<&Mesh> {
        None
    }
    pub fn mesh_material(&self) -> Option<&Material> {
        self.material
    }
    pub fn has_vertex_colors(&self) -> bool {
        self.has_vertex_colors
    }
    pub fn has_instance(&self) -> bool {
        self.instance.is_some()
    }
    pub fn instance_mesh_idx(&self) -> i32 {
        self.instance_mesh_idx
    }
    pub fn model_data(&self) -> Option<&ModelRenderData> {
        self.model_data
    }
    pub fn bone_matrices(&self) -> &[Matrix4<f32>] {
        self.model_data
            .map(|d| d.bone_matrices.as_slice())
            .unwrap_or(&[])
    }
    pub fn is_skinned(&self) -> bool {
        self.model_data.map(|d| d.is_skinned).unwrap_or(false)
    }
    pub fn mesh_transform(&self) -> Matrix4<f32> {
        self.model_data
            .map(|d| d.mesh_transform)
            .unwrap_or_else(|| Matrix4::from_scale(1.0))
    }
    pub fn bone_count(&self) -> i32 {
        self.model_data.map(|d| d.bone_count).unwrap_or(0)
    }
}
