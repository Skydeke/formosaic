use cgmath::Vector3;

use crate::engine::rendering::abstracted::processable::Processable;
use crate::{engine::architecture::models::mesh::Mesh, opengl::shaders::render_state::RenderState};

pub trait Model {
    /// Render the model
    fn render<T: Processable>(&self, instance_state: &RenderState<T>, mesh_idx: usize);

    /// Bind and configure the model for rendering
    fn bind_and_configure(&mut self, mesh_idx: usize);

    /// Unbind the model
    fn unbind(&mut self, mesh_idx: usize);

    /// Delete or free the model resources
    fn delete(&mut self);

    /// Returns the lowest point (y or z) of the model
    fn get_lowest(&self) -> f32;

    /// Returns all meshes of the model
    fn get_meshes(&self) -> &[Mesh];

    /// Returns center of Verticies
    fn centroid(&self) -> Option<Vector3<f32>>;
}
