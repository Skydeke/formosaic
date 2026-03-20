use cgmath::Vector3;

use crate::engine::architecture::models::mesh::Mesh;
use crate::engine::architecture::models::model::Model;
use crate::engine::rendering::abstracted::processable::Processable;
use crate::engine::rendering::abstracted::renderable::Renderable;
use crate::opengl::constants::render_mode::RenderMode;
use crate::opengl::shaders::RenderState;

/// Puzzle setup parameters derived from model geometry analysis.
#[derive(Debug, Clone, Copy)]
pub struct PuzzleParams {
    /// Uniform scale to apply to the entity so the model fits the target world radius.
    pub entity_scale: f32,
    /// Camera orbit distance for a comfortable view.
    pub orbit_distance: f32,
    /// Minimum scramble displacement in model space.
    pub min_disp: f32,
    /// Maximum scramble displacement in model space.
    pub max_disp: f32,
    /// Bounding-sphere radius in model space (informational).
    pub model_space_radius: f32,
}

impl PuzzleParams {
    fn default_for(target_world_radius: f32) -> Self {
        Self {
            entity_scale: 0.005,
            orbit_distance: target_world_radius * 3.0,
            min_disp: 3.0,
            max_disp: 15.0,
            model_space_radius: 1.0,
        }
    }
}

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

    /// Analyse the model geometry and return parameters suitable for puzzle setup.
    ///
    /// Computes:
    /// - The bounding-sphere radius in model space (half the AABB diagonal).
    /// - Recommended entity world scale so the model's bounding sphere has a
    ///   given target world-space radius.
    /// - Recommended orbit camera distance for a comfortable view.
    /// - Recommended scramble displacement range (min/max) in model space,
    ///   proportional to model size so gaps look the same regardless of model.
    pub fn compute_puzzle_params(
        &self,
        target_world_radius: f32,
        fov_radians: f32,
    ) -> PuzzleParams {
        use cgmath::InnerSpace;

        // ── Bounding box in model space ───────────────────────────────────────
        let mut min = Vector3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut max = Vector3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);

        for mesh in &self.meshes {
            let pos = mesh.positions();
            let mut i = 0;
            while i + 2 < pos.len() {
                let x = pos[i];
                let y = pos[i + 1];
                let z = pos[i + 2];
                if x < min.x {
                    min.x = x;
                }
                if y < min.y {
                    min.y = y;
                }
                if z < min.z {
                    min.z = z;
                }
                if x > max.x {
                    max.x = x;
                }
                if y > max.y {
                    max.y = y;
                }
                if z > max.z {
                    max.z = z;
                }
                i += 3;
            }
        }

        // Fallback for empty models.
        if min.x == f32::INFINITY {
            return PuzzleParams::default_for(target_world_radius);
        }

        let extent = max - min;
        let model_radius = extent.magnitude() * 0.5; // half diagonal = bounding sphere
        let model_radius = model_radius.max(0.001);

        // ── Derived parameters ────────────────────────────────────────────────
        // Scale so the bounding sphere matches the target world radius.
        let entity_scale = target_world_radius / model_radius;

        // Camera distance: model fills ~65% of the view frustum height.
        // half_fov gives the half-angle; tan(half_fov) * distance = world_radius.
        let half_fov = fov_radians * 0.5;
        let orbit_distance = target_world_radius / (half_fov.tan() * 0.65);

        // Small enough that near-solution the gaps are tiny; large enough to be
        // clearly visible from the side.
        let min_disp = model_radius * 0.02;
        let max_disp = model_radius * 0.12;

        PuzzleParams {
            entity_scale,
            orbit_distance,
            min_disp,
            max_disp,
            model_space_radius: model_radius,
        }
    }

    /// Scramble all meshes: offset every triangle by a random amount along `axis`.
    pub fn scramble_along_axis(&mut self, axis: Vector3<f32>, min_disp: f32, max_disp: f32) {
        for mesh in &mut self.meshes {
            mesh.scramble_along_axis(axis, min_disp, max_disp);
        }
    }

    /// Upload positions lerped between scrambled (t=1.0) and original (t=0.0).
    /// Drive this every frame during the unscramble animation.
    pub fn upload_lerp(&self, t: f32) {
        for mesh in &self.meshes {
            mesh.upload_lerp(t);
        }
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
