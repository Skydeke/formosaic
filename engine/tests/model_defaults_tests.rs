use cgmath::{Matrix4, Vector3};
use formosaic_engine::architecture::models::material::Material;
use formosaic_engine::architecture::models::mesh::Mesh;
use formosaic_engine::architecture::models::model::Model;
use formosaic_engine::opengl::shaders::render_state::RenderState;
use formosaic_engine::rendering::abstracted::processable::Processable;

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
    fn get_lowest(&self) -> f32 {
        0.0
    }
    fn get_meshes(&self) -> &[Mesh] {
        &self.meshes
    }
    fn centroid(&self) -> Option<Vector3<f32>> {
        None
    }
    fn mesh_transform(&self, _mesh_idx: usize) -> Option<Matrix4<f32>> {
        None
    }
    fn get_material(&self, _mesh_idx: usize) -> Option<&Material> {
        None
    }
    fn has_vertex_colors(&self, _mesh_idx: usize) -> bool {
        false
    }
}

#[test]
fn model_default_mesh_helpers_are_stable() {
    let model = DummyModel::new();

    assert_eq!(model.get_mesh_count(), 0);
    assert!(model.get_material(0).is_none());
    assert!(!model.has_vertex_colors(0));
    assert!(model.centroid().is_none());
    assert!(model.mesh_transform(0).is_none());
}
