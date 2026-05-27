//! Model loader — parses mesh data from raw bytes using assimp.
//!
//! Supports the full set of texture types that russimp-ng exposes:
//! diffuse/base-color, normal, metallic-roughness (ORM), emissive,
//! occlusion, specular, and lightmap.  All are stored in the engine
//! Material; the shader decides which to actually sample.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use cgmath::{Matrix4, Quaternion, SquareMatrix, Vector3, Vector4};
use russimp_ng::material::{Material as AiMaterial, PropertyTypeInfo, TextureType};
use russimp_ng::mesh::Mesh as AiMesh;
use russimp_ng::scene::{PostProcess, Scene};
use russimp_ng::node::Node as AiNode;

use crate::architecture::models::animation::{AnimationClip, BoneChannel, PositionKey, RotationKey, ScalingKey};
use crate::architecture::models::material::{AlphaMode, Material};
use crate::architecture::models::mesh::Mesh;
use crate::architecture::models::model_cache::ModelCache;
use crate::architecture::models::simple_model::SimpleModel;
use crate::architecture::models::skeleton::{BoneData, Skeleton};
use crate::opengl::constants::render_mode::RenderMode;
use crate::opengl::fbos::simple_texture::SimpleTexture;
use crate::opengl::objects::pbo::{supports_pbo, Pbo};
use crate::opengl::textures::texture::Texture;

pub struct ModelLoader;

#[derive(Clone, Debug)]
pub struct PreparedMaterial {
    pub diffuse_color: Vector4<f32>,
    pub specular_color: Vector4<f32>,
    pub ambient_color: Vector4<f32>,
    pub emissive_color: Vector4<f32>,
    pub metallic_factor: f32,
    pub roughness_factor: f32,
    pub diffuse_texture: Option<PreparedTexture>,
    pub normal_texture: Option<PreparedTexture>,
    pub metallic_roughness_texture: Option<PreparedTexture>,
    pub emissive_texture: Option<PreparedTexture>,
    pub occlusion_texture: Option<PreparedTexture>,
    pub specular_texture: Option<PreparedTexture>,
    pub cull_backface: bool,
    pub alpha_mode: AlphaMode,
    pub emissive_strength: f32,
}

#[derive(Clone, Debug)]
pub struct PreparedTexture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct PreparedMesh {
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
    pub texcoords: Vec<f32>,
    pub indices: Vec<u32>,
    pub colors: Vec<f32>,
    pub material_index: usize,
    pub bone_indices: Vec<[i32; 4]>,
    pub bone_weights: Vec<[f32; 4]>,
}

/// CPU-side payload used while the loader parses bytes on a worker thread.
#[derive(Clone, Debug)]
pub struct ModelLoadData {
    pub meshes: Vec<PreparedMesh>,
    pub materials: Vec<PreparedMaterial>,
    pub centroid: Option<Vector3<f32>>,
    pub mesh_transforms: Vec<Matrix4<f32>>,
    pub render_mode: RenderMode,
    pub skeleton: Option<Skeleton>,
    pub animations: Vec<AnimationClip>,
}

impl PreparedMaterial {
    fn into_material(self) -> Material {
        Material {
            name: None,
            diffuse_color: self.diffuse_color,
            specular_color: self.specular_color,
            ambient_color: self.ambient_color,
            emissive_color: self.emissive_color,
            transparent_color: Vector4::new(0.0, 0.0, 0.0, 0.0),
            metallic_factor: self.metallic_factor,
            roughness_factor: self.roughness_factor,
            diffuse_texture: self.diffuse_texture.map(ModelLoader::upload_prepared_texture),
            normal_texture: self.normal_texture.map(ModelLoader::upload_prepared_texture),
            metallic_roughness_texture: self.metallic_roughness_texture.map(ModelLoader::upload_prepared_texture),
            emissive_texture: self.emissive_texture.map(ModelLoader::upload_prepared_texture),
            occlusion_texture: self.occlusion_texture.map(ModelLoader::upload_prepared_texture),
            specular_texture: self.specular_texture.map(ModelLoader::upload_prepared_texture),
            cull_backface: self.cull_backface,
            alpha_mode: self.alpha_mode,
            emissive_strength: self.emissive_strength,
        }
    }
}

impl ModelLoadData {
    pub fn build(self) -> Rc<RefCell<SimpleModel>> {
        let ModelLoadData { meshes, materials, centroid, mesh_transforms, render_mode, skeleton, animations } = self;
        let meshes: Vec<Mesh> = meshes
            .into_iter()
            .map(|m| {
                let mut mesh = Mesh::from_raw(m.positions, m.normals, m.texcoords, m.indices, m.colors, m.bone_indices, m.bone_weights);
                if let Some(prepared) = materials.get(m.material_index) {
                    mesh.set_material(prepared.clone().into_material());
                }
                mesh
            })
            .collect();

        let mut model = SimpleModel::with_mesh_transforms(
            meshes,
            render_mode,
            centroid,
            mesh_transforms,
        );
        model.set_animation_data(skeleton, animations);
        Rc::new(RefCell::new(model))
    }
}

