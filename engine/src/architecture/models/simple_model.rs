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
    bone_matrices: Vec<Matrix4<f32>>,
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
            bone_matrices: Vec::new(),
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
        if let Some(ref mut skel) = self.skeleton {
            let bind_poses: Vec<Matrix4<f32>> =
                skel.bones.iter().map(|b| b.bind_local_transform).collect();
            self.bone_matrices = skel.compute_final_matrices(&bind_poses).to_vec();
        } else {
            self.bone_matrices = Vec::new();
        }
    }

    pub fn update_animation(&mut self, dt: f32) {
        if let Some(ref mut skel) = self.skeleton {
            self.bone_matrices = self.player.evaluate(skel);
            self.player.update(dt);
        }
    }

    pub fn bone_matrices(&self) -> &[Matrix4<f32>] {
        if self.skeleton.is_some() {
            &self.bone_matrices
        } else {
            &[]
        }
    }

    pub fn visual_center(&self) -> Option<Vector3<f32>> {
        let mut min = Vector3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut max = Vector3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
        let bones = self.bone_matrices();

        for (mesh_idx, mesh) in self.meshes.iter().enumerate() {
            let mesh_transform = self
                .mesh_transforms
                .get(mesh_idx)
                .copied()
                .unwrap_or_else(|| Matrix4::from_scale(1.0));
            let pos = mesh.positions();
            let offsets = mesh.displacement_offsets();
            let displacement_t = mesh.current_displacement_lerp();
            let bone_indices = mesh.bone_indices();
            let bone_weights = mesh.bone_weights();

            let mut vertex_idx = 0usize;
            let mut i = 0usize;
            while i + 2 < pos.len() {
                let local = Vector4::new(
                    pos[i] + offsets.get(i).copied().unwrap_or(0.0) * displacement_t,
                    pos[i + 1] + offsets.get(i + 1).copied().unwrap_or(0.0) * displacement_t,
                    pos[i + 2] + offsets.get(i + 2).copied().unwrap_or(0.0) * displacement_t,
                    1.0,
                );

                let skinned = if !bones.is_empty() {
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

                let p = if mesh.is_skinned() && !bones.is_empty() {
                    skinned
                } else {
                    mesh_transform * skinned
                };
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

        if min.x == f32::INFINITY {
            self.centroid
        } else {
            Some((min + max) * 0.5)
        }
    }

    pub fn animations(&self) -> &[AnimationClip] {
        &self.animations
    }

    pub fn pick_solve_animation(&mut self) {
        if self.animations.is_empty() {
            return;
        }

        let prefer = [
            "idle", "interact", "pose", "stand", "celebrat", "victory", "win", "success",
        ];
        let avoid = [
            "run", "walk", "jump", "fall", "hit", "punch", "attack", "roll", "hurt",
        ];

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
                prefer
                    .iter()
                    .position(|k| n.contains(k))
                    .map(|i| 100 - i as i32)
                    .unwrap_or(0)
            })
            .map(|(idx, clip)| (idx, clip.clone()))
            .or_else(|| self.animations.first().cloned().map(|clip| (0, clip)));

        let Some((_idx, clip)) = chosen else {
            return;
        };

        self.player.play(clip);

        // Immediately evaluate so bone_matrices reflect the first frame.
        if let Some(ref mut skel) = self.skeleton {
            self.bone_matrices = self.player.evaluate(skel);
        }
    }

    pub fn set_displacement_offsets(&mut self, offsets_per_mesh: Vec<Vec<f32>>) {
        for (mesh, offsets) in self.meshes.iter_mut().zip(offsets_per_mesh) {
            mesh.set_displacement_offsets(offsets);
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
        let bones = self.bone_matrices();
        for (mesh_idx, mesh) in self.meshes.iter().enumerate() {
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

    fn bone_matrices(&self) -> &[Matrix4<f32>] {
        if self.skeleton.is_some() {
            &self.bone_matrices
        } else {
            &[]
        }
    }

    fn update_animation(&mut self, dt: f32) {
        if let Some(ref mut skel) = self.skeleton {
            self.bone_matrices = self.player.evaluate(skel);
            self.player.update(dt);
        }
    }

    fn animations(&self) -> &[AnimationClip] {
        &self.animations
    }
}
