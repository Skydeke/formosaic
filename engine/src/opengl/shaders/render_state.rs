use crate::{
    architecture::models::{material::Material, mesh::Mesh, model::Model},
    rendering::{
        abstracted::{irenderer::IRenderer, processable::Processable},
        instances::camera::camera::Camera,
    },
};

pub struct RenderState<'a, T: Processable> {
    renderer:          &'a dyn IRenderer,
    instance:          Option<&'a T>,
    camera:            Option<&'a Camera>,
    #[allow(dead_code)]
    instance_mesh_idx: i32,
    material_owned:    Option<Material>,
    has_vertex_colors: bool,
}

impl<'a, T: Processable> RenderState<'a, T> {
    /// Primary constructor used by EntityRenderer.
    /// Material and vertex-color flag are pre-resolved by the caller from a
    /// shared borrow of the model, before borrow_mut() is taken for rendering.
    /// This avoids aliased-borrow UB with SimpleEntity's unsafe get_model().
    pub fn new_preresolved(
        renderer:          &'a dyn IRenderer,
        instance:          &'a T,
        camera:            &'a Camera,
        instance_mesh_idx: usize,
        material:          Option<Material>,
        has_vertex_colors: bool,
    ) -> Self {
        Self {
            renderer,
            instance: Some(instance),
            camera: Some(camera),
            instance_mesh_idx: instance_mesh_idx as i32,
            material_owned: material,
            has_vertex_colors,
        }
    }

    /// Legacy constructor — safe only when no borrow_mut on the model is active.
    pub fn new(
        renderer:          &'a dyn IRenderer,
        instance:          &'a T,
        camera:            &'a Camera,
        instance_mesh_idx: usize,
    ) -> Self {
        let model = instance.get_model();
        let material_owned    = model.get_material(instance_mesh_idx).cloned();
        let has_vertex_colors = model.has_vertex_colors(instance_mesh_idx);
        Self {
            renderer,
            instance: Some(instance),
            camera: Some(camera),
            instance_mesh_idx: instance_mesh_idx as i32,
            material_owned,
            has_vertex_colors,
        }
    }

    pub fn new_without_instance(renderer: &'a dyn IRenderer, camera: &'a Camera) -> Self {
        Self {
            renderer,
            instance: None,
            camera: Some(camera),
            instance_mesh_idx: -1,
            material_owned: None,
            has_vertex_colors: false,
        }
    }

    pub fn new_screenspace(renderer: &'a dyn IRenderer) -> Self {
        Self {
            renderer,
            instance: None,
            camera: None,
            instance_mesh_idx: -1,
            material_owned: None,
            has_vertex_colors: false,
        }
    }

    pub fn renderer(&self)               -> &dyn IRenderer    { self.renderer }
    pub fn camera(&self)                 -> &Camera            {
        self.camera.expect("RenderState::camera() on screenspace state")
    }
    pub fn camera_opt(&self)             -> Option<&Camera>   { self.camera }
    pub fn instance(&self)               -> Option<&T>        { self.instance }
    pub fn mesh(&self)                   -> Option<&Mesh>     { None }
    pub fn mesh_material(&self)          -> Option<&Material> { self.material_owned.as_ref() }
    pub fn has_vertex_colors(&self)      -> bool              { self.has_vertex_colors }
    pub fn has_instance(&self)           -> bool              { self.instance.is_some() }
}
