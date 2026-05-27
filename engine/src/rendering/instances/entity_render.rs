use std::cell::RefCell;
use std::rc::Rc;

use cgmath::Vector3;

use crate::architecture::models::material::Material;
use crate::architecture::models::model::Model;
use crate::architecture::scene::entity::scene_object::SceneObject;
use crate::architecture::scene::node::node::NodeBehavior;
use crate::architecture::scene::scene_context::SceneContext;
use crate::opengl::shaders::uniform::{
    UniformAdapter, UniformBoolean, UniformFloat, UniformInt, UniformMatrix4Array, UniformTexture,
};
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

        shader_program.add_per_instance_uniform(Box::new(UniformAdapter {
            uniform: UniformMatrix4::new("uModel"),
            extractor: Box::new(|state: &RenderState<T>| {
                let entity = state.instance().unwrap();
                let entity_matrix = entity.transform().get_matrix();
                let mesh_matrix = entity
                    .get_model()
                    .mesh_transform(state.instance_mesh_idx() as usize)
                    .unwrap_or_else(|| cgmath::Matrix4::from_scale(1.0));
                entity_matrix * mesh_matrix
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

        shader_program.add_per_instance_uniform(Box::new(UniformAdapter {
            uniform: UniformInt::new("uBoneCount"),
            extractor: Box::new(|state: &RenderState<T>| {
                state
                    .instance()
                    .map(|e| e.get_model().bone_matrices().len() as i32)
                    .unwrap_or(0)
            }),
        }));

        shader_program.add_per_instance_uniform(Box::new(UniformAdapter {
            uniform: UniformMatrix4Array::new("uBones", 64),
            extractor: Box::new(|state: &RenderState<T>| {
                state
                    .instance()
                    .map(|e| e.get_model().bone_matrices().to_vec())
                    .unwrap_or_default()
            }),
        }));

        log::info!("EntityRenderer initialized successfully");
        Ok(Self { shader_program })
    }
}

/// Key that determines whether two draw calls share the same GL state
/// (culling, blending, texture binding).  Draw calls with the same key
/// are batched together to minimise driver state changes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct MaterialKey {
    cull_backface: bool,
    alpha_blend: bool, // Blend vs. opaque/mask
    alpha_mask: bool,  // Has non-trivial alpha cutoff
    diffuse_tex_id: Option<u32>,
}

fn material_key(mat: Option<&Material>) -> MaterialKey {
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
                        draw_calls.push(DrawCall {
                            node: Rc::clone(node),
                            mesh_idx: i,
                            key: material_key(mat),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::architecture::models::material::AlphaMode;

    fn make_mat(cull_backface: bool, alpha_mode: AlphaMode) -> Material {
        Material {
            cull_backface,
            alpha_mode,
            ..Material::default()
        }
    }

    #[test]
    fn material_key_handles_culling() {
        let with_cull = make_mat(true, AlphaMode::Opaque);
        let no_cull = make_mat(false, AlphaMode::Opaque);
        assert_ne!(material_key(Some(&with_cull)), material_key(Some(&no_cull)));
        assert!(material_key(Some(&with_cull)).cull_backface);
        assert!(!material_key(Some(&no_cull)).cull_backface);
    }

    #[test]
    fn material_key_handles_alpha_blend() {
        let opaque = make_mat(true, AlphaMode::Opaque);
        let blend = make_mat(true, AlphaMode::Blend);
        assert_ne!(material_key(Some(&opaque)), material_key(Some(&blend)));
        let key = material_key(Some(&blend));
        assert!(key.alpha_blend);
        assert!(!key.alpha_mask);
    }

    #[test]
    fn material_key_handles_alpha_mask() {
        let opaque = make_mat(true, AlphaMode::Opaque);
        let mask = make_mat(true, AlphaMode::Mask(0.5));
        assert_ne!(material_key(Some(&opaque)), material_key(Some(&mask)));
        let key = material_key(Some(&mask));
        assert!(!key.alpha_blend);
        assert!(key.alpha_mask);
    }

    #[test]
    fn material_key_none() {
        assert_eq!(
            material_key(None),
            MaterialKey {
                cull_backface: false,
                alpha_blend: false,
                alpha_mask: false,
                diffuse_tex_id: None,
            }
        );
    }

    #[test]
    fn material_key_groups_identical() {
        let a = make_mat(true, AlphaMode::Opaque);
        let b = make_mat(true, AlphaMode::Opaque);
        assert_eq!(material_key(Some(&a)), material_key(Some(&b)));
    }

    #[test]
    fn material_sort_groups_same_keys_together() {
        let mat_a = make_mat(false, AlphaMode::Blend);
        let mat_b = make_mat(true, AlphaMode::Opaque);
        let key_a = material_key(Some(&mat_a));
        let key_b = material_key(Some(&mat_b));

        // Interleaved keys — after sorting all key_a items come first, then all key_b
        let mut keys = vec![key_b.clone(), key_a.clone(), key_b.clone(), key_a.clone()];
        keys.sort_by(|a, b| a.cmp(b));

        assert_eq!(keys[0], key_a);
        assert_eq!(keys[1], key_a);
        assert_eq!(keys[2], key_b);
        assert_eq!(keys[3], key_b);
    }
}