/// Builds a SimpleModel incrementally across multiple frames so the
/// UI never freezes.  Work is split into two phases:
///
/// **Phase 1** – Build one mesh each frame (VAO / VBO creation, no textures).
/// **Phase 2** – Upload one material's textures each frame (glTexImage2D +
/// mipmap generation) and attach it to every mesh that references it.
///
/// After `build_next()` returns `true` you must NOT finalize in the same
/// frame — always wait one frame before calling `finish()` so the entropy
/// search and scramble don't pile on top of the last texture upload.
pub struct IncrementalModelBuilder {
    data: ModelLoadData,
    /// (material_index, Mesh without textures)
    built_meshes: Vec<(usize, Mesh)>,
    /// How many meshes have been built so far (phase-1 cursor).
    next_mesh: usize,
    /// How many materials have been uploaded so far (phase-2 cursor).
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
        if total == 0 { return 1.0; }
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
        // ── Phase 1: build mesh geometry (fast, <1 ms) ────────────────
        if self.next_mesh < self.data.meshes.len() {
            let m = &self.data.meshes[self.next_mesh];
            let mesh = Mesh::from_raw(
                m.positions.clone(),
                m.normals.clone(),
                m.texcoords.clone(),
                m.indices.clone(),
                m.colors.clone(),
                m.bone_indices.clone(),
                m.bone_weights.clone(),
            );
            self.built_meshes.push((m.material_index, mesh));
            self.next_mesh += 1;
            return false;
        }

        // ── Phase 2: upload one material's textures (5–15 ms) ─────────
        if self.next_material < self.data.materials.len() {
            let material = self.data.materials[self.next_material]
                .clone()
                .into_material();
            // Attach to every mesh that references this material index.
            for (mat_idx, mesh) in &mut self.built_meshes {
                if *mat_idx == self.next_material {
                    mesh.set_material(material.clone());
                }
            }
            self.next_material += 1;
            return false;
        }

        true // all done
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
        );
        model.set_animation_data(self.data.skeleton, self.data.animations);
        Rc::new(RefCell::new(model))
    }
}

impl ModelLoader {
    pub fn load_from_bytes(cache_key: &str, bytes: &[u8], hint: &str) -> Rc<RefCell<SimpleModel>> {
        if let Some(model) = ModelCache::get(cache_key) {
            return model;
        }
        let model = Self::prepare_from_bytes(cache_key, bytes, hint).build();
        ModelCache::insert(cache_key.to_string(), model.clone());
        model
    }

    pub fn prepare_from_bytes_with_path(path: &str, bytes: &[u8]) -> ModelLoadData {
        let hint = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("obj");
        Self::prepare_from_bytes(path, bytes, hint)
    }

