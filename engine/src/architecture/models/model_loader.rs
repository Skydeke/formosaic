//! Model loader — parses mesh data from raw bytes using assimp.
//!
//! Platform-specific file I/O is intentionally absent here.  The caller
//! (game layer) reads the bytes however it needs to (filesystem, Android
//! asset manager, etc.) and passes them in.  The engine stays generic.

use std::cell::RefCell;
use std::rc::Rc;

use cgmath::{Vector3, Vector4};
use russimp_ng::material::Material as AiMaterial;
use russimp_ng::mesh::Mesh as AiMesh;
use russimp_ng::scene::{PostProcess, Scene};

use crate::architecture::models::material::Material;
use crate::architecture::models::mesh::Mesh;
use crate::architecture::models::model_cache::ModelCache;
use crate::architecture::models::simple_model::SimpleModel;
use crate::opengl::constants::render_mode::RenderMode;

pub struct ModelLoader;

impl ModelLoader {
    /// Parse a model from raw bytes.
    ///
    /// `cache_key` is an arbitrary string used to de-duplicate loads (typically
    /// the asset path).  `hint` is the file extension used by assimp to pick a
    /// parser (e.g. `"fbx"`, `"obj"`).
    pub fn load_from_bytes(
        cache_key: &str,
        bytes: &[u8],
        hint: &str,
    ) -> Rc<RefCell<SimpleModel>> {
        if let Some(model) = ModelCache::get(cache_key) {
            return model;
        }

        let scene = Scene::from_buffer(
            bytes,
            vec![
                PostProcess::Triangulate,
                PostProcess::GenerateNormals,
                PostProcess::ImproveCacheLocality,
                PostProcess::CalculateTangentSpace,
                PostProcess::PreTransformVertices,
                PostProcess::EmbedTextures,
            ],
            hint,
        )
        .unwrap_or_else(|e| panic!("Failed to parse model '{}': {:?}", cache_key, e));

        let mut sum   = Vector3::new(0.0f32, 0.0, 0.0);
        let mut count = 0usize;

        let materials: Vec<Material> = scene.materials.iter()
            .map(|m| Self::process_material(m))
            .collect();

        let meshes: Vec<Mesh> = scene.meshes.iter()
            .map(|m| {
                for v in &m.vertices {
                    sum += Vector3::new(v.x, v.y, v.z);
                    count += 1;
                }
                let mut mesh = Self::process_mesh(m);
                let idx = m.material_index as usize;
                if idx < materials.len() {
                    mesh.set_material(materials[idx].clone());
                }
                mesh
            })
            .collect();

        let centroid = if count > 0 { sum / count as f32 }
                       else         { Vector3::new(0.0, 0.0, 0.0) };

        let model = Rc::new(RefCell::new(
            SimpleModel::with_centroid(meshes, RenderMode::Triangles, centroid),
        ));

        ModelCache::insert(cache_key.to_string(), model.clone());
        model
    }

    /// Convenience wrapper: derive the format hint from the path extension
    /// and forward to `load_from_bytes`.
    pub fn load_from_bytes_with_path(path: &str, bytes: &[u8]) -> Rc<RefCell<SimpleModel>> {
        let hint = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("obj");
        Self::load_from_bytes(path, bytes, hint)
    }

    fn process_mesh(ai_mesh: &AiMesh) -> Mesh {
        let mut positions  = Vec::new();
        let mut normals    = Vec::new();
        let mut texcoords  = Vec::new();
        let mut indices    = Vec::new();

        for v in &ai_mesh.vertices {
            positions.extend_from_slice(&[v.x, v.y, v.z]);
        }
        if !ai_mesh.normals.is_empty() {
            for n in &ai_mesh.normals {
                normals.extend_from_slice(&[n.x, n.y, n.z]);
            }
        }
        if let Some(Some(coords)) = ai_mesh.texture_coords.first() {
            for t in coords.iter() {
                texcoords.push(t.x);
                texcoords.push(t.y);
            }
        }
        for f in &ai_mesh.faces {
            for idx in &f.0 { indices.push(*idx as u32); }
        }

        Mesh::from_raw(positions, normals, texcoords, indices)
    }

    fn process_material(ai_material: &AiMaterial) -> Material {
        let mut mat = Material::default();
        for prop in &ai_material.properties {
            match prop.key.as_str() {
                "$clr.diffuse" => {
                    if let russimp_ng::material::PropertyTypeInfo::FloatArray(v) = &prop.data {
                        if v.len() >= 3 {
                            mat.diffuse_color = Vector4::new(v[0], v[1], v[2], 1.0);
                        }
                    }
                }
                "$clr.specular" => {
                    if let russimp_ng::material::PropertyTypeInfo::FloatArray(v) = &prop.data {
                        if v.len() >= 3 {
                            mat.specular_color = Vector4::new(v[0], v[1], v[2], 1.0);
                        }
                    }
                }
                _ => {}
            }
        }
        mat
    }
}
