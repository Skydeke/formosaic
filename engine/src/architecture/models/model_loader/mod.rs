//! Model loader — parses mesh data from raw bytes using assimp.
//!
//! Supports the full set of texture types that russimp-ng exposes:
//! diffuse/base-color, normal, metallic-roughness (ORM), emissive,
//! occlusion, specular, and lightmap.

pub mod data;
pub mod incremental;

pub use data::*;
pub use incremental::IncrementalModelBuilder;

use std::collections::HashMap;

use cgmath::{Matrix4, Quaternion, SquareMatrix, Vector3, Vector4};
use russimp_ng::material::{Material as AiMaterial, PropertyTypeInfo, TextureType};
use russimp_ng::mesh::Mesh as AiMesh;
use russimp_ng::node::Node as AiNode;
use russimp_ng::scene::{PostProcess, Scene};

use crate::architecture::models::animation::{
    AnimationClip, BoneChannel, PositionKey, RotationKey, ScalingKey,
};
use crate::architecture::models::material::{AlphaMode, Material};
use crate::architecture::models::model_cache::ModelCache;
use crate::architecture::models::simple_model::SimpleModel;
use crate::architecture::models::skeleton::{BoneData, Skeleton};
use crate::opengl::constants::render_mode::RenderMode;
use crate::opengl::fbos::simple_texture::SimpleTexture;
use crate::opengl::objects::pbo::{supports_pbo, Pbo};
use crate::opengl::textures::texture::Texture;

use std::cell::RefCell;
use std::rc::Rc;

pub struct ModelLoader;

impl ModelLoader {
    pub fn load_from_bytes(
        cache_key: &str,
        bytes: &[u8],
        hint: &str,
    ) -> Result<Rc<RefCell<SimpleModel>>, String> {
        if let Some(model) = ModelCache::get(cache_key) {
            return Ok(model);
        }
        let data = Self::prepare_from_bytes(cache_key, bytes, hint)?;
        let model = data.build();
        ModelCache::insert(cache_key.to_string(), model.clone());
        Ok(model)
    }

    pub fn prepare_from_bytes_with_path(path: &str, bytes: &[u8]) -> Result<ModelLoadData, String> {
        let hint = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("obj");
        Self::prepare_from_bytes(path, bytes, hint)
    }

