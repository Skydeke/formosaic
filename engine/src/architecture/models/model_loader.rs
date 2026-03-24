//! Model loader — parses mesh data from raw bytes using assimp.

use std::cell::RefCell;
use std::rc::Rc;

use cgmath::{Vector3, Vector4};
use russimp_ng::material::{Material as AiMaterial, TextureType};
use russimp_ng::mesh::Mesh as AiMesh;
use russimp_ng::scene::{PostProcess, Scene};

use crate::architecture::models::material::Material;
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

    fn convert_texture(tex: &russimp_ng::material::Texture) -> Option<Rc<dyn Texture>> {
        match &tex.data {
            russimp_ng::material::DataContent::Bytes(bytes) => {
                if tex.height > 0 {
                    // ── RAW DATA (not compressed!) ──
                    let expected_size = (tex.width * tex.height * 4) as usize;

                    if bytes.len() != expected_size {
                        log::error!(
                            "[Texture] Raw embedded texture size mismatch: got={}, expected={}",
                            bytes.len(),
                            expected_size
                        );
                        return None;
                    }

                    return Some(Rc::new(ModelLoader::upload_rgba_texture(
                        tex.width, tex.height, bytes,
                    )));
                }

                // ── COMPRESSED (PNG/JPG) ──
                use image::io::Reader as ImageReader;
                use std::io::Cursor;

                let img = ImageReader::new(Cursor::new(bytes))
                    .with_guessed_format()
                    .ok()?
                    .decode()
                    .ok()?
                    .into_rgba8();

                let (w, h) = (img.width(), img.height());
                let rgba = img.into_raw();

                Some(Rc::new(ModelLoader::upload_rgba_texture(w, h, &rgba)))
            }

            russimp_ng::material::DataContent::Texel(texels) => {
                // rarely used for embedded, but keep it
                let mut rgba = Vec::with_capacity(texels.len() * 4);
                for t in texels {
                    rgba.push(t.r);
                    rgba.push(t.g);
                    rgba.push(t.b);
                    rgba.push(t.a);
                }

                Some(Rc::new(ModelLoader::upload_rgba_texture(
                    tex.width, tex.height, &rgba,
                )))
            }
        }
    }

    /// Upload raw RGBA bytes as a GL_TEXTURE_2D and return a SimpleTexture.
    fn upload_rgba_texture(w: u32, h: u32, rgba: &[u8]) -> SimpleTexture {
        let tex = SimpleTexture::create();
        log::info!(
            "[ModelLoader] upload_rgba_texture: id={}, size={}x{}",
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
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
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
        if !ai_mesh.normals.is_empty() {
            for n in &ai_mesh.normals {
                normals.extend_from_slice(&[n.x, n.y, n.z]);
            }
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
        // First vertex-color channel (COLOR_0 in glTF/GLB)
        if let Some(Some(chan)) = ai_mesh.colors.first() {
            for c in chan.iter() {
                colors.push(c.r);
                colors.push(c.g);
                colors.push(c.b);
                colors.push(c.a);
            }
        }

        log::info!(
            "verts={} uvs={}",
            ai_mesh.vertices.len(),
            ai_mesh
                .texture_coords
                .first()
                .and_then(|x| x.as_ref())
                .map(|v| v.len())
                .unwrap_or(0)
        );

        Mesh::from_raw(positions, normals, texcoords, indices, colors)
    }

    // ── Material processing ───────────────────────────────────────────────────

    /// Build a material from Assimp properties.
    ///
    /// `embedded` maps GLB image indices to already-uploaded GL textures.
    /// When the material's `$tex.file` property is `"*N"`, we look up index N.
    fn process_material(ai_material: &AiMaterial) -> Material {
        let mut mat = Material::default();
        for prop in &ai_material.properties {
            log::debug!(
                "[MatProp] key='{}' data={}",
                prop.key,
                match &prop.data {
                    russimp_ng::material::PropertyTypeInfo::FloatArray(v) =>
                        format!("Float{:?}", &v[..v.len().min(4)]),
                    russimp_ng::material::PropertyTypeInfo::String(s) => format!("String({:?})", s),
                    russimp_ng::material::PropertyTypeInfo::IntegerArray(v) =>
                        format!("Int{:?}", &v[..v.len().min(4)]),
                    _ => "Buffer/Other".to_string(),
                }
            );
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
                "$clr.emissive" => {
                    if let russimp_ng::material::PropertyTypeInfo::FloatArray(v) = &prop.data {
                        if v.len() >= 3 {
                            mat.emissive_color = Vector4::new(v[0], v[1], v[2], 1.0);
                        }
                    }
                }
                "$mat.opacity" => {
                    if let russimp_ng::material::PropertyTypeInfo::FloatArray(v) = &prop.data {
                        if !v.is_empty() {
                            // TODO: store opacity somewhere in Material
                            log::debug!("[TODO] material opacity={}", v[0]);
                        }
                    }
                }
                "$mat.gltf.alphaMode" => {
                    if let russimp_ng::material::PropertyTypeInfo::String(s) = &prop.data {
                        // TODO: store alpha_mode in Material
                        log::debug!("[TODO] material alpha_mode={}", s);
                    }
                }
                "$mat.gltf.alphaCutoff" => {
                    if let russimp_ng::material::PropertyTypeInfo::FloatArray(v) = &prop.data {
                        if !v.is_empty() {
                            // TODO: store alpha_cutoff in Material
                            log::debug!("[TODO] material alpha_cutoff={}", v[0]);
                        }
                    }
                }
                _ => {}
            }
        }

        // ── TEXTURES (NEW, CLEAN WAY) ──

        // Diffuse / BaseColor
        if let Some(tex) = ai_material
            .textures
            .get(&TextureType::BaseColor)
            .or_else(|| ai_material.textures.get(&TextureType::Diffuse))
        {
            let tex = tex.borrow();
            if let Some(gl_tex) = Self::convert_texture(&tex) {
                mat.diffuse_texture = Some(gl_tex);
            }
        }

        // Normal map
        if let Some(tex) = ai_material.textures.get(&TextureType::Normals) {
            let tex = tex.borrow();
            if let Some(gl_tex) = Self::convert_texture(&tex) {
                mat.normal_texture = Some(gl_tex);
            }
        }

        // Specular
        // if let Some(tex) = ai_material.textures.get(&TextureType::Specular) {
        //     let tex = tex.borrow();
        //     if let Some(gl_tex) = convert_texture(&tex) {
        //         mat.specular_texture = Some(gl_tex);
        //     }
        // }

        mat
    }
}
