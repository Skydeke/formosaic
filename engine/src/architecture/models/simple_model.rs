use cgmath::{Matrix4, Vector3, Vector4};

use crate::architecture::models::animation::AnimationClip;
use crate::architecture::models::animation_player::AnimationPlayer;
use crate::architecture::models::mesh::Mesh;
use crate::architecture::models::model::Model;
use crate::architecture::models::skeleton::Skeleton;
use crate::opengl::constants::render_mode::RenderMode;
use crate::opengl::shaders::RenderState;
use crate::rendering::abstracted::processable::Processable;
use crate::rendering::abstracted::renderable::Renderable;

pub struct SimpleModel {
    meshes: Vec<Mesh>,
    render_mode: RenderMode,
    centroid: Option<Vector3<f32>>,
    mesh_transforms: Vec<Matrix4<f32>>,
    pub skeleton: Option<Skeleton>,
    pub animations: Vec<AnimationClip>,
    pub player: AnimationPlayer,
    /// Per-mesh bone matrices.  Each mesh gets its own set so that meshes
    /// in different local spaces (different node transforms) use the correct
    /// per-mesh inverse bind matrices from the skeleton.
    bone_matrices: Vec<Vec<Matrix4<f32>>>,
    /// Cached AABB center.  Set whenever bone matrices or vertex positions
    /// change.  `visual_center()` returns this or falls back to `centroid`.
    cached_visual_center: Option<Vector3<f32>>,
}