    pub fn prepare_from_bytes(
        cache_key: &str,
        bytes: &[u8],
        hint: &str,
    ) -> Result<ModelLoadData, String> {
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
        .map_err(|e| format!("Failed to parse model '{}': {:?}", cache_key, e))?;

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

        let materials: Vec<PreparedMaterial> = scene
            .materials
            .iter()
            .map(Self::process_material_prepared)
            .collect();

        let mut mesh_transforms: Vec<Option<Matrix4<f32>>> = vec![None; scene.meshes.len()];
        if let Some(root) = &scene.root {
            Self::collect_node_transforms(root, Matrix4::from_scale(1.0), &mut mesh_transforms);
        }

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
            .enumerate()
            .map(|(mesh_idx, m)| {
                // Accumulate centroid in world space by applying the mesh's node
                // transform.  Without this, hierarchical models (where body parts
                // are positioned via node transforms) produce a centroid near the
                // scene origin rather than the visual centre of the model, causing
                // the orbit camera to target empty space instead of the character.
                let node_xform = mesh_transforms
                    .get(mesh_idx)
                    .copied()
                    .flatten()
                    .unwrap_or_else(|| Matrix4::from_scale(1.0));
                for v in &m.vertices {
                    let local = cgmath::Vector4::new(v.x, v.y, v.z, 1.0);
                    let world = node_xform * local;
                    sum += Vector3::new(world.x, world.y, world.z);
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

        let has_bones = meshes
            .iter()
            .any(|m| m.bone_indices.iter().any(|bi| bi[0] >= 0));
        let skeleton = if has_bones {
            Self::build_skeleton(&scene, &meshes)
        } else {
            None
        };

        let animations: Vec<AnimationClip> = scene
            .animations
            .iter()
            .map(Self::extract_animation)
            .collect();

        Ok(ModelLoadData {
            meshes,
            materials,
            centroid: Some(centroid),
            mesh_transforms,
            render_mode: RenderMode::Triangles,
            skeleton,
            animations,
        })
    }

    fn build_skeleton(scene: &Scene, _prepared_meshes: &[PreparedMesh]) -> Option<Skeleton> {
        // Collect ALL bones across ALL meshes.  Different meshes may have
        // different offset matrices for the same bone (because they are in
        // different local spaces / node transforms).  We store every offset
        // so each mesh uses the correct one.
        let mut bone_names_ordered: Vec<String> = Vec::new();
        // Per-bone: one offset per mesh (indexed by mesh_idx).
        // Initialised to None; set to Some for meshes that reference the bone.
        let mut bone_offsets: HashMap<String, Vec<Option<Matrix4<f32>>>> = HashMap::new();
        let mesh_count = scene.meshes.len();
        for (mesh_idx, ai_mesh) in scene.meshes.iter().enumerate() {
            for bone in &ai_mesh.bones {
                let name = bone.name.clone();
                if !bone_offsets.contains_key(&name) {
                    bone_names_ordered.push(name.clone());
                    bone_offsets.insert(name.clone(), vec![None; mesh_count]);
                }
                if let Some(slot) = bone_offsets.get_mut(&name) {
                    slot[mesh_idx] = Some(Self::ai_matrix_to_cg(&bone.offset_matrix));
                }
            }
        }

        if bone_names_ordered.is_empty() {
            return None;
        }

        let name_to_idx: HashMap<&str, usize> = bone_names_ordered
            .iter()
            .enumerate()
            .map(|(i, n)| (n.as_str(), i))
            .collect();

        fn find_armature_root(
            node: &Rc<russimp_ng::node::Node>,
            bone_name: &str,
        ) -> Option<Rc<russimp_ng::node::Node>> {
            for child in node.children.borrow().iter() {
                if child.name == bone_name {
                    return Some(node.clone());
                }
            }
            for child in node.children.borrow().iter() {
                if let Some(found) = find_armature_root(child, bone_name) {
                    return Some(found);
                }
            }
            None
        }

        let armature_root: Option<Rc<russimp_ng::node::Node>> =
            bone_names_ordered.first().and_then(|first_bone| {
                scene
                    .root
                    .as_ref()
                    .and_then(|root| find_armature_root(root, first_bone))
            });
        let search_root = armature_root.as_ref().or_else(|| scene.root.as_ref());

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

        fn find_bind_local(node: &russimp_ng::node::Node, bone_name: &str) -> Option<Matrix4<f32>> {
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
            let offsets = &bone_offsets[bone_name.as_str()];
            // Unwrap each per-mesh slot (every bone should have at least one mesh referencing it)
            let offset_matrices: Vec<Matrix4<f32>> = offsets
                .iter()
                .map(|o| o.unwrap_or_else(|| Matrix4::identity()))
                .collect();
            let bind_local_transform = if let Some(root) = search_root {
                find_bind_local(root, bone_name).unwrap_or_else(|| Matrix4::identity())
            } else {
                Matrix4::identity()
            };
            let parent_index = if let Some(root) = search_root {
                find_parent(root, bone_name, &name_to_idx)
            } else {
                None
            };
            bone_data_vec.push(BoneData {
                name: bone_name.clone(),
                bind_local_transform,
                offset_matrices,
                parent_index,
            });
        }

        // Find the skeleton root's own ancestor transform (e.g. the Armature
        // node's accumulated world matrix). Assimp offset matrices are relative
        // to the scene root, but the bone hierarchy in compute_final_matrices
        // starts at the skeleton root — so we need to plug the gap here.
        fn find_root_ancestor(
            node: &Rc<russimp_ng::node::Node>,
            bone_name: &str,
            accumulated: Matrix4<f32>,
        ) -> Option<Matrix4<f32>> {
            let node_world = accumulated * ModelLoader::ai_matrix_to_cg(&node.transformation);
            for child in node.children.borrow().iter() {
                if child.name == bone_name {
                    return Some(node_world);
                }
                if let Some(r) = find_root_ancestor(child, bone_name, node_world) {
                    return Some(r);
                }
            }
            None
        }

        let ancestor = bone_names_ordered.first().and_then(|first_bone| {
            scene
                .root
                .as_ref()
                .and_then(|root| find_root_ancestor(root, first_bone, Matrix4::identity()))
        });
        let mut skel = Skeleton::new(bone_data_vec, mesh_count);
        if let Some(t) = ancestor {
            skel.root_ancestor_transform = t;
        }
        Some(skel)
    }

    fn extract_animation(ai_anim: &russimp_ng::animation::Animation) -> AnimationClip {
        let channels: Vec<BoneChannel> = ai_anim
            .channels
            .iter()
            .map(|ch| BoneChannel {
                bone_name: ch.name.clone(),
                position_keys: ch
                    .position_keys
                    .iter()
                    .map(|k| PositionKey {
                        time: k.time,
                        value: Vector3::new(k.value.x, k.value.y, k.value.z),
                    })
                    .collect(),
                rotation_keys: ch
                    .rotation_keys
                    .iter()
                    .map(|k| RotationKey {
                        time: k.time,
                        value: Quaternion::new(k.value.w, k.value.x, k.value.y, k.value.z),
                    })
                    .collect(),
                scaling_keys: ch
                    .scaling_keys
                    .iter()
                    .map(|k| ScalingKey {
                        time: k.time,
                        value: Vector3::new(k.value.x, k.value.y, k.value.z),
                    })
                    .collect(),
            })
            .collect();

        AnimationClip {
            name: ai_anim.name.clone(),
            duration_ticks: ai_anim.duration,
            ticks_per_second: ai_anim.ticks_per_second,
            channels,
        }
    }

    pub(crate) fn convert_texture(tex: &russimp_ng::material::Texture) -> Option<PreparedTexture> {
        use image::ImageFormat;
        use std::io::Cursor;

        match &tex.data {
            russimp_ng::material::DataContent::Bytes(bytes) => {
                if tex.height > 0 && tex.width > 0 {
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

    pub(crate) fn upload_prepared_texture(tex: PreparedTexture) -> Rc<dyn Texture> {
        Rc::new(Self::upload_rgba_texture(tex.width, tex.height, &tex.rgba))
    }

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
                    log::error!(
                        "[ModelLoader] GL error after PBO texture upload: 0x{:X}",
                        err
                    );
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

    fn process_mesh_prepared(
        ai_mesh: &AiMesh,
        name_to_global_idx: &HashMap<String, usize>,
    ) -> PreparedMesh {
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
                    let slot = bone_weights[vid].iter().position(|w| *w == 0.0);
                    if let Some(s) = slot {
                        bone_indices[vid][s] = global_idx;
                        bone_weights[vid][s] = vw.weight;
                    }
                }
            }
        }

        for weights in bone_weights.iter_mut() {
            let sum: f32 = weights.iter().sum();
            if sum > 0.0 {
                for w in weights.iter_mut() {
                    *w /= sum;
                }
            } else if sum == 0.0 {
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
            matrix.a1, matrix.b1, matrix.c1, matrix.d1, matrix.a2, matrix.b2, matrix.c2, matrix.d2,
            matrix.a3, matrix.b3, matrix.c3, matrix.d3, matrix.a4, matrix.b4, matrix.c4, matrix.d4,
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
                "$mat.twosided" | "$mat.twoSided" | "$mat.two_sided" => match &prop.data {
                    PropertyTypeInfo::IntegerArray(v) => {
                        if !v.is_empty() {
                            mat.cull_backface = v[0] == 0;
                        }
                    }
                    PropertyTypeInfo::FloatArray(v) => {
                        if !v.is_empty() {
                            mat.cull_backface = v[0] == 0.0;
                        }
                    }
                    PropertyTypeInfo::String(s) => {
                        mat.cull_backface = s != "true" && s != "1";
                    }
                    _ => {}
                },
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

#[cfg(test)]
mod tests {
    use super::*;
    use russimp_ng::Matrix4x4;

    fn identity4() -> Matrix4x4 {
        Matrix4x4 {
            a1: 1.0,
            a2: 0.0,
            a3: 0.0,
            a4: 0.0,
            b1: 0.0,
            b2: 1.0,
            b3: 0.0,
            b4: 0.0,
            c1: 0.0,
            c2: 0.0,
            c3: 1.0,
            c4: 0.0,
            d1: 0.0,
            d2: 0.0,
            d3: 0.0,
            d4: 1.0,
        }
    }

    fn scale999() -> Matrix4x4 {
        Matrix4x4 {
            a1: 999.0,
            a2: 0.0,
            a3: 0.0,
            a4: 0.0,
            b1: 0.0,
            b2: 999.0,
            b3: 0.0,
            b4: 0.0,
            c1: 0.0,
            c2: 0.0,
            c3: 999.0,
            c4: 0.0,
            d1: 0.0,
            d2: 0.0,
            d3: 0.0,
            d4: 1.0,
        }
    }

    fn node(name: &str, xform: Matrix4x4) -> Rc<russimp_ng::node::Node> {
        Rc::new(russimp_ng::node::Node {
            name: name.to_string(),
            children: RefCell::new(Vec::new()),
            meshes: Vec::new(),
            metadata: None,
            transformation: xform,
            parent: std::rc::Weak::new(),
        })
    }

    fn bone(name: &str) -> russimp_ng::bone::Bone {
        russimp_ng::bone::Bone {
            name: name.to_string(),
            offset_matrix: identity4(),
            weights: Vec::new(),
        }
    }

    #[test]
    fn build_skeleton_avoids_name_collision_with_mesh_nodes() {
        let root = node("RootNode", identity4());
        let armature = node("CharacterArmature", identity4());
        let bone_root = node("Root", identity4());
        let bone_body = node("Body", identity4());
        let mesh_body = node("Body", scale999());
        let mesh_head = node("Head", identity4());

        bone_root.children.borrow_mut().push(bone_body.clone());
        armature.children.borrow_mut().push(bone_root.clone());
        root.children.borrow_mut().push(armature.clone());
        root.children.borrow_mut().push(mesh_body.clone());
        root.children.borrow_mut().push(mesh_head.clone());

        let scene = russimp_ng::scene::Scene {
            root: Some(root),
            meshes: vec![russimp_ng::mesh::Mesh {
                name: "TestMesh".into(),
                bones: vec![bone("Root"), bone("Body")],
                ..Default::default()
            }],
            materials: Vec::new(),
            animations: Vec::new(),
            cameras: Vec::new(),
            lights: Vec::new(),
            metadata: None,
            flags: 0,
        };

        let skel = ModelLoader::build_skeleton(&scene, &[]).unwrap();

        let root_idx = skel.bones.iter().position(|b| b.name == "Root").unwrap();
        let body_idx = skel.bones.iter().position(|b| b.name == "Body").unwrap();

        assert_eq!(
            skel.bones[body_idx].parent_index,
            Some(root_idx),
            "Body bone parent should be Root (armature subtree search)"
        );
        assert_eq!(
            skel.bones[root_idx].parent_index, None,
            "Root bone should have no parent"
        );

        let body_transform = skel.bones[body_idx].bind_local_transform;
        assert!(
            (body_transform.x.x - 1.0).abs() < 1e-5,
            "Body bind_local_transform should be identity (from bone), not scale=999 (from mesh node). Got x.x={}",
            body_transform.x.x
        );
    }
}