    pub fn prepare_from_bytes(cache_key: &str, bytes: &[u8], hint: &str) -> ModelLoadData {
        let scene = Scene::from_buffer(
            bytes,
            vec![
                PostProcess::Triangulate,
                PostProcess::JoinIdenticalVertices,
                PostProcess::ImproveCacheLocality,
                PostProcess::EmbedTextures,
                PostProcess::FlipUVs,
            ],
            hint,
        )
        .unwrap_or_else(|e| panic!("Failed to parse model '{}': {:?}", cache_key, e));

        // Ensure the model has proper UV coordinates and normals
        let scene = &scene;
        for mesh in &scene.meshes {
            if mesh.vertices.is_empty() {
                log::warn!("[ModelLoader] mesh has no vertices");
            }
            if mesh.normals.is_empty() {
                log::warn!("[ModelLoader] mesh has no normals - lighting will be incorrect");
            }
            if mesh.texture_coords.is_empty() {
                log::warn!(
                    "[ModelLoader] mesh has no texture coordinates - texture mapping will not work"
                );
            }
        }

        let mut sum = Vector3::new(0.0f32, 0.0, 0.0);
        let mut count = 0usize;

        let materials: Vec<PreparedMaterial> = scene.materials.iter().map(Self::process_material_prepared).collect();

        let mut mesh_transforms: Vec<Option<Matrix4<f32>>> = vec![None; scene.meshes.len()];
        if let Some(root) = &scene.root {
            Self::collect_node_transforms(root, Matrix4::from_scale(1.0), &mut mesh_transforms);
        }

        // ── Build global bone index mapping before processing meshes ──
        // This ensures per-vertex bone indices reference global skeleton
        // indices (0, 1, 2, ...) rather than per-mesh local indices.
        // The ordering must match build_skeleton's bone ordering.
        let name_to_global_idx: HashMap<String, usize> = {
            let mut map = HashMap::new();
            let mut next = 0;
            for ai_mesh in &scene.meshes {
                for bone in &ai_mesh.bones {
                    if !map.contains_key(&bone.name) {
                        map.insert(bone.name.clone(), next);
                        next += 1;
                    }
                }
            }
            map
        };

        let meshes: Vec<PreparedMesh> = scene
            .meshes
            .iter()
            .map(|m| {
                for v in &m.vertices {
                    sum += Vector3::new(v.x, v.y, v.z);
                    count += 1;
                }
                Self::process_mesh_prepared(m, &name_to_global_idx)
            })
            .collect();

        let centroid = if count > 0 {
            sum / count as f32
        } else {
            Vector3::new(0.0, 0.0, 0.0)
        };

        let mesh_transforms = mesh_transforms
            .into_iter()
            .map(|m| m.unwrap_or_else(|| Matrix4::from_scale(1.0)))
            .collect();

        // ── Build skeleton from bone data ─────────────────────────────────
        let has_bones = meshes.iter().any(|m| m.bone_indices.iter().any(|bi| bi[0] >= 0));
        let skeleton = if has_bones {
            Self::build_skeleton(&scene, &meshes)
        } else {
            None
        };

        // ── Extract animations ────────────────────────────────────────────
        let animations: Vec<AnimationClip> = scene.animations.iter().map(Self::extract_animation).collect();

        ModelLoadData {
            meshes,
            materials,
            centroid: Some(centroid),
            mesh_transforms,
            render_mode: RenderMode::Triangles,
            skeleton,
            animations,
        }
    }

    /// Build a Skeleton from the Assimp scene's node hierarchy and per-mesh bone data.
    fn build_skeleton(scene: &Scene, _prepared_meshes: &[PreparedMesh]) -> Option<Skeleton> {
        // Collect unique bone data from scene meshes, preserving insertion order.
        let mut bone_names_ordered: Vec<String> = Vec::new();
        let mut bone_offsets: HashMap<String, Matrix4<f32>> = HashMap::new();
        for ai_mesh in &scene.meshes {
            for bone in &ai_mesh.bones {
                let name = bone.name.clone();
                if !bone_offsets.contains_key(&name) {
                    bone_names_ordered.push(name.clone());
                    bone_offsets.insert(name, Self::ai_matrix_to_cg(&bone.offset_matrix));
                }
            }
        }

        if bone_names_ordered.is_empty() {
            return None;
        }

        // Build name→index lookup
        let name_to_idx: HashMap<&str, usize> = bone_names_ordered.iter().enumerate().map(|(i, n)| (n.as_str(), i)).collect();

        // Recursive helper to find parent index for a bone
        fn find_parent(
            node: &russimp_ng::node::Node,
            bone_name: &str,
            name_to_idx: &HashMap<&str, usize>,
        ) -> Option<usize> {
            for child in node.children.borrow().iter() {
                if child.name == bone_name {
                    return name_to_idx.get(node.name.as_str()).copied();
                }
            }
            for child in node.children.borrow().iter() {
                if let Some(idx) = find_parent(child, bone_name, name_to_idx) {
                    return Some(idx);
                }
            }
            None
        }

        fn find_bind_local(
            node: &russimp_ng::node::Node,
            bone_name: &str,
        ) -> Option<Matrix4<f32>> {
            if node.name == bone_name {
                return Some(ModelLoader::ai_matrix_to_cg(&node.transformation));
            }
            for child in node.children.borrow().iter() {
                if let Some(x) = find_bind_local(child, bone_name) {
                    return Some(x);
                }
            }
            None
        }

        let mut bone_data_vec: Vec<BoneData> = Vec::with_capacity(bone_names_ordered.len());
        for bone_name in &bone_names_ordered {
            let offset = bone_offsets[bone_name.as_str()];
            let bind_local_transform = if let Some(root) = &scene.root {
                find_bind_local(root, bone_name).unwrap_or_else(|| Matrix4::identity())
            } else {
                Matrix4::identity()
            };
            let parent_index = if let Some(root) = &scene.root {
                find_parent(root, bone_name, &name_to_idx)
            } else {
                None
            };
            bone_data_vec.push(BoneData {
                name: bone_name.clone(),
                bind_local_transform,
                offset_matrix: offset,
                parent_index,
            });
        }

        Some(Skeleton::new(bone_data_vec))
    }

