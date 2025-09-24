use std::cell::RefCell;
use std::rc::Rc;

use cgmath::{Vector3, Vector4};
use russimp::material::Material as AiMaterial;
use russimp::mesh::Mesh as AiMesh;
use russimp::scene::{PostProcess, Scene};

use crate::engine::architecture::models::material::Material;
use crate::engine::architecture::models::mesh::Mesh;
use crate::engine::architecture::models::model_cache::ModelCache;
use crate::engine::architecture::models::simple_model::SimpleModel;
use crate::opengl::constants::data_type::DataType;
use crate::opengl::constants::render_mode::RenderMode;
use crate::opengl::constants::vbo_usage::VboUsage;
use crate::opengl::objects::attribute::Attribute;
use crate::opengl::objects::data_buffer::DataBuffer;
use crate::opengl::objects::index_buffer::IndexBuffer;
use crate::opengl::objects::vao::Vao;

/// Cross-platform model loader
pub struct ModelLoader;

impl ModelLoader {
    /// Load a model from a path (Linux: filesystem, Android: APK assets)
    pub fn load(path: &str) -> Rc<RefCell<SimpleModel>> {
        if let Some(model) = ModelCache::get(path) {
            return model;
        }

        // Not cached → load as before
        let bytes = Self::load_asset_bytes(path);

        let extension = std::path::Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("obj");

        let scene = Scene::from_buffer(
            &bytes,
            vec![
                PostProcess::Triangulate,
                PostProcess::GenerateNormals,
                PostProcess::ImproveCacheLocality,
                PostProcess::OptimizeMeshes,
                PostProcess::CalculateTangentSpace,
                PostProcess::PreTransformVertices,
                PostProcess::EmbedTextures,
            ],
            extension,
        )
        .unwrap_or_else(|_| panic!("Failed to load model '{}'", path));

        let mut sum = Vector3::new(0.0, 0.0, 0.0);
        let mut count = 0usize;

        let materials: Vec<Material> = scene
            .materials
            .iter()
            .map(|ai_mat| Self::process_material(ai_mat))
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
                let mat_index = m.material_index as usize;
                if mat_index < materials.len() {
                    mesh.set_material(materials[mat_index].clone());
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

        // Insert into cache
        ModelCache::insert(path.to_string(), model.clone());
        model
    }

    #[cfg(target_os = "android")]
    fn load_asset_bytes(path: &str) -> Vec<u8> {
        use jni::objects::{JObject, JValue};
        use jni::JavaVM;

        let ctx = ndk_context::android_context();

        // Get JNI environment
        let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) }.unwrap();
        let mut env = vm.attach_current_thread().unwrap();

        // Get the activity and asset manager
        let activity = unsafe { JObject::from_raw(ctx.context().cast()) };
        let asset_manager = env
            .call_method(
                activity,
                "getAssets",
                "()Landroid/content/res/AssetManager;",
                &[],
            )
            .unwrap()
            .l()
            .unwrap();

        // Open the asset
        let path_jstring = env.new_string(path).unwrap();
        let input_stream = env
            .call_method(
                asset_manager,
                "open",
                "(Ljava/lang/String;)Ljava/io/InputStream;",
                &[JValue::Object(&path_jstring)],
            )
            .unwrap()
            .l()
            .unwrap();

        // Get available bytes
        let available = env
            .call_method(&input_stream, "available", "()I", &[])
            .unwrap()
            .i()
            .unwrap() as usize;

        // Read all bytes
        let byte_array = env.new_byte_array(available as i32).unwrap();
        let bytes_read = env
            .call_method(
                &input_stream,
                "read",
                "([B)I",
                &[JValue::Object(&byte_array)],
            )
            .unwrap()
            .i()
            .unwrap();

        // Convert to Rust Vec (JNI uses i8, we need u8)
        let mut buffer_i8 = vec![0i8; bytes_read as usize];
        env.get_byte_array_region(&byte_array, 0, &mut buffer_i8[..])
            .unwrap();
        let buffer: Vec<u8> = buffer_i8.into_iter().map(|b| b as u8).collect();

        // Close the stream
        let _ = env.call_method(&input_stream, "close", "()V", &[]);

        buffer
    }

    #[cfg(not(target_os = "android"))]
    fn load_asset_bytes(path: &str) -> Vec<u8> {
        std::fs::read(std::path::Path::new("assets/3d").join(path))
            .unwrap_or_else(|_| panic!("Failed to read file '{}'", path))
    }

    /// Convert an AiMesh into our Mesh + VBOs
    fn process_mesh(ai_mesh: &AiMesh) -> Mesh {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut texcoords = Vec::new();
        let mut indices = Vec::new();

        // vertices
        for v in &ai_mesh.vertices {
            positions.extend_from_slice(&[v.x, v.y, v.z]);
        }

        // normals
        if !ai_mesh.normals.is_empty() {
            for n in &ai_mesh.normals {
                normals.extend_from_slice(&[n.x, n.y, n.z]);
            }
        }

        // texcoords (only channel 0)
        if let Some(Some(coords)) = ai_mesh.texture_coords.first() {
            for t in coords.iter() {
                texcoords.push(t.x);
                texcoords.push(t.y);
            }
        }

        // indices
        for f in &ai_mesh.faces {
            for idx in &f.0 {
                indices.push(*idx as i32);
            }
        }

        // ---- Buffers ----
        let mut pos_buffer = DataBuffer::new(VboUsage::StaticDraw);
        pos_buffer.store_float(0, &positions);

        let mut vao = Vao::create();
        let pos_attr = Attribute::of(0, 3, DataType::Float, false);
        vao.load_data_buffer(Rc::new(pos_buffer), &[pos_attr]);

        if !texcoords.is_empty() {
            let mut tex_buffer = DataBuffer::new(VboUsage::StaticDraw);
            tex_buffer.store_float(0, &texcoords);
            let tex_attr = Attribute::of(1, 2, DataType::Float, false);
            vao.load_data_buffer(Rc::new(tex_buffer), &[tex_attr]);
        }

        if !normals.is_empty() {
            let mut normal_buffer = DataBuffer::new(VboUsage::StaticDraw);
            normal_buffer.store_float(0, &normals);
            let normal_attr = Attribute::of(2, 3, DataType::Float, false);
            vao.load_data_buffer(Rc::new(normal_buffer), &[normal_attr]);
        }

        let mut indices_buffer = IndexBuffer::new(VboUsage::StaticDraw);
        indices_buffer.store_int(0, &indices);
        vao.load_index_buffer(Rc::new(indices_buffer), true);

        Mesh::from_vao(vao)
    }

    fn process_material(ai_material: &AiMaterial) -> Material {
        let mut mat = Material::default();

        for prop in &ai_material.properties {
            match prop.key.as_str() {
                "$clr.diffuse" => {
                    if let russimp::material::PropertyTypeInfo::FloatArray(values) = &prop.data {
                        if values.len() >= 3 {
                            mat.diffuse_color = Vector4::new(values[0], values[1], values[2], 1.0);
                        }
                    }
                }
                "$clr.specular" => {
                    if let russimp::material::PropertyTypeInfo::FloatArray(values) = &prop.data {
                        if values.len() >= 3 {
                            mat.specular_color = Vector4::new(values[0], values[1], values[2], 1.0);
                        }
                    }
                }
                // handle transparency, emissive, etc.
                _ => {}
            }
        }

        mat
    }
}
