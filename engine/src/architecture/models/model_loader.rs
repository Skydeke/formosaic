//! Model loader — parses mesh data from raw bytes using assimp.
//!
//! Supports the full set of texture types that russimp-ng exposes:
//! diffuse/base-color, normal, metallic-roughness (ORM), emissive,
//! occlusion, specular, and lightmap.  All are stored in the engine
//! Material; the shader decides which to actually sample.

use std::cell::RefCell;
use std::rc::Rc;

use cgmath::{Matrix4, Vector3, Vector4};
use russimp_ng::material::{Material as AiMaterial, PropertyTypeInfo, TextureType};
use russimp_ng::mesh::Mesh as AiMesh;
use russimp_ng::scene::{PostProcess, Scene};
use russimp_ng::node::Node as AiNode;

use crate::architecture::models::material::{AlphaMode, Material};
use crate::architecture::models::mesh::Mesh;
use crate::architecture::models::model_cache::ModelCache;
use crate::architecture::models::simple_model::SimpleModel;
use crate::opengl::constants::render_mode::RenderMode;
use crate::opengl::fbos::simple_texture::SimpleTexture;
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
}

/// CPU-side payload used while the loader parses bytes on a worker thread.
#[derive(Clone, Debug)]
pub struct ModelLoadData {
    pub meshes: Vec<PreparedMesh>,
    pub materials: Vec<PreparedMaterial>,
    pub centroid: Option<Vector3<f32>>,
    pub mesh_transforms: Vec<Matrix4<f32>>,
    pub render_mode: RenderMode,
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
        let ModelLoadData { meshes, materials, centroid, mesh_transforms, render_mode } = self;
        let meshes: Vec<Mesh> = meshes
            .into_iter()
            .map(|m| {
                let mut mesh = Mesh::from_raw(m.positions, m.normals, m.texcoords, m.indices, m.colors);
                if let Some(prepared) = materials.get(m.material_index) {
                    mesh.set_material(prepared.clone().into_material());
                }
                mesh
            })
            .collect();

        Rc::new(RefCell::new(SimpleModel::with_mesh_transforms(
            meshes,
            render_mode,
            centroid,
            mesh_transforms,
        )))
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

        let meshes: Vec<PreparedMesh> = scene
            .meshes
            .iter()
            .map(|m| {
                for v in &m.vertices {
                    sum += Vector3::new(v.x, v.y, v.z);
                    count += 1;
                }
                Self::process_mesh_prepared(m)
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

        ModelLoadData {
            meshes,
            materials,
            centroid: Some(centroid),
            mesh_transforms,
            render_mode: RenderMode::Triangles,
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
    fn upload_rgba_texture(w: u32, h: u32, rgba: &[u8]) -> SimpleTexture {
        let tex = SimpleTexture::create();
        log::info!(
            "[ModelLoader] upload_rgba_texture: id={}, {}x{}",
            tex.get_id(),
            w,
            h
        );
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

        log::debug!(
            "[ModelLoader] mesh: verts={} norms={} uvs={} colors={}",
            ai_mesh.vertices.len(),
            ai_mesh.normals.len(),
            texcoords.len() / 2,
            colors.len() / 4,
        );

        Mesh::from_raw(positions, normals, texcoords, indices, colors)
    }

    fn process_mesh_prepared(ai_mesh: &AiMesh) -> PreparedMesh {
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

        PreparedMesh {
            positions,
            normals,
            texcoords,
            indices,
            colors,
            material_index: ai_mesh.material_index as usize,
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
