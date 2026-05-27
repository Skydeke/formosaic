use cgmath::{Matrix4, Quaternion, SquareMatrix, Vector3};

#[derive(Clone, Debug)]
pub struct BoneWeight {
    pub vertex_id: u32,
    pub weight: f32,
}

#[derive(Clone, Debug)]
pub struct BoneData {
    pub name: String,
    pub bind_local_transform: Matrix4<f32>,
    pub offset_matrix: Matrix4<f32>,
    pub parent_index: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct Skeleton {
    pub bones: Vec<BoneData>,
    /// Pre-allocated workspace for final matrices (avoids re-allocation each frame).
    final_matrices: Vec<Matrix4<f32>>,
}

impl Skeleton {
    pub fn new(bones: Vec<BoneData>) -> Self {
        let count = bones.len();
        Self {
            bones,
            final_matrices: vec![Matrix4::identity(); count],
        }
    }

    pub fn bone_count(&self) -> usize {
        self.bones.len()
    }

    /// Compute final skinning matrices for GPU upload.
    /// Each final matrix = `animated_world[i] * offset_matrix[i]`.
    ///
    /// The shader applies this as: `uBones[i] * vec4(pos, 1.0)`, which first
    /// transforms the vertex from model space to bone-local space (offset matrix),
    /// then from bone-local to animated world space (world matrix).
    ///
    /// `local_transforms` must have length == `self.bones.len()`.
    /// Each local transform is the bone's animated transform in its parent's space.
    pub fn compute_final_matrices(&mut self, local_transforms: &[Matrix4<f32>]) -> &[Matrix4<f32>] {
        assert_eq!(local_transforms.len(), self.bones.len());

        // Process bones in hierarchy depth order so that when we compute
        // world[child] we have already computed world[parent].
        // Bone indices from the loader are not guaranteed to be sorted by depth.
        let mut depth = vec![0usize; self.bones.len()];
        for i in 0..self.bones.len() {
            let mut d = 0;
            let mut cur = i;
            while let Some(p) = self.bones[cur].parent_index {
                d += 1;
                cur = p;
            }
            depth[i] = d;
        }
        let mut order: Vec<usize> = (0..self.bones.len()).collect();
        order.sort_by_key(|&i| depth[i]);

        // Accumulate world transforms through the hierarchy in depth order
        let mut world = vec![Matrix4::identity(); self.bones.len()];
        for &i in &order {
            let local = local_transforms[i];
            world[i] = match self.bones[i].parent_index {
                Some(parent) => world[parent] * local,
                None => local,
            };
        }

        // Multiply by offset matrix: final = world * offset
        for i in 0..self.bones.len() {
            self.final_matrices[i] = world[i] * self.bones[i].offset_matrix;
        }

        if log::log_enabled!(log::Level::Debug) {
            log::debug!(
                "[Skeleton] compute_final_matrices bones={} root_parent={:?}",
                self.bones.len(),
                self.bones.first().and_then(|b| b.parent_index),
            );
        }

        &self.final_matrices
    }

    /// Build full bone-local transforms from decomposed parts (position, rotation, scale).
    pub fn make_local_transform(pos: Vector3<f32>, rot: Quaternion<f32>, scale: Vector3<f32>) -> Matrix4<f32> {
        Matrix4::from_translation(pos) * Matrix4::from(rot) * Matrix4::from_nonuniform_scale(scale.x, scale.y, scale.z)
    }
}
