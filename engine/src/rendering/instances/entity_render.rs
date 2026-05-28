use std::cell::RefCell;
use std::rc::Rc;

use cgmath::Matrix4;
use cgmath::Vector3;

use crate::architecture::models::material::Material;
use crate::architecture::models::model::Model;
use crate::architecture::scene::entity::scene_object::SceneObject;
use crate::architecture::scene::node::node::NodeBehavior;
use crate::architecture::scene::scene_context::SceneContext;
use crate::opengl::shaders::uniform::{
    UniformAdapter, UniformBoolean, UniformFloat, UniformInt, UniformMatrix4Array, UniformTexture,
};
use crate::opengl::shaders::render_state::ModelRenderData;
use crate::opengl::shaders::{RenderState, ShaderProgram, UniformMatrix4, UniformVec3};
use crate::rendering::abstracted::irenderer::IRenderer;

pub struct EntityRenderer<T: SceneObject> {
    shader_program: ShaderProgram<T>,
}

const DEFAULT_VERT: &str = include_str!("../../../assets/shaders/basic.vert.glsl");
const DEFAULT_FRAG: &str = include_str!("../../../assets/shaders/basic.frag.glsl");

impl<T: SceneObject + 'static> EntityRenderer<T> {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_shaders(DEFAULT_VERT, DEFAULT_FRAG)
    }

    pub fn with_shaders(
        vertex_src: &str,
        fragment_src: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        log::info!("Initializing EntityRenderer");
        let mut shader_program = ShaderProgram::<T>::from_sources(vertex_src, fragment_src)?;

        shader_program.add_per_instance_uniform(Box::new(UniformAdapter {
            uniform: UniformMatrix4::new("uVP"),
            extractor: Box::new(|state: &RenderState<T>| {
                *state.camera().get_projection_view_matrix()
            }),
        }));

        // uModel: read pre-resolved data from RenderState (no get_model needed).
        shader_program.add_per_instance_uniform(Box::new(UniformAdapter {
            uniform: UniformMatrix4::new("uModel"),
            extractor: Box::new(|state: &RenderState<T>| {
                let entity = state.instance().unwrap();
                let entity_matrix = entity.transform().get_matrix();
                if state.is_skinned() {
                    entity_matrix
                } else {
                    entity_matrix * state.mesh_transform()
                }
            }),
        }));

        shader_program.add_per_instance_uniform(Box::new(UniformAdapter {
            uniform: UniformTexture::new("albedoTex", 0),
            extractor: Box::new(|state: &RenderState<T>| {
                state
                    .mesh_material()
                    .and_then(|mat| mat.diffuse_texture.clone())
            }),
        }));

        shader_program.add_per_instance_uniform(Box::new(UniformAdapter {
            uniform: UniformVec3::new("albedoConst"),
            extractor: Box::new(|state: &RenderState<T>| {
                state
                    .mesh_material()
                    .map(|mat| mat.diffuse_color.truncate())
                    .unwrap_or(Vector3::new(1.0, 1.0, 1.0))
            }),
        }));

        shader_program.add_per_instance_uniform(Box::new(UniformAdapter {
            uniform: UniformFloat::new("uOpacity"),
            extractor: Box::new(|state: &RenderState<T>| {
                state
                    .mesh_material()
                    .map(|mat| mat.diffuse_color.w)
                    .unwrap_or(1.0)
            }),
        }));

        shader_program.add_per_instance_uniform(Box::new(UniformAdapter {
            uniform: UniformFloat::new("uAlphaCutoff"),
            extractor: Box::new(|state: &RenderState<T>| {
                match state.mesh_material().map(|mat| mat.alpha_mode) {
                    Some(crate::architecture::models::material::AlphaMode::Mask(cutoff)) => cutoff,
                    _ => 0.01,
                }
            }),
        }));

        shader_program.add_per_instance_uniform(Box::new(UniformAdapter {
            uniform: UniformVec3::new("uCameraPos"),
            extractor: Box::new(|state: &RenderState<T>| state.camera().transform.position),
        }));

        shader_program.add_per_instance_uniform(Box::new(UniformAdapter {
            uniform: UniformBoolean::new("isAlbedoMapped"),
            extractor: Box::new(|state: &RenderState<T>| {
                state
                    .mesh_material()
                    .and_then(|mat| mat.diffuse_texture.as_ref())
                    .is_some()
            }),
        }));

        shader_program.add_per_instance_uniform(Box::new(UniformAdapter {
            uniform: UniformBoolean::new("uHasVertexColors"),
            extractor: Box::new(|state: &RenderState<T>| state.has_vertex_colors()),
        }));

        shader_program.add_per_instance_uniform(Box::new(UniformAdapter {
            uniform: UniformFloat::new("uMetallicFactor"),
            extractor: Box::new(|state: &RenderState<T>| {
                state
                    .mesh_material()
                    .map(|mat| mat.metallic_factor)
                    .unwrap_or(0.0)
            }),
        }));

        shader_program.add_per_instance_uniform(Box::new(UniformAdapter {
            uniform: UniformFloat::new("uRoughnessFactor"),
            extractor: Box::new(|state: &RenderState<T>| {
                state
                    .mesh_material()
                    .map(|mat| mat.roughness_factor)
                    .unwrap_or(0.5)
            }),
        }));

        // uBoneCount: read from pre-resolved RenderState data.
        shader_program.add_per_instance_uniform(Box::new(UniformAdapter {
            uniform: UniformInt::new("uBoneCount"),
            extractor: Box::new(|state: &RenderState<T>| state.bone_count()),
        }));

        // uBones: read from pre-resolved RenderState data.
        shader_program.add_per_instance_uniform(Box::new(UniformAdapter {
            uniform: UniformMatrix4Array::new("uBones", 64),
            extractor: Box::new(|state: &RenderState<T>| state.bone_matrices().to_vec()),
        }));

        log::info!("EntityRenderer initialized successfully");
        Ok(Self { shader_program })
    }
}

