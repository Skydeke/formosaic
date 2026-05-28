use cgmath::{Matrix4, Vector3, Vector4};
use std::cell::RefCell;
use std::rc::Rc;

use crate::architecture::models::animation::AnimationClip;
use crate::architecture::models::material::{AlphaMode, Material};
use crate::architecture::models::mesh::Mesh;
use crate::architecture::models::simple_model::SimpleModel;
use crate::architecture::models::skeleton::Skeleton;
use crate::opengl::constants::render_mode::RenderMode;

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
    pub fn into_material(self) -> Material {
        Material {
            name: None,
            diffuse_color: self.diffuse_color,
            specular_color: self.specular_color,
            ambient_color: self.ambient_color,
            emissive_color: self.emissive_color,
            transparent_color: Vector4::new(0.0, 0.0, 0.0, 0.0),
            metallic_factor: self.metallic_factor,
            roughness_factor: self.roughness_factor,
            diffuse_texture: self
                .diffuse_texture
                .map(super::ModelLoader::upload_prepared_texture),
            normal_texture: self
                .normal_texture
                .map(super::ModelLoader::upload_prepared_texture),
            metallic_roughness_texture: self
                .metallic_roughness_texture
                .map(super::ModelLoader::upload_prepared_texture),
            emissive_texture: self
                .emissive_texture
                .map(super::ModelLoader::upload_prepared_texture),
            occlusion_texture: self
                .occlusion_texture
                .map(super::ModelLoader::upload_prepared_texture),
            specular_texture: self
                .specular_texture
                .map(super::ModelLoader::upload_prepared_texture),
            cull_backface: self.cull_backface,
            alpha_mode: self.alpha_mode,
            emissive_strength: self.emissive_strength,
        }
    }
}

impl ModelLoadData {
    pub fn build(self) -> Rc<RefCell<SimpleModel>> {
        let ModelLoadData {
            meshes,
            materials,
            centroid,
            mesh_transforms,
            render_mode,
            skeleton,
            animations,
        } = self;
        let meshes: Vec<Mesh> = meshes
            .into_iter()
            .map(|m| {
                let mut mesh = Mesh::from_raw(
                    m.positions,
                    m.normals,
                    m.texcoords,
                    m.indices,
                    m.colors,
                    m.bone_indices,
                    m.bone_weights,
                );
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
        )
        .expect("ModelLoadData::build: at least one mesh must be in the loaded model");
        model.set_animation_data(skeleton, animations);
        Rc::new(RefCell::new(model))
    }
}