impl SimpleModel {
    /// Creates a new SimpleModel. Returns an error if `meshes` is empty.
    pub fn new(meshes: Vec<Mesh>, render_mode: RenderMode) -> Result<Self, &'static str> {
        Self::with_bounds(meshes, render_mode)
    }

    /// Returns an error if `meshes` is empty.
    pub fn with_centroid(
        meshes: Vec<Mesh>,
        render_mode: RenderMode,
        centroid: Vector3<f32>,
    ) -> Result<Self, &'static str> {
        if meshes.is_empty() {
            return Err("SimpleModel must have at least one mesh");
        }
        let mesh_count = meshes.len();
        Ok(Self {
            meshes,
            render_mode,
            centroid: Some(centroid),
            mesh_transforms: vec![Matrix4::from_scale(1.0); mesh_count],
            skeleton: None,
            animations: Vec::new(),
            player: AnimationPlayer::new(),
            bone_matrices: vec![Vec::new(); mesh_count],
            cached_visual_center: None,
        })
    }

    /// Returns an error if `meshes` is empty.
    pub fn with_bounds(meshes: Vec<Mesh>, render_mode: RenderMode) -> Result<Self, &'static str> {
        if meshes.is_empty() {
            return Err("SimpleModel must have at least one mesh");
        }
        let mesh_count = meshes.len();
        Ok(Self {
            meshes,
            render_mode,
            centroid: None,
            mesh_transforms: vec![Matrix4::from_scale(1.0); mesh_count],
            skeleton: None,
            animations: Vec::new(),
            player: AnimationPlayer::new(),
            bone_matrices: Vec::new(),
            cached_visual_center: None,
        })
    }

    /// Returns an error if `meshes` is empty.
    pub fn with_mesh_transforms(
        meshes: Vec<Mesh>,
        render_mode: RenderMode,
        centroid: Option<Vector3<f32>>,
        mesh_transforms: Vec<Matrix4<f32>>,
    ) -> Result<Self, &'static str> {
        if meshes.is_empty() {
            return Err("SimpleModel must have at least one mesh");
        }
        Ok(Self {
            meshes,
            render_mode,
            centroid,
            mesh_transforms,
            skeleton: None,
            animations: Vec::new(),
            player: AnimationPlayer::new(),
            bone_matrices: Vec::new(),
            cached_visual_center: None,
        })
    }

    pub fn meshes(&self) -> &[Mesh] {
        &self.meshes
    }

    pub fn set_animation_data(
        &mut self,
        skeleton: Option<Skeleton>,
        animations: Vec<AnimationClip>,
    ) {
        self.skeleton = skeleton;
        self.animations = animations;
        self.bone_matrices.resize_with(self.meshes.len(), Vec::new);
        if let Some(ref mut skel) = self.skeleton {
            let bind_poses: Vec<Matrix4<f32>> =
                skel.bones.iter().map(|b| b.bind_local_transform).collect();
            for mesh_idx in 0..self.meshes.len() {
                let mesh_off = mesh_idx.min(skel.mesh_count.saturating_sub(1));
                self.bone_matrices[mesh_idx] =
                    skel.compute_final_matrices(&bind_poses, mesh_off).to_vec();
            }
        } else {
            for bm in &mut self.bone_matrices {
                bm.clear();
            }
        }
        self.refresh_visual_center();
    }

    pub fn update_animation(&mut self, dt: f32) {
        if let Some(ref mut skel) = self.skeleton {
            for mesh_idx in 0..self.meshes.len() {
                let mesh_off = mesh_idx.min(skel.mesh_count.saturating_sub(1));
                self.bone_matrices[mesh_idx] = self.player.evaluate(skel, mesh_off);
            }
            self.player.update(dt);
        }
    }

    pub fn bone_matrices_for_mesh(&self, mesh_idx: usize) -> &[Matrix4<f32>] {
        if self.skeleton.is_some() {
            self.bone_matrices.get(mesh_idx).map_or(&[], |b| b.as_slice())
        } else {
            &[]
        }
    }

    fn refresh_visual_center(&mut self) {
        let mut min = Vector3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut max = Vector3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);

        for (mesh_idx, mesh) in self.meshes.iter().enumerate() {
            let bones = self.bone_matrices_for_mesh(mesh_idx);
            let mesh_transform = self
                .mesh_transforms
                .get(mesh_idx)
                .copied()
                .unwrap_or_else(|| Matrix4::from_scale(1.0));
            let pos = mesh.positions();
            let bone_indices = mesh.bone_indices();
            let bone_weights = mesh.bone_weights();

            let mut vertex_idx = 0usize;
            let mut i = 0usize;
            while i + 2 < pos.len() {
                let local = Vector4::new(pos[i], pos[i + 1], pos[i + 2], 1.0);

                let skinned = if mesh.is_skinned() {
                    let mut blended = Vector4::new(0.0, 0.0, 0.0, 0.0);
                    let mut has_influence = false;
                    if let (Some(indices), Some(weights)) =
                        (bone_indices.get(vertex_idx), bone_weights.get(vertex_idx))
                    {
                        for j in 0..4 {
                            let bi = indices[j];
                            if bi >= 0 {
                                let weight = weights[j];
                                let bi = bi as usize;
                                if bi < bones.len() {
                                    blended += bones[bi] * local * weight;
                                    has_influence = true;
                                }
                            }
                        }
                    }
                    if has_influence {
                        blended
                    } else {
                        local
                    }
                } else {
                    mesh_transform * local
                };

                let p = Vector3::new(skinned.x, skinned.y, skinned.z);
                min.x = min.x.min(p.x);
                min.y = min.y.min(p.y);
                min.z = min.z.min(p.z);
                max.x = max.x.max(p.x);
                max.y = max.y.max(p.y);
                max.z = max.z.max(p.z);

                vertex_idx += 1;
                i += 3;
            }
        }

        self.cached_visual_center = if min.x == f32::INFINITY {
            self.centroid
        } else {
            Some((min + max) * 0.5)
        };
    }

    pub fn visual_center(&self) -> Option<Vector3<f32>> {
        self.cached_visual_center.or(self.centroid)
    }

    pub fn animations(&self) -> &[AnimationClip] {
        &self.animations
    }

    pub fn upload_mesh_positions(&mut self, mesh_idx: usize, positions: Vec<f32>) {
        if let Some(mesh) = self.meshes.get_mut(mesh_idx) {
            mesh.upload_positions(positions);
        }
    }

    pub fn play_animation(&mut self, index: usize) {
        if index >= self.animations.len() {
            return;
        }
        let clip = self.animations[index].clone();
        self.player.play(clip, &self.bone_matrices);
        if let Some(ref mut skel) = self.skeleton {
            for mesh_idx in 0..self.meshes.len() {
                let mesh_off = mesh_idx.min(skel.mesh_count.saturating_sub(1));
                self.bone_matrices[mesh_idx] = self.player.evaluate(skel, mesh_off);
            }
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
            let bones = self.bone_matrices_for_mesh(mesh_idx);
            let mesh_transform = self
                .mesh_transforms
                .get(mesh_idx)
                .copied()
                .unwrap_or_else(|| Matrix4::from_scale(1.0));
            let pos = mesh.positions();
            let bone_indices = mesh.bone_indices();
            let bone_weights = mesh.bone_weights();
            let is_skinned = mesh.is_skinned() && !bones.is_empty();

            let mut vertex_idx = 0usize;
            let mut i = 0;
            while i + 2 < pos.len() {
                let local = Vector4::new(pos[i], pos[i + 1], pos[i + 2], 1.0);
                let skinned = if is_skinned {
                    let mut blended = Vector4::new(0.0, 0.0, 0.0, 0.0);
                    let mut has_influence = false;
                    if let (Some(indices), Some(weights)) =
                        (bone_indices.get(vertex_idx), bone_weights.get(vertex_idx))
                    {
                        for influence in 0..4 {
                            let bone_idx = indices[influence];
                            let weight = weights[influence];
                            if bone_idx >= 0 && weight > 0.0 {
                                if let Some(bone) = bones.get(bone_idx as usize) {
                                    blended += *bone * local * weight;
                                    has_influence = true;
                                }
                            }
                        }
                    }
                    if has_influence {
                        blended
                    } else {
                        local
                    }
                } else {
                    local
                };
                let p = if is_skinned { skinned } else { mesh_transform * skinned };
                lowest = lowest.min(p.y);
                vertex_idx += 1;
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

    fn visual_center(&self) -> Option<Vector3<f32>> {
        SimpleModel::visual_center(self)
    }

    fn mesh_transform(&self, mesh_idx: usize) -> Option<Matrix4<f32>> {
        self.mesh_transforms.get(mesh_idx).copied()
    }

    fn bone_matrices(&self, mesh_idx: usize) -> &[Matrix4<f32>] {
        self.bone_matrices_for_mesh(mesh_idx)
    }

    fn update_animation(&mut self, dt: f32) {
        if let Some(ref mut skel) = self.skeleton {
            for mesh_idx in 0..self.meshes.len() {
                let mesh_off = mesh_idx.min(skel.mesh_count.saturating_sub(1));
                self.bone_matrices[mesh_idx] = self.player.evaluate(skel, mesh_off);
            }
            self.player.update(dt);
        }
    }

    fn animations(&self) -> &[AnimationClip] {
        &self.animations
    }
}
