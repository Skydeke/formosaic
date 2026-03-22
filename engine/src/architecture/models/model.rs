use cgmath::Vector3;

use crate::rendering::abstracted::processable::Processable;
use crate::{
    architecture::models::{material::Material, mesh::Mesh},
    opengl::shaders::render_state::RenderState,
};

pub trait Model {
    fn render<T: Processable>(&self, instance_state: &RenderState<T>, mesh_idx: usize);
    fn bind_and_configure(&mut self, mesh_idx: usize);
    fn unbind(&mut self, mesh_idx: usize);
    fn delete(&mut self);
    fn get_lowest(&self) -> f32;
    fn get_meshes(&self) -> &[Mesh];
    fn centroid(&self) -> Option<Vector3<f32>>;

    /// Number of sub-meshes (default: get_meshes().len()).
    fn get_mesh_count(&self) -> usize {
        self.get_meshes().len()
    }

    /// Per-mesh material (default: reads from get_meshes()).
    fn get_material(&self, mesh_idx: usize) -> Option<&Material> {
        self.get_meshes().get(mesh_idx).and_then(|m| m.material())
    }

    /// Whether mesh at mesh_idx has vertex colors (default: reads from get_meshes()).
    fn has_vertex_colors(&self, mesh_idx: usize) -> bool {
        self.get_meshes()
            .get(mesh_idx)
            .map(|m| m.has_vertex_colors())
            .unwrap_or(false)
    }
}
