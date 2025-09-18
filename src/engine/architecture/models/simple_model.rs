use cgmath::Vector3;

use crate::engine::architecture::models::mesh::Mesh;
use crate::engine::architecture::models::model::Model; // Add this import
use crate::engine::rendering::abstracted::processable::Processable;
use crate::engine::rendering::abstracted::renderable::Renderable;
use crate::opengl::constants::render_mode::RenderMode;
use crate::opengl::shaders::RenderState;

pub struct SimpleModel {
    meshes: Vec<Mesh>,
    render_mode: RenderMode,
    centroid: Option<Vector3<f32>>,
}

impl SimpleModel {
    pub fn new(meshes: Vec<Mesh>, render_mode: RenderMode) -> Self {
        Self::with_bounds(meshes, render_mode)
    }

    pub fn with_centroid(
        meshes: Vec<Mesh>,
        render_mode: RenderMode,
        centroid: Vector3<f32>,
    ) -> Self {
        if meshes.is_empty() {
            panic!("SimpleModel must have at least one mesh");
        }
        Self {
            meshes,
            render_mode,
            centroid: Some(centroid),
        }
    }

    pub fn with_bounds(meshes: Vec<Mesh>, render_mode: RenderMode) -> Self {
        if meshes.is_empty() {
            panic!("SimpleModel must have at least one mesh");
        }
        Self {
            meshes,
            render_mode,
            centroid: None,
        }
    }

    pub fn meshes(&self) -> &[Mesh] {
        &self.meshes
    }

    pub fn delete(&mut self) {
        for mesh in &mut self.meshes {
            mesh.delete(true);
        }
    }
}

// Implement Model trait
impl Model for SimpleModel {
    fn render<T: Processable>(&self, _instance_state: &RenderState<T>, mesh_idx: usize) {
        let mesh = &self.meshes[mesh_idx];
        mesh.render(self.render_mode);
    }

    fn bind_and_configure(&mut self, mesh_idx: usize) {
        let mesh = &mut self.meshes[mesh_idx];
        mesh.bind();
    }

    fn unbind(&mut self, mesh_idx: usize) {
        let mesh = &self.meshes[mesh_idx];
        mesh.unbind();
    }

    fn delete(&mut self) {
        for mesh in &mut self.meshes {
            mesh.delete(true);
        }
    }

    fn get_lowest(&self) -> f32 {
        self.meshes
            .iter()
            .map(|m| m.lowest())
            .fold(f32::INFINITY, |a, b| a.min(b))
    }

    fn get_meshes(&self) -> &[Mesh] {
        &self.meshes
    }

    fn centroid(&self) -> Option<Vector3<f32>> {
        self.centroid
    }
}