    /// Convert a russimp animation into our engine AnimationClip.
    fn extract_animation(ai_anim: &russimp_ng::animation::Animation) -> AnimationClip {
        let channels: Vec<BoneChannel> = ai_anim.channels.iter().map(|ch| {
            BoneChannel {
                bone_name: ch.name.clone(),
                position_keys: ch.position_keys.iter().map(|k| PositionKey {
                    time: k.time,
                    value: Vector3::new(k.value.x, k.value.y, k.value.z),
                }).collect(),
                rotation_keys: ch.rotation_keys.iter().map(|k| RotationKey {
                    time: k.time,
                    value: Quaternion::new(k.value.w, k.value.x, k.value.y, k.value.z),
                }).collect(),
                scaling_keys: ch.scaling_keys.iter().map(|k| ScalingKey {
                    time: k.time,
                    value: Vector3::new(k.value.x, k.value.y, k.value.z),
                }).collect(),
            }
        }).collect();

        AnimationClip {
            name: ai_anim.name.clone(),
            duration_ticks: ai_anim.duration,
            ticks_per_second: ai_anim.ticks_per_second,
            channels,
        }
    }

    // ── Texture conversion ────────────────────────────────────────────────────

    fn convert_texture(tex: &russimp_ng::material::Texture) -> Option<PreparedTexture> {
        use image::ImageFormat;
        use std::io::Cursor;

        match &tex.data {
            russimp_ng::material::DataContent::Bytes(bytes) => {
                // Assimp:
                // height > 0 => raw pixel data
                // height == 0 => compressed embedded image
                if tex.height > 0 {
                    let expected = (tex.width * tex.height * 4) as usize;

                    if bytes.len() != expected {
                        log::error!(
                            "[Texture] Raw size mismatch: got={}, expected={}",
                            bytes.len(),
                            expected
                        );
                        return None;
                    }

                    return Some(PreparedTexture {
                        width: tex.width,
                        height: tex.height,
                        rgba: bytes.to_vec(),
                    });
                }

                // Compressed embedded image
                let format_hint = tex
                    .ach_format_hint
                    .trim_matches(char::from(0))
                    .to_ascii_lowercase();

                log::debug!("[Texture] embedded format hint: '{}'", format_hint);

                let cursor = Cursor::new(bytes);

                let decoded = match format_hint.as_str() {
                    "png" => image::load(cursor, ImageFormat::Png).ok()?,
                    "jpg" | "jpeg" => image::load(cursor, ImageFormat::Jpeg).ok()?,
                    "bmp" => image::load(cursor, ImageFormat::Bmp).ok()?,
                    "tga" => image::load(cursor, ImageFormat::Tga).ok()?,

                    // fallback
                    _ => image::load_from_memory(bytes).ok()?,
                };

                let img = decoded.into_rgba8();

                Some(PreparedTexture {
                    width: img.width(),
                    height: img.height(),
                    rgba: img.into_raw(),
                })
            }

            russimp_ng::material::DataContent::Texel(texels) => {
                let mut rgba = Vec::with_capacity(texels.len() * 4);

                for t in texels {
                    rgba.push(t.r);
                    rgba.push(t.g);
                    rgba.push(t.b);
                    rgba.push(t.a);
                }

                Some(PreparedTexture {
                    width: tex.width,
                    height: tex.height,
                    rgba,
                })
            }
        }
    }

    fn upload_prepared_texture(tex: PreparedTexture) -> Rc<dyn Texture> {
        Rc::new(Self::upload_rgba_texture(tex.width, tex.height, &tex.rgba))
    }

