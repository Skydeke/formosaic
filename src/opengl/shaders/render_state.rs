use crate::engine::{
    architecture::models::{mesh::Mesh, model::Model},
    rendering::{
        abstracted::{irenderer::IRenderer, processable::Processable},
        instances::camera::camera::Camera,
    },
};

pub struct RenderState<'a, T: Processable> {
    renderer: &'a dyn IRenderer,
    instance: Option<&'a T>,
    camera: &'a Camera,
    instance_mesh_idx: i32,
    mesh: Option<&'a Mesh>,
}

impl<'a, T: Processable> RenderState<'a, T> {
    /// Creates a RenderState for a specific instance + mesh
    pub fn new(
        renderer: &'a dyn IRenderer,
        instance: &'a T,
        camera: &'a Camera,
        instance_mesh_idx: usize,
    ) -> Self {
        let mesh = instance.get_model().get_meshes().get(instance_mesh_idx);

        Self {
            renderer,
            instance: Some(instance),
            camera,
            instance_mesh_idx: instance_mesh_idx as i32,
            mesh,
        }
    }

    /// Creates a RenderState with only renderer + camera (no instance bound)
    pub fn new_without_instance(renderer: &'a dyn IRenderer, camera: &'a Camera) -> Self {
        Self {
            renderer,
            instance: None,
            camera,
            instance_mesh_idx: -1,
            mesh: None,
        }
    }

    pub fn renderer(&self) -> &dyn IRenderer {
        self.renderer
    }

    pub fn camera(&self) -> &Camera {
        self.camera
    }

    pub fn instance(&self) -> Option<&T> {
        self.instance
    }

    pub fn mesh(&self) -> Option<&Mesh> {
        self.mesh
    }

    pub fn has_instance(&self) -> bool {
        self.instance.is_some()
    }
}
