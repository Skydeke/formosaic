//! Model loader — parses mesh data from raw bytes using assimp.

use std::cell::RefCell;
use std::collections::HashMap;
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
use crate::opengl::fbos::simple_texture::SimpleTexture;
use crate::opengl::textures::texture::Texture;

pub struct ModelLoader;

impl ModelLoader {
    pub fn load_from_bytes(cache_key: &str, bytes: &[u8], hint: &str) -> Rc<RefCell<SimpleModel>> {
        if let Some(model) = ModelCache::get(cache_key) {
            return model;
        }

        // Extract embedded textures BEFORE Assimp parses the file, directly
        // from the raw GLB bytes. russimp-ng does not expose scene.textures,
        // so we parse the GLB JSON+BIN chunks ourselves.
        let embedded = Self::extract_glb_textures(bytes);

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

        let materials: Vec<Material> = scene
            .materials
            .iter()
            .map(|m| Self::process_material(m, &embedded))
            .collect();

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

    // ── GLB embedded texture extraction ──────────────────────────────────────

    /// Parse a GLB file's JSON+BIN chunks and extract all embedded images.
    /// Returns a map from image index (0, 1, …) to an uploaded GL texture.
    /// Returns an empty map for non-GLB files or on any parse error.
    fn extract_glb_textures(bytes: &[u8]) -> HashMap<usize, Rc<dyn Texture>> {
        let mut map = HashMap::new();

        // GLB magic: 0x46546c67 ("glTF" little-endian)
        if bytes.len() < 12 {
            return map;
        }
        let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        if magic != 0x46546c67 {
            return map; // not a GLB file
        }

        // JSON chunk starts at offset 12
        if bytes.len() < 20 {
            return map;
        }
        let json_len = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]) as usize;
        let json_type = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        if json_type != 0x4E4F534A {
            return map; // first chunk is not JSON
        }
        let json_end = 20 + json_len;
        if bytes.len() < json_end {
            return map;
        }
        let gltf: serde_json::Value = match serde_json::from_slice(&bytes[20..json_end]) {
            Ok(v) => v,
            Err(e) => {
                log::warn!("[ModelLoader] GLB JSON parse failed: {}", e);
                return map;
            }
        };

        // BIN chunk starts right after JSON chunk
        let bin_header_start = json_end;
        if bytes.len() < bin_header_start + 8 {
            return map;
        }
        let bin_len = u32::from_le_bytes([
            bytes[bin_header_start],
            bytes[bin_header_start + 1],
            bytes[bin_header_start + 2],
            bytes[bin_header_start + 3],
        ]) as usize;
        let bin_type = u32::from_le_bytes([
            bytes[bin_header_start + 4],
            bytes[bin_header_start + 5],
            bytes[bin_header_start + 6],
            bytes[bin_header_start + 7],
        ]);
        if bin_type != 0x004E4942 {
            return map; // second chunk is not BIN
        }
        let bin_start = bin_header_start + 8;
        let bin_end = bin_start + bin_len;
        if bytes.len() < bin_end {
            return map;
        }
        let bin = &bytes[bin_start..bin_end];

        // Extract each image referenced by bufferView
        let images = match gltf["images"].as_array() {
            Some(a) => a,
            None => return map,
        };
        let buffer_views = match gltf["bufferViews"].as_array() {
            Some(a) => a,
            None => return map,
        };

        for (img_idx, img) in images.iter().enumerate() {
            let bv_idx = match img["bufferView"].as_u64() {
                Some(i) => i as usize,
                None => continue,
            };
            if bv_idx >= buffer_views.len() {
                continue;
            }
            let bv = &buffer_views[bv_idx];
            let bv_offset = bv["byteOffset"].as_u64().unwrap_or(0) as usize;
            let bv_length = match bv["byteLength"].as_u64() {
                Some(l) => l as usize,
                None => continue,
            };
            if bv_offset + bv_length > bin.len() {
                continue;
            }

            let img_bytes = &bin[bv_offset..bv_offset + bv_length];

            // Decode with the `image` crate
            use image::io::Reader as ImageReader;
            use std::io::Cursor;
            let decoded = (|| -> Option<(u32, u32, Vec<u8>)> {
                let reader = ImageReader::new(Cursor::new(img_bytes))
                    .with_guessed_format()
                    .ok()?;
                let img = reader.decode().ok()?;
                let rgba = img.into_rgba8();
                let (w, h) = (rgba.width(), rgba.height());
                Some((w, h, rgba.into_raw()))
            })();

            match decoded {
                Some((w, h, rgba)) => {
                    let gl_tex = Self::upload_rgba_texture(w, h, &rgba);
                    log::info!("[ModelLoader] GLB image {} uploaded ({}×{})", img_idx, w, h);
                    map.insert(img_idx, Rc::new(gl_tex) as Rc<dyn Texture>);
                }
                None => {
                    log::warn!("[ModelLoader] GLB image {}: decode failed", img_idx);
                }
            }
        }

        map
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
    fn process_material(
        ai_material: &AiMaterial,
        embedded: &HashMap<usize, Rc<dyn Texture>>,
    ) -> Material {
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
                "$tex.file" | "$tex.diffuse" => {
                    if let russimp_ng::material::PropertyTypeInfo::String(path) = &prop.data {
                        if let Some(rest) = path.strip_prefix('*') {
                            if let Ok(idx) = rest.parse::<usize>() {
                                if let Some(tex) = embedded.get(&idx) {
                                    // TODO: handle multiple diffuse textures
                                    log::debug!("[TODO] set diffuse texture idx={}", idx);
                                    mat.diffuse_texture = Some(tex.clone());
                                }
                            }
                        }
                    }
                }
                "$tex.normal" => {
                    if let russimp_ng::material::PropertyTypeInfo::String(path) = &prop.data {
                        if let Some(rest) = path.strip_prefix('*') {
                            if let Ok(idx) = rest.parse::<usize>() {
                                if let Some(tex) = embedded.get(&idx) {
                                    // TODO: store normal texture in Material
                                    log::debug!("[TODO] set normal texture idx={}", idx);
                                }
                            }
                        }
                    }
                }
                "$tex.specular" => {
                    if let russimp_ng::material::PropertyTypeInfo::String(path) = &prop.data {
                        if let Some(rest) = path.strip_prefix('*') {
                            if let Ok(idx) = rest.parse::<usize>() {
                                if let Some(tex) = embedded.get(&idx) {
                                    // TODO: store specular texture in Material
                                    log::debug!("[TODO] set specular texture idx={}", idx);
                                }
                            }
                        }
                    }
                }
                "$tex.emissive" => {
                    if let russimp_ng::material::PropertyTypeInfo::String(path) = &prop.data {
                        if let Some(rest) = path.strip_prefix('*') {
                            if let Ok(idx) = rest.parse::<usize>() {
                                if let Some(tex) = embedded.get(&idx) {
                                    // TODO: store emissive texture in Material
                                    log::debug!("[TODO] set emissive texture idx={}", idx);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        mat
    }
}