    /// Upload raw RGBA bytes as GL_TEXTURE_2D and return a SimpleTexture.
    ///
    /// Uses a PBO (pixel-buffer-object) for asynchronous upload when the
    /// driver supports it (GLES 3.0+ / desktop GL 2.1+).  Falls back to
    /// the traditional synchronous path otherwise.
    fn upload_rgba_texture(w: u32, h: u32, rgba: &[u8]) -> SimpleTexture {
        let tex = SimpleTexture::create();
        let data_size = (w * h * 4) as usize;
        log::info!(
            "[ModelLoader] upload_rgba_texture: id={}, {}x{} ({} bytes)",
            tex.get_id(),
            w,
            h,
            data_size
        );

        if supports_pbo() {
            let mut pbo = Pbo::create();
            pbo.store_data(rgba);
            unsafe {
                gl::BindTexture(gl::TEXTURE_2D, tex.get_id());
                pbo.bind();
                gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA as i32,
                    w as i32,
                    h as i32,
                    0,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    std::ptr::null(),
                );
                pbo.unbind();
                gl::GenerateMipmap(gl::TEXTURE_2D);
                gl::TexParameteri(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_MIN_FILTER,
                    gl::LINEAR_MIPMAP_LINEAR as i32,
                );
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
                gl::BindTexture(gl::TEXTURE_2D, 0);
                let err = gl::GetError();
                if err != gl::NO_ERROR {
                    log::error!("[ModelLoader] GL error after PBO texture upload: 0x{:X}", err);
                }
            }
            pbo.delete();
        } else {
            unsafe {
                gl::BindTexture(gl::TEXTURE_2D, tex.get_id());
                gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA as i32,
                    w as i32,
                    h as i32,
                    0,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    rgba.as_ptr() as *const _,
                );
                gl::GenerateMipmap(gl::TEXTURE_2D);
                gl::TexParameteri(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_MIN_FILTER,
                    gl::LINEAR_MIPMAP_LINEAR as i32,
                );
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
                gl::BindTexture(gl::TEXTURE_2D, 0);
                let err = gl::GetError();
                if err != gl::NO_ERROR {
                    log::error!("[ModelLoader] GL error after texture upload: 0x{:X}", err);
                }
            }
        }
        tex
    }

    // ── Mesh processing ───────────────────────────────────────────────────────

    fn process_mesh(ai_mesh: &AiMesh) -> Mesh {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut texcoords = Vec::new();
        let mut indices = Vec::new();
        let mut colors = Vec::new();

        for v in &ai_mesh.vertices {
            positions.extend_from_slice(&[v.x, v.y, v.z]);
        }
        for n in &ai_mesh.normals {
            normals.extend_from_slice(&[n.x, n.y, n.z]);
        }
        if let Some(Some(coords)) = ai_mesh.texture_coords.first() {
            if coords.len() == ai_mesh.vertices.len() {
                for t in coords.iter() {
                    texcoords.push(t.x);
                    texcoords.push(t.y);
                }
            } else {
                log::warn!(
                    "[ModelLoader] UV count mismatch: verts={} uvs={}",
                    ai_mesh.vertices.len(),
                    coords.len()
                );
            }
        }
        for f in &ai_mesh.faces {
            for idx in &f.0 {
                indices.push(*idx as u32);
            }
        }
        if let Some(Some(chan)) = ai_mesh.colors.first() {
            for c in chan.iter() {
                colors.push(c.r);
                colors.push(c.g);
                colors.push(c.b);
                colors.push(c.a);
            }
        }

        // Extract bone data — same as process_mesh_prepared
        let vert_count = ai_mesh.vertices.len();
        let mut bone_indices: Vec<[i32; 4]> = vec![[-1, -1, -1, -1]; vert_count];
        let mut bone_weights: Vec<[f32; 4]> = vec![[0.0, 0.0, 0.0, 0.0]; vert_count];
        for (bone_idx, bone) in ai_mesh.bones.iter().enumerate() {
            for vw in &bone.weights {
                let vid = vw.vertex_id as usize;
                if vid < vert_count {
                    let slot = bone_weights[vid].iter().position(|w| *w == 0.0);
                    if let Some(s) = slot {
                        bone_indices[vid][s] = bone_idx as i32;
                        bone_weights[vid][s] = vw.weight;
                    }
                }
            }
        }
        for weights in bone_weights.iter_mut() {
            let sum: f32 = weights.iter().sum();
            if sum > 0.0 {
                for w in weights.iter_mut() { *w /= sum; }
            } else {
                weights[0] = 1.0;
            }
        }

        log::debug!(
            "[ModelLoader] mesh: verts={} norms={} uvs={} colors={} bones={}",
            ai_mesh.vertices.len(),
            ai_mesh.normals.len(),
            texcoords.len() / 2,
            colors.len() / 4,
            ai_mesh.bones.len(),
        );

        Mesh::from_raw(positions, normals, texcoords, indices, colors, bone_indices, bone_weights)
    }

    fn process_mesh_prepared(ai_mesh: &AiMesh, name_to_global_idx: &HashMap<String, usize>) -> PreparedMesh {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut texcoords = Vec::new();
        let mut indices = Vec::new();
        let mut colors = Vec::new();

        for v in &ai_mesh.vertices {
            positions.extend_from_slice(&[v.x, v.y, v.z]);
        }
        for n in &ai_mesh.normals {
            normals.extend_from_slice(&[n.x, n.y, n.z]);
        }
        if let Some(Some(coords)) = ai_mesh.texture_coords.first() {
            if coords.len() == ai_mesh.vertices.len() {
                for t in coords.iter() {
                    texcoords.push(t.x);
                    texcoords.push(t.y);
                }
            }
        }
        for f in &ai_mesh.faces {
            for idx in &f.0 {
                indices.push(*idx as u32);
            }
        }
        if let Some(Some(chan)) = ai_mesh.colors.first() {
            for c in chan.iter() {
                colors.push(c.r);
                colors.push(c.g);
                colors.push(c.b);
                colors.push(c.a);
            }
        }

        // ── Extract bone data ─────────────────────────────────────────────
        let vert_count = ai_mesh.vertices.len();
        let mut bone_indices: Vec<[i32; 4]> = vec![[-1, -1, -1, -1]; vert_count];
        let mut bone_weights: Vec<[f32; 4]> = vec![[0.0, 0.0, 0.0, 0.0]; vert_count];

        for (bone_idx, bone) in ai_mesh.bones.iter().enumerate() {
            let global_idx = name_to_global_idx
                .get(&bone.name)
                .copied()
                .unwrap_or(bone_idx) as i32;
            for vw in &bone.weights {
                let vid = vw.vertex_id as usize;
                if vid < vert_count {
                    // Find first empty slot (weight == 0.0)
                    let slot = bone_weights[vid].iter().position(|w| *w == 0.0);
                    if let Some(s) = slot {
                        bone_indices[vid][s] = global_idx;
                        bone_weights[vid][s] = vw.weight;
                    }
                }
            }
        }

        // Normalize weights so they sum to 1.0
        for weights in bone_weights.iter_mut() {
            let sum: f32 = weights.iter().sum();
            if sum > 0.0 {
                for w in weights.iter_mut() {
                    *w /= sum;
                }
            } else if sum == 0.0 {
                // No bones — set identity weight on bone 0
                weights[0] = 1.0;
            }
        }

        PreparedMesh {
            positions,
            normals,
            texcoords,
            indices,
            colors,
            material_index: ai_mesh.material_index as usize,
            bone_indices,
            bone_weights,
        }
    }

    fn ai_matrix_to_cg(matrix: &russimp_ng::Matrix4x4) -> Matrix4<f32> {
        Matrix4::new(
            matrix.a1, matrix.b1, matrix.c1, matrix.d1,
            matrix.a2, matrix.b2, matrix.c2, matrix.d2,
            matrix.a3, matrix.b3, matrix.c3, matrix.d3,
            matrix.a4, matrix.b4, matrix.c4, matrix.d4,
        )
    }

    fn collect_node_transforms(
        node: &Rc<AiNode>,
        parent_transform: Matrix4<f32>,
        mesh_transforms: &mut [Option<Matrix4<f32>>],
    ) {
        let local = Self::ai_matrix_to_cg(&node.transformation);
        let world = parent_transform * local;

        for mesh_idx in &node.meshes {
            let idx = *mesh_idx as usize;
            if let Some(slot) = mesh_transforms.get_mut(idx) {
                if slot.is_none() {
                    *slot = Some(world);
                }
            }
        }

        for child in node.children.borrow().iter() {
            Self::collect_node_transforms(child, world, mesh_transforms);
        }
    }

    // ── Material processing ───────────────────────────────────────────────────

    /// Build a full Material from Assimp properties and embedded textures.
    fn process_material(ai_material: &AiMaterial) -> Material {
        let mut mat = Material::default();

        // ── Scalar properties ─────────────────────────────────────────────
        for prop in &ai_material.properties {
            match prop.key.as_str() {
                "$clr.diffuse" | "$clr.base" => {
                    if let PropertyTypeInfo::FloatArray(v) = &prop.data {
                        if v.len() >= 3 {
                            let a = if v.len() >= 4 { v[3] } else { 1.0 };
                            mat.diffuse_color = Vector4::new(v[0], v[1], v[2], a);
                        }
                    }
                }
                "$clr.specular" => {
                    if let PropertyTypeInfo::FloatArray(v) = &prop.data {
                        if v.len() >= 3 {
                            mat.specular_color = Vector4::new(v[0], v[1], v[2], 1.0);
                        }
                    }
                }
                "$clr.ambient" => {
                    if let PropertyTypeInfo::FloatArray(v) = &prop.data {
                        if v.len() >= 3 {
                            mat.ambient_color = Vector4::new(v[0], v[1], v[2], 1.0);
                        }
                    }
                }
                "$clr.emissive" => {
                    if let PropertyTypeInfo::FloatArray(v) = &prop.data {
                        if v.len() >= 3 {
                            mat.emissive_color = Vector4::new(v[0], v[1], v[2], 1.0);
                        }
                    }
                }
                // glTF PBR metallic/roughness factors
                "$mat.gltf.pbrMetallicRoughness.metallicFactor" => {
                    if let PropertyTypeInfo::FloatArray(v) = &prop.data {
                        if !v.is_empty() {
                            mat.metallic_factor = v[0];
                        }
                    }
                }
                "$mat.gltf.pbrMetallicRoughness.roughnessFactor" => {
                    if let PropertyTypeInfo::FloatArray(v) = &prop.data {
                        if !v.is_empty() {
                            mat.roughness_factor = v[0];
                        }
                    }
                }
                // KHR_materials_emissive_strength
                "$mat.gltf.emissiveStrength" => {
                    if let PropertyTypeInfo::FloatArray(v) = &prop.data {
                        if !v.is_empty() {
                            mat.emissive_strength = v[0];
                        }
                    }
                }
                "$mat.opacity" => {
                    if let PropertyTypeInfo::FloatArray(v) = &prop.data {
                        if !v.is_empty() {
                            mat.diffuse_color.w = v[0];
                            if v[0] < 0.999 && matches!(mat.alpha_mode, AlphaMode::Opaque) {
                                mat.alpha_mode = AlphaMode::Blend;
                            }
                        }
                    }
                }
                "$mat.gltf.alphaMode" => {
                    if let PropertyTypeInfo::String(s) = &prop.data {
                        mat.alpha_mode = match s.as_str() {
                            "MASK" => AlphaMode::Mask(0.5),
                            "BLEND" => AlphaMode::Blend,
                            _ => AlphaMode::Opaque,
                        };
                    }
                }
                "$mat.gltf.alphaCutoff" => {
                    if let PropertyTypeInfo::FloatArray(v) = &prop.data {
                        if !v.is_empty() {
                            mat.alpha_mode = AlphaMode::Mask(v[0]);
                        }
                    }
                }
                "$mat.twosided" | "$mat.twoSided" | "$mat.two_sided" => {
                    match &prop.data {
                        PropertyTypeInfo::IntegerArray(v) => {
                            if !v.is_empty() { mat.cull_backface = v[0] == 0; }
                        }
                        PropertyTypeInfo::FloatArray(v) => {
                            if !v.is_empty() { mat.cull_backface = v[0] == 0.0; }
                        }
                        PropertyTypeInfo::String(s) => {
                            mat.cull_backface = s != "true" && s != "1";
                        }
                        _ => {}
                    }
                }
                _ => {
                    log::trace!("[MatProp] key='{}' (unhandled)", prop.key);
                }
            }
        }

        // ── Texture maps ──────────────────────────────────────────────────
        // Priority: glTF PBR types first, then legacy Phong fallbacks.

        // Albedo / base color
        mat.diffuse_texture = Self::load_tex(ai_material, TextureType::BaseColor)
            .or_else(|| Self::load_tex(ai_material, TextureType::Diffuse));

        // Normal map
        mat.normal_texture = Self::load_tex(ai_material, TextureType::Normals)
            .or_else(|| Self::load_tex(ai_material, TextureType::Height));

        // Metallic-roughness ORM (glTF packs them into one texture)
        mat.metallic_roughness_texture =
            Self::load_tex(ai_material, TextureType::Unknown) // russimp maps ORM to Unknown
                .or_else(|| Self::load_tex(ai_material, TextureType::Metalness))
                .or_else(|| Self::load_tex(ai_material, TextureType::Roughness));

        // Emissive
        mat.emissive_texture = Self::load_tex(ai_material, TextureType::Emissive);

        // Ambient occlusion (separate from ORM, when present)
        mat.occlusion_texture = Self::load_tex(ai_material, TextureType::LightMap)
            .or_else(|| Self::load_tex(ai_material, TextureType::AmbientOcclusion));

        // Specular (legacy)
        mat.specular_texture = Self::load_tex(ai_material, TextureType::Specular)
            .or_else(|| Self::load_tex(ai_material, TextureType::Shininess));

        // Log all texture types actually present for debugging
        for (ty, _) in &ai_material.textures {
            log::debug!("[Material] texture type present: {:?}", ty);
        }

        mat
    }

    fn process_material_prepared(ai_material: &AiMaterial) -> PreparedMaterial {
        let mut mat = Material::default();

        for prop in &ai_material.properties {
            match prop.key.as_str() {
                "$clr.diffuse" | "$clr.base" => {
                    if let PropertyTypeInfo::FloatArray(v) = &prop.data {
                        if v.len() >= 3 {
                            let a = if v.len() >= 4 { v[3] } else { 1.0 };
                            mat.diffuse_color = Vector4::new(v[0], v[1], v[2], a);
                        }
                    }
                }
                "$clr.specular" => {
                    if let PropertyTypeInfo::FloatArray(v) = &prop.data {
                        if v.len() >= 3 {
                            mat.specular_color = Vector4::new(v[0], v[1], v[2], 1.0);
                        }
                    }
                }
                "$clr.ambient" => {
                    if let PropertyTypeInfo::FloatArray(v) = &prop.data {
                        if v.len() >= 3 {
                            mat.ambient_color = Vector4::new(v[0], v[1], v[2], 1.0);
                        }
                    }
                }
                "$clr.emissive" => {
                    if let PropertyTypeInfo::FloatArray(v) = &prop.data {
                        if v.len() >= 3 {
                            mat.emissive_color = Vector4::new(v[0], v[1], v[2], 1.0);
                        }
                    }
                }
                "$mat.gltf.pbrMetallicRoughness.metallicFactor" => {
                    if let PropertyTypeInfo::FloatArray(v) = &prop.data {
                        if !v.is_empty() { mat.metallic_factor = v[0]; }
                    }
                }
                "$mat.gltf.pbrMetallicRoughness.roughnessFactor" => {
                    if let PropertyTypeInfo::FloatArray(v) = &prop.data {
                        if !v.is_empty() { mat.roughness_factor = v[0]; }
                    }
                }
                "$mat.gltf.emissiveStrength" => {
                    if let PropertyTypeInfo::FloatArray(v) = &prop.data {
                        if !v.is_empty() { mat.emissive_strength = v[0]; }
                    }
                }
                "$mat.opacity" => {
                    if let PropertyTypeInfo::FloatArray(v) = &prop.data {
                        if !v.is_empty() {
                            mat.diffuse_color.w = v[0];
                            if v[0] < 0.999 && matches!(mat.alpha_mode, AlphaMode::Opaque) {
                                mat.alpha_mode = AlphaMode::Blend;
                            }
                        }
                    }
                }
                "$mat.gltf.alphaMode" => {
                    if let PropertyTypeInfo::String(s) = &prop.data {
                        mat.alpha_mode = match s.as_str() {
                            "MASK" => AlphaMode::Mask(0.5),
                            "BLEND" => AlphaMode::Blend,
                            _ => AlphaMode::Opaque,
                        };
                    }
                }
                "$mat.gltf.alphaCutoff" => {
                    if let PropertyTypeInfo::FloatArray(v) = &prop.data {
                        if !v.is_empty() { mat.alpha_mode = AlphaMode::Mask(v[0]); }
                    }
                }
                "$mat.twosided" | "$mat.twoSided" | "$mat.two_sided" => {
                    match &prop.data {
                        PropertyTypeInfo::IntegerArray(v) => {
                            if !v.is_empty() { mat.cull_backface = v[0] == 0; }
                        }
                        PropertyTypeInfo::FloatArray(v) => {
                            if !v.is_empty() { mat.cull_backface = v[0] == 0.0; }
                        }
                        PropertyTypeInfo::String(s) => {
                            mat.cull_backface = s != "true" && s != "1";
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        PreparedMaterial {
            diffuse_color: mat.diffuse_color,
            specular_color: mat.specular_color,
            ambient_color: mat.ambient_color,
            emissive_color: mat.emissive_color,
            metallic_factor: mat.metallic_factor,
            roughness_factor: mat.roughness_factor,
            diffuse_texture: Self::load_tex_prepared(ai_material, TextureType::BaseColor)
                .or_else(|| Self::load_tex_prepared(ai_material, TextureType::Diffuse)),
            normal_texture: Self::load_tex_prepared(ai_material, TextureType::Normals)
                .or_else(|| Self::load_tex_prepared(ai_material, TextureType::Height)),
            metallic_roughness_texture: Self::load_tex_prepared(ai_material, TextureType::Unknown)
                .or_else(|| Self::load_tex_prepared(ai_material, TextureType::Metalness))
                .or_else(|| Self::load_tex_prepared(ai_material, TextureType::Roughness)),
            emissive_texture: Self::load_tex_prepared(ai_material, TextureType::Emissive),
            occlusion_texture: Self::load_tex_prepared(ai_material, TextureType::LightMap)
                .or_else(|| Self::load_tex_prepared(ai_material, TextureType::AmbientOcclusion)),
            specular_texture: Self::load_tex_prepared(ai_material, TextureType::Specular)
                .or_else(|| Self::load_tex_prepared(ai_material, TextureType::Shininess)),
            cull_backface: mat.cull_backface,
            alpha_mode: mat.alpha_mode,
            emissive_strength: mat.emissive_strength,
        }
    }

    /// Try to load a texture of a given type from this material.
    fn load_tex(mat: &AiMaterial, ty: TextureType) -> Option<Rc<dyn Texture>> {
        let tex_ref = mat.textures.get(&ty)?;
        let tex = tex_ref.borrow();
        let result = Self::convert_texture(&tex);
        if result.is_none() {
            log::warn!("[ModelLoader] failed to convert {:?} texture", ty);
        }
        result.map(|tex| Rc::new(Self::upload_rgba_texture(tex.width, tex.height, &tex.rgba)) as Rc<dyn Texture>)
    }

    fn load_tex_prepared(mat: &AiMaterial, ty: TextureType) -> Option<PreparedTexture> {
        let tex_ref = mat.textures.get(&ty)?;
        let tex = tex_ref.borrow();
        let result = Self::convert_texture(&tex);
        if result.is_none() {
            log::warn!("[ModelLoader] failed to convert {:?} texture", ty);
        }
        result
    }
}
