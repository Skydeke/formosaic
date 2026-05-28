use std::cell::RefCell;
use std::rc::Rc;

use crate::architecture::models::mesh::Mesh;
use crate::architecture::models::simple_model::SimpleModel;

use super::data::ModelLoadData;

/// Builds a SimpleModel incrementally across multiple frames so the
/// UI never freezes.  Work is split into two phases:
///
/// **Phase 1** – Build one mesh each frame (VAO / VBO creation, no textures).
/// **Phase 2** – Upload one material's textures each frame (glTexImage2D +
/// mipmap generation) and attach it to every mesh that references it.
///
/// After `build_next()` returns `true` you must NOT finalize in the same
/// frame — always wait one frame before calling `finish()` so the
/// texture upload completes before model usage begins.
pub struct IncrementalModelBuilder {
    data: ModelLoadData,
    built_meshes: Vec<(usize, Mesh)>,
    next_mesh: usize,
    next_material: usize,
}

impl IncrementalModelBuilder {
    pub fn new(data: ModelLoadData) -> Self {
        let mesh_count = data.meshes.len();
        Self {
            data,
            built_meshes: Vec::with_capacity(mesh_count),
            next_mesh: 0,
            next_material: 0,
        }
    }

    /// Progress in `[0, 1]`.  Accounts for both mesh-building and
    /// texture-upload phases so the bar moves smoothly.
    pub fn progress(&self) -> f32 {
        let total = self.data.meshes.len() + self.data.materials.len();
        if total == 0 {
            return 1.0;
        }
        let done = self.next_mesh.min(self.data.meshes.len())
            + self.next_material.min(self.data.materials.len());
        done as f32 / total as f32
    }

    /// Do one unit of work:
    ///
    /// 1. If meshes remain → build VAO/VBO for the next mesh (no textures).
    /// 2. If materials remain → upload one material's textures and attach
    ///    to every mesh that uses it.
    ///
    /// Returns `true` when **all** work is complete.
    pub fn build_next(&mut self) -> bool {
        if self.next_mesh < self.data.meshes.len() {
            // Take ownership of the current mesh data instead of cloning,
            // since DataBuffer uploads below consume the data immediately.
            let m = &mut self.data.meshes[self.next_mesh];
            let mesh = Mesh::from_raw(
                std::mem::take(&mut m.positions),
                std::mem::take(&mut m.normals),
                std::mem::take(&mut m.texcoords),
                std::mem::take(&mut m.indices),
                std::mem::take(&mut m.colors),
                std::mem::take(&mut m.bone_indices),
                std::mem::take(&mut m.bone_weights),
            );
            self.built_meshes.push((m.material_index, mesh));
            self.next_mesh += 1;
            return false;
        }

        if self.next_material < self.data.materials.len() {
            let material = self.data.materials[self.next_material]
                .clone()
                .into_material();
            for (mat_idx, mesh) in &mut self.built_meshes {
                if *mat_idx == self.next_material {
                    mesh.set_material(material.clone());
                }
            }
            self.next_material += 1;
            return false;
        }

        true
    }

    /// Consume the builder and produce the final `SimpleModel`.
    /// Only call after `build_next()` has returned `true`.
    pub fn finish(self) -> Rc<RefCell<SimpleModel>> {
        let meshes: Vec<Mesh> = self.built_meshes.into_iter().map(|(_, m)| m).collect();
        let mut model = SimpleModel::with_mesh_transforms(
            meshes,
            self.data.render_mode,
            self.data.centroid,
            self.data.mesh_transforms,
        )
        .expect("IncrementalModelBuilder::finish: at least one mesh must have been built");
        model.set_animation_data(self.data.skeleton, self.data.animations);
        Rc::new(RefCell::new(model))
    }
}
