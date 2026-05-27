use cgmath::{Matrix4, SquareMatrix, Vector3, Vector4};

use crate::architecture::models::animation::AnimationClip;
use crate::architecture::models::animation_player::AnimationPlayer;
use crate::architecture::models::mesh::Mesh;
use crate::architecture::models::model::Model;
use crate::architecture::models::skeleton::Skeleton;
use crate::opengl::constants::render_mode::RenderMode;
use crate::opengl::shaders::RenderState;
use crate::rendering::abstracted::processable::Processable;
use crate::rendering::abstracted::renderable::Renderable;

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
    /// Compute puzzle parameters from raw per-mesh position data and transforms.
    /// No GPU context needed — works on the background thread.
    pub fn from_raw_positions(
        mesh_positions: &[&[f32]],
        mesh_transforms: &[Matrix4<f32>],
        target_world_radius: f32,
        fov_radians: f32,
    ) -> Self {
        use cgmath::InnerSpace;
        let mut min = Vector3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut max = Vector3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);

        for (mesh_idx, pos) in mesh_positions.iter().enumerate() {
            let mesh_transform = mesh_transforms
                .get(mesh_idx)
                .copied()
                .unwrap_or_else(|| Matrix4::from_scale(1.0));
            let mut i = 0;
            while i + 2 < pos.len() {
                let p = mesh_transform * Vector4::new(pos[i], pos[i + 1], pos[i + 2], 1.0);
                let (x, y, z) = (p.x, p.y, p.z);
                if x < min.x { min.x = x; }
                if y < min.y { min.y = y; }
                if z < min.z { min.z = z; }
                if x > max.x { max.x = x; }
                if y > max.y { max.y = y; }
                if z > max.z { max.z = z; }
                i += 3;
            }
        }

        if min.x == f32::INFINITY {
            return Self::default_for(target_world_radius);
        }

        let extent = max - min;
        let model_radius = (extent.magnitude() * 0.5).max(0.001);
        let entity_scale = target_world_radius / model_radius;
        let half_fov = fov_radians * 0.5;
        let orbit_distance = target_world_radius / (half_fov.tan() * 0.65);

        Self {
            entity_scale,
            orbit_distance,
            min_disp: model_radius * 0.02,
            max_disp: model_radius * 0.12,
            model_space_radius: model_radius,
        }
    }

    pub fn default_for(target_world_radius: f32) -> Self {
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
    mesh_transforms: Vec<Matrix4<f32>>,
    pub skeleton: Option<Skeleton>,
    pub animations: Vec<AnimationClip>,
    pub player: AnimationPlayer,
    bone_matrices: Vec<Matrix4<f32>>,
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
        let mesh_count = meshes.len();
        if meshes.is_empty() {
            panic!("SimpleModel must have at least one mesh");
        }
        Self {
            meshes,
            render_mode,
            centroid: Some(centroid),
            mesh_transforms: vec![Matrix4::from_scale(1.0); mesh_count],
            skeleton: None,
            animations: Vec::new(),
            player: AnimationPlayer::new(),
            bone_matrices: Vec::new(),
        }
    }

    pub fn with_bounds(meshes: Vec<Mesh>, render_mode: RenderMode) -> Self {
        let mesh_count = meshes.len();
        if meshes.is_empty() {
            panic!("SimpleModel must have at least one mesh");
        }
        Self {
            meshes,
            render_mode,
            centroid: None,
            mesh_transforms: vec![Matrix4::from_scale(1.0); mesh_count],
            skeleton: None,
            animations: Vec::new(),
            player: AnimationPlayer::new(),
            bone_matrices: Vec::new(),
        }
    }

    pub fn with_mesh_transforms(
        meshes: Vec<Mesh>,
        render_mode: RenderMode,
        centroid: Option<Vector3<f32>>,
        mesh_transforms: Vec<Matrix4<f32>>,
    ) -> Self {
        if meshes.is_empty() {
            panic!("SimpleModel must have at least one mesh");
        }
        Self {
            meshes,
            render_mode,
            centroid,
            mesh_transforms,
            skeleton: None,
            animations: Vec::new(),
            player: AnimationPlayer::new(),
            bone_matrices: Vec::new(),
        }
    }

    pub fn meshes(&self) -> &[Mesh] {
        &self.meshes
    }

    pub fn set_animation_data(&mut self, skeleton: Option<Skeleton>, animations: Vec<AnimationClip>) {
        self.skeleton = skeleton;
        self.animations = animations;
        let bone_count = self.skeleton.as_ref().map(|s| s.bone_count()).unwrap_or(0);
        self.bone_matrices = vec![Matrix4::identity(); bone_count.max(1)];
    }

    pub fn update_animation(&mut self, dt: f32) {
        if let Some(ref mut skel) = self.skeleton {
            self.player.update(dt);
            self.bone_matrices = self.player.evaluate(skel);
        }
    }

    pub fn bone_matrices(&self) -> &[Matrix4<f32>] {
        if self.skeleton.is_some() && self.player.has_clip() {
            &self.bone_matrices
        } else {
            &[]
        }
    }

    pub fn animations(&self) -> &[AnimationClip] {
        &self.animations
    }

    pub fn pick_random_animation(&mut self) {
        if self.animations.is_empty() {
            return;
        }
        use rand::Rng;
        let idx = rand::rng().random_range(0..self.animations.len());
        self.player.play(self.animations[idx].clone());
    }

    pub fn pick_solve_animation(&mut self) {
        if self.animations.is_empty() {
            return;
        }

        let prefer = ["idle", "interact", "pose", "stand", "celebrat", "victory", "win", "success"];
        let avoid = ["run", "walk", "jump", "fall", "hit", "punch", "attack", "roll", "hurt"];

        let chosen = self
            .animations
            .iter()
            .enumerate()
            .filter(|(_, clip)| {
                let n = clip.name.to_lowercase();
                !avoid.iter().any(|k| n.contains(k))
            })
            .max_by_key(|(_, clip)| {
                let n = clip.name.to_lowercase();
                prefer.iter().position(|k| n.contains(k)).map(|i| 100 - i as i32).unwrap_or(0)
            })
            .map(|(idx, clip)| (idx, clip.clone()))
            .or_else(|| self.animations.first().cloned().map(|clip| (0, clip)));

        let Some((idx, clip)) = chosen else { return; };

        self.player.play(clip);
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

        for (mesh_idx, mesh) in self.meshes.iter().enumerate() {
            let mesh_transform = self
                .mesh_transforms
                .get(mesh_idx)
                .copied()
                .unwrap_or_else(|| Matrix4::from_scale(1.0));
            let pos = mesh.positions();
            let mut i = 0;
            while i + 2 < pos.len() {
                let p = mesh_transform * Vector4::new(pos[i], pos[i + 1], pos[i + 2], 1.0);
                let x = p.x;
                let y = p.y;
                let z = p.z;
                if x < min.x { min.x = x; }
                if y < min.y { min.y = y; }
                if z < min.z { min.z = z; }
                if x > max.x { max.x = x; }
                if y > max.y { max.y = y; }
                if z > max.z { max.z = z; }
                i += 3;
            }
        }

        if min.x == f32::INFINITY {
            return PuzzleParams::default_for(target_world_radius);
        }

        let extent = max - min;
        let model_radius = extent.magnitude() * 0.5;
        let model_radius = model_radius.max(0.001);

        let entity_scale = target_world_radius / model_radius;
        let half_fov = fov_radians * 0.5;
        let orbit_distance = target_world_radius / (half_fov.tan() * 0.65);

        PuzzleParams {
            entity_scale,
            orbit_distance,
            min_disp: model_radius * 0.02,
            max_disp: model_radius * 0.12,
            model_space_radius: model_radius,
        }
    }

    pub fn scramble_along_axis(&mut self, axis: Vector3<f32>, min_disp: f32, max_disp: f32) {
        for (mesh_idx, mesh) in self.meshes.iter_mut().enumerate() {
            let mesh_transform = self
                .mesh_transforms
                .get(mesh_idx)
                .cloned()
                .unwrap_or_else(|| cgmath::Matrix4::from_scale(1.0));

            mesh.scramble_along_axis(axis, min_disp, max_disp, mesh_transform);
        }
    }

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

