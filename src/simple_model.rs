use crate::mesh::Mesh;
use crate::opengl::constants::render_mode::RenderMode;
use crate::opengl::shaders::RenderState;
use crate::renderable::Renderable;

pub struct SimpleModel {
    meshes: Vec<Mesh>,
    render_mode: RenderMode,
}

impl SimpleModel {
    pub fn new(meshes: Vec<Mesh>, render_mode: RenderMode) -> Self {
        Self::with_bounds(meshes, render_mode)
    }

    pub fn with_bounds(meshes: Vec<Mesh>, render_mode: RenderMode) -> Self {
        if meshes.is_empty() {
            panic!("SimpleModel must have at least one mesh");
        }
        Self {
            meshes,
            render_mode,
        }
    }

    pub fn bind_and_configure(&mut self, mesh_idx: usize) {
        let mesh = &mut self.meshes[mesh_idx];
        mesh.bind();
    }

    pub fn render(&self, _state: &RenderState, mesh_idx: usize) {
        let mesh = &self.meshes[mesh_idx];
        mesh.render(self.render_mode);
    }

    pub fn unbind(&self, mesh_idx: usize) {
        let mesh = &self.meshes[mesh_idx];
        mesh.unbind();
    }

    pub fn lowest(&self) -> f32 {
        self.meshes
            .iter()
            .map(|m| m.lowest())
            .fold(f32::INFINITY, |a, b| a.min(b))
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
