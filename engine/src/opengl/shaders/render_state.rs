use crate::{
    architecture::models::{mesh::Mesh, model::Model},
    rendering::{
        abstracted::{irenderer::IRenderer, processable::Processable},
        instances::camera::camera::Camera,
    },
};

pub struct RenderState<'a, T: Processable> {
    renderer:          &'a dyn IRenderer,
    instance:          Option<&'a T>,
    camera:            Option<&'a Camera>,
    #[allow(dead_code)] // retained for future per-mesh shader logic
    instance_mesh_idx: i32,
    mesh:              Option<&'a Mesh>,
}

impl<'a, T: Processable> RenderState<'a, T> {
    /// Full state: instance + camera.
    pub fn new(
        renderer:          &'a dyn IRenderer,
        instance:          &'a T,
        camera:            &'a Camera,
        instance_mesh_idx: usize,
    ) -> Self {
        let mesh = instance.get_model().get_meshes().get(instance_mesh_idx);
        Self {
            renderer,
            instance: Some(instance),
            camera: Some(camera),
            instance_mesh_idx: instance_mesh_idx as i32,
            mesh,
        }
    }

    /// Camera only, no instance (e.g. per-render uniform pass in OutlineRenderer).
    pub fn new_without_instance(renderer: &'a dyn IRenderer, camera: &'a Camera) -> Self {
        Self {
            renderer,
            instance: None,
            camera: Some(camera),
            instance_mesh_idx: -1,
            mesh: None,
        }
    }

    /// No camera, no instance (fullscreen-quad renderers such as HudRenderer
    /// and DiscRenderer whose shaders don't use a view-projection matrix).
    pub fn new_screenspace(renderer: &'a dyn IRenderer) -> Self {
        Self {
            renderer,
            instance: None,
            camera: None,
            instance_mesh_idx: -1,
            mesh: None,
        }
    }

    pub fn renderer(&self) -> &dyn IRenderer { self.renderer }

    /// Panics if the render state was created without a camera.
    pub fn camera(&self) -> &Camera {
        self.camera.expect("RenderState::camera() called on a screenspace state (no camera bound)")
    }

    pub fn camera_opt(&self) -> Option<&Camera> { self.camera }

    pub fn instance(&self) -> Option<&T> { self.instance }

    pub fn mesh(&self) -> Option<&Mesh> { self.mesh }

    pub fn has_instance(&self) -> bool { self.instance.is_some() }
}