/// Key that determines whether two draw calls share the same GL state
/// (culling, blending, texture binding).  Draw calls with the same key
/// are batched together to minimise driver state changes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MaterialKey {
    pub cull_backface: bool,
    pub alpha_blend: bool,
    pub alpha_mask: bool,
    pub diffuse_tex_id: Option<u32>,
}

pub fn material_key(mat: Option<&Material>) -> MaterialKey {
    match mat {
        Some(m) => MaterialKey {
            cull_backface: m.cull_backface,
            alpha_blend: matches!(
                m.alpha_mode,
                crate::architecture::models::material::AlphaMode::Blend
            ),
            alpha_mask: matches!(
                m.alpha_mode,
                crate::architecture::models::material::AlphaMode::Mask(_)
            ),
            diffuse_tex_id: m.diffuse_texture.as_ref().map(|t| t.get_id()),
        },
        None => MaterialKey {
            cull_backface: false,
            alpha_blend: false,
            alpha_mask: false,
            diffuse_tex_id: None,
        },
    }
}

/// Owned draw-call descriptor — no borrows, so it can be collected and sorted freely.
struct DrawCall {
    node: Rc<RefCell<dyn NodeBehavior>>,
    mesh_idx: usize,
    key: MaterialKey,
    /// Pre-resolved from the model's RefCell<..> at draw-call build time,
    /// so uniform extractors never need to touch the model RefCell during rendering.
    model_data: ModelRenderData,
}

impl<T: SceneObject + 'static> IRenderer for EntityRenderer<T> {
    fn render(&mut self, context: &SceneContext) {
        let camera = context.get_camera();

        if let Some(scene) = context.scene() {
            let entity_nodes = scene.collect_nodes_of_type::<T>();

            // ── Collect all draw calls ──────────────────────────────────────
            let mut draw_calls: Vec<DrawCall> = Vec::with_capacity(entity_nodes.len() * 4);
            for node in &entity_nodes {
                let node_ref = node.borrow();
                if let Some(entity) = node_ref.as_any().downcast_ref::<T>() {
                    let model = entity.model();
                    let model_ref = model.borrow();
                    let mesh_count = model_ref.get_mesh_count();

                    for i in 0..mesh_count {
                        let mat = model_ref.get_material(i);
                        let meshes = model_ref.get_meshes();
                        let is_skinned = meshes
                            .get(i)
                            .map(|m| m.is_skinned())
                            .unwrap_or(false);
                        let mesh_transform = model_ref
                            .mesh_transform(i)
                            .unwrap_or_else(|| Matrix4::from_scale(1.0));
                        let bone_matrices = if is_skinned {
                            model_ref.bone_matrices(i).to_vec()
                        } else {
                            Vec::new()
                        };
                        let bone_count = bone_matrices.len() as i32;
                        draw_calls.push(DrawCall {
                            node: Rc::clone(node),
                            mesh_idx: i,
                            key: material_key(mat),
                            model_data: ModelRenderData {
                                mesh_transform,
                                is_skinned,
                                bone_matrices,
                                bone_count,
                            },
                        });
                    }
                }
            }

            if draw_calls.is_empty() {
                return;
            }

            // ── Sort by material key to batch same-state draw calls ─────
            draw_calls.sort_by(|a, b| a.key.cmp(&b.key));

            let camera_ref = camera.borrow();
            self.shader_program.bind();

            let mut last_key: Option<MaterialKey> = None;
            for dc in &draw_calls {
                // Re-borrow the entity and its model from the scene graph.
                let node_ref = dc.node.borrow();
                let entity = node_ref.as_any().downcast_ref::<T>().unwrap();
                let model = entity.model();
                let model_ref = model.borrow();
                let material = model_ref.get_material(dc.mesh_idx);
                let has_vc = model_ref.has_vertex_colors(dc.mesh_idx);

                // Only change GL state when the material actually changes.
                if last_key.as_ref() != Some(&dc.key) {
                    self.prepare_material(material);
                    last_key = Some(dc.key.clone());
                }

                let render_state = RenderState::new_preresolved(
                    self,
                    entity,
                    &camera_ref,
                    dc.mesh_idx,
                    material,
                    has_vc,
                    &dc.model_data,
                );
                self.shader_program
                    .update_per_instance_uniforms(&render_state);

                model_ref.bind_and_configure(dc.mesh_idx);
                model_ref.render(&render_state, dc.mesh_idx);
                model_ref.unbind(dc.mesh_idx);
            }

            self.shader_program.unbind();
        }
    }

    fn finish(&mut self) {}
}

impl<T: SceneObject> Drop for EntityRenderer<T> {
    fn drop(&mut self) {
        log::info!("EntityRenderer dropped");
    }
}
