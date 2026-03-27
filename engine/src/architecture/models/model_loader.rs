//! Model loader — parses mesh data from raw bytes using assimp.
//!
//! Supports the full set of texture types that russimp-ng exposes:
//! diffuse/base-color, normal, metallic-roughness (ORM), emissive,
//! occlusion, specular, and lightmap.  All are stored in the engine
//! Material; the shader decides which to actually sample.

use std::cell::RefCell;
use std::rc::Rc;

use cgmath::{Vector3, Vector4};
use russimp_ng::material::{Material as AiMaterial, PropertyTypeInfo, TextureType};
use russimp_ng::mesh::Mesh as AiMesh;
use russimp_ng::scene::{PostProcess, Scene};

use crate::architecture::models::material::{AlphaMode, Material};
use crate::architecture::models::mesh::Mesh;
use crate::architecture::models::model_cache::ModelCache;
use crate::architecture::models::simple_model::SimpleModel;
use crate::opengl::constants::render_mode::RenderMode;
use crate::opengl::fbos::simple_texture::SimpleTexture;
use crate::opengl::textures::texture::Texture;

pub struct ModelLoader;

impl ModelLoader {
    pub fn load_from_bytes(cache_key: &str, bytes: &[u8], hint: &str) -> Rc<RefCell<SimpleModel>> {
        if let Some(model) = ModelCache::get(cache_key) {
            return model;
        }

        let scene = Scene::from_buffer(
            bytes,
            vec![
                PostProcess::Triangulate,
                PostProcess::GenerateNormals,
                PostProcess::ImproveCacheLocality,
                PostProcess::PreTransformVertices,
                PostProcess::EmbedTextures,
                PostProcess::FlipUVs,
            ],
            hint,
        )
        .unwrap_or_else(|e| panic!("Failed to parse model '{}': {:?}", cache_key, e));

        let mut sum = Vector3::new(0.0f32, 0.0, 0.0);
        let mut count = 0usize;

        let materials: Vec<Material> = scene.materials.iter().map(Self::process_material).collect();

        let meshes: Vec<Mesh> = scene
            .meshes
            .iter()
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

        let centroid = if count > 0 {
            sum / count as f32
        } else {
            Vector3::new(0.0, 0.0, 0.0)
        };

        let model = Rc::new(RefCell::new(SimpleModel::with_centroid(
            meshes,
            RenderMode::Triangles,
            centroid,
        )));

        ModelCache::insert(cache_key.to_string(), model.clone());
        model
    }

    pub fn load_from_bytes_with_path(path: &str, bytes: &[u8]) -> Rc<RefCell<SimpleModel>> {
        let hint = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("obj");
        Self::load_from_bytes(path, bytes, hint)
    }

    // ── Texture conversion ────────────────────────────────────────────────────

    fn convert_texture(tex: &russimp_ng::material::Texture) -> Option<Rc<dyn Texture>> {
        match &tex.data {
            russimp_ng::material::DataContent::Bytes(bytes) => {
                if tex.height > 0 {
                    // Raw (uncompressed) pixel data
                    let expected = (tex.width * tex.height * 4) as usize;
                    if bytes.len() != expected {
                        log::error!(
                            "[Texture] Raw size mismatch: got={}, expected={}",
                            bytes.len(),
                            expected
                        );
                        return None;
                    }
                    return Some(Rc::new(Self::upload_rgba_texture(
                        tex.width, tex.height, bytes,
                    )));
                }

                // Compressed (PNG / JPEG / …) — decode with the `image` crate
                use image::io::Reader as ImageReader;
                use std::io::Cursor;

                let img = ImageReader::new(Cursor::new(bytes))
                    .with_guessed_format()
                    .ok()?
                    .decode()
                    .ok()?
                    .into_rgba8();

                let (w, h) = (img.width(), img.height());
                Some(Rc::new(Self::upload_rgba_texture(w, h, &img.into_raw())))
            }

            russimp_ng::material::DataContent::Texel(texels) => {
                let mut rgba = Vec::with_capacity(texels.len() * 4);
                for t in texels {
                    rgba.push(t.r);
                    rgba.push(t.g);
                    rgba.push(t.b);
                    rgba.push(t.a);
                }
                Some(Rc::new(Self::upload_rgba_texture(
                    tex.width, tex.height, &rgba,
                )))
            }
        }
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
                "$mat.twosided" => {
                    if let PropertyTypeInfo::IntegerArray(v) = &prop.data {
                        if !v.is_empty() {
                            mat.cull_backface = v[0] == 0;
                        }
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

    /// Try to load a texture of a given type from this material.
    fn load_tex(mat: &AiMaterial, ty: TextureType) -> Option<Rc<dyn Texture>> {
        let tex_ref = mat.textures.get(&ty)?;
        let tex = tex_ref.borrow();
        let result = Self::convert_texture(&tex);
        if result.is_none() {
            log::warn!("[ModelLoader] failed to convert {:?} texture", ty);
        }
        result
    }
}