impl Model for SimpleModel {
    fn render<T: Processable>(&self, _instance_state: &RenderState<T>, mesh_idx: usize) {
        let mesh = &self.meshes[mesh_idx];
        mesh.render(self.render_mode);
    }

    fn bind_and_configure(&self, mesh_idx: usize) {
        let mesh = &self.meshes[mesh_idx];
        mesh.bind();
    }

    fn unbind(&self, mesh_idx: usize) {
        let mesh = &self.meshes[mesh_idx];
        mesh.unbind();
    }

    fn delete(&mut self) {
        for mesh in &mut self.meshes {
            mesh.delete(true);
        }
    }

    fn get_lowest(&self) -> f32 {
        let mut lowest = f32::INFINITY;
        for (mesh_idx, mesh) in self.meshes.iter().enumerate() {
            let mesh_transform = self
                .mesh_transforms
                .get(mesh_idx)
                .copied()
                .unwrap_or_else(|| Matrix4::from_scale(1.0));
            let pos = mesh.positions();
            let mut i = 0;
            while i + 2 < pos.len() {
                let p = mesh_transform * Vector4::new(pos[i], pos[i + 1], pos[i + 2], 1.0);
                lowest = lowest.min(p.y);
                i += 3;
            }
        }
        lowest
    }

    fn get_meshes(&self) -> &[Mesh] {
        &self.meshes
    }

    fn centroid(&self) -> Option<Vector3<f32>> {
        self.centroid
    }

    fn mesh_transform(&self, mesh_idx: usize) -> Option<Matrix4<f32>> {
        self.mesh_transforms.get(mesh_idx).copied()
    }

    fn bone_matrices(&self) -> &[Matrix4<f32>] {
        if self.skeleton.is_some() && self.player.has_clip() {
            &self.bone_matrices
        } else {
            &[]
        }
    }

    fn update_animation(&mut self, dt: f32) {
        if let Some(ref mut skel) = self.skeleton {
            self.player.update(dt);
            self.bone_matrices = self.player.evaluate(skel);
        }
    }

    fn animations(&self) -> &[AnimationClip] {
        &self.animations
    }
}
