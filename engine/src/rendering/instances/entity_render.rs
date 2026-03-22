use cgmath::Vector3;

use crate::architecture::models::material::Material;
use crate::architecture::models::model::Model;
use crate::architecture::scene::entity::scene_object::SceneObject;
use crate::architecture::scene::scene_context::SceneContext;
use crate::rendering::abstracted::irenderer::IRenderer;
use crate::opengl::shaders::uniform::{UniformBoolean, UniformTexture};
use crate::opengl::shaders::UniformVec3;
use crate::opengl::shaders::{uniform::UniformAdapter, RenderState, ShaderProgram, UniformMatrix4};
use std::cell::RefCell;
use std::rc::Rc;

pub struct EntityRenderer<T: SceneObject> {
    shader_program: ShaderProgram<T>,
}

const DEFAULT_VERT: &str = include_str!("../../../assets/shaders/basic.vert.glsl");
const DEFAULT_FRAG: &str = include_str!("../../../assets/shaders/basic.frag.glsl");

impl<T: SceneObject + 'static> EntityRenderer<T> {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_shaders(DEFAULT_VERT, DEFAULT_FRAG)
    }

    pub fn with_shaders(vertex_src: &str, fragment_src: &str) -> Result<Self, Box<dyn std::error::Error>> {
        log::info!("Initializing EntityRenderer");
        let mut shader_program = ShaderProgram::<T>::from_sources(vertex_src, fragment_src)?;

        shader_program.add_per_instance_uniform(Rc::new(RefCell::new(UniformAdapter {
            uniform: UniformMatrix4::new("uVP"),
            extractor: Box::new(|state: &RenderState<T>| {
                *state.camera().get_projection_view_matrix()
            }),
        })));

        shader_program.add_per_instance_uniform(Rc::new(RefCell::new(UniformAdapter {
            uniform: UniformMatrix4::new("uModel"),
            extractor: Box::new(|state: &RenderState<T>| {
                state.instance().unwrap().transform().get_matrix()
            }),
        })));

        shader_program.add_per_instance_uniform(Rc::new(RefCell::new(UniformAdapter {
            uniform: UniformTexture::new("albedoTex", 0),
            extractor: Box::new(|state: &RenderState<T>| {
                state.mesh_material()
                    .and_then(|mat| mat.diffuse_texture.clone())
            }),
        })));

        shader_program.add_per_instance_uniform(Rc::new(RefCell::new(UniformAdapter {
            uniform: UniformVec3::new("albedoConst"),
            extractor: Box::new(|state: &RenderState<T>| {
                state.mesh_material()
                    .map(|mat| mat.diffuse_color.truncate())
                    .unwrap_or(Vector3::new(1.0, 1.0, 1.0))
            }),
        })));

        shader_program.add_per_instance_uniform(Rc::new(RefCell::new(UniformAdapter {
            uniform: UniformBoolean::new("isAlbedoMapped"),
            extractor: Box::new(|state: &RenderState<T>| {
                state.mesh_material()
                    .and_then(|mat| mat.diffuse_texture.as_ref())
                    .is_some()
            }),
        })));

        shader_program.add_per_instance_uniform(Rc::new(RefCell::new(UniformAdapter {
            uniform: UniformBoolean::new("uHasVertexColors"),
            extractor: Box::new(|state: &RenderState<T>| {
                state.has_vertex_colors()
            }),
        })));

        log::info!("EntityRenderer initialized successfully");
        Ok(Self { shader_program })
    }
}

impl<T: SceneObject + 'static> IRenderer for EntityRenderer<T> {
    fn render(&mut self, context: &SceneContext) {
        let camera = context.get_camera();

        if let Some(scene) = context.scene() {
            let entity_nodes = scene.collect_nodes_of_type::<T>();

            unsafe { gl::Disable(gl::CULL_FACE); }

            for node in &entity_nodes {
                let node_ref = node.borrow();
                if let Some(entity) = node_ref.as_any().downcast_ref::<T>() {
                    let camera_ref = camera.borrow();
                    self.shader_program.bind();

                    let model = entity.model();

                    // ── Resolve everything we need from the model BEFORE
                    //    taking borrow_mut — avoids aliasing with get_model()
                    //    inside RenderState (SimpleEntity uses unsafe as_ptr).
                    let mesh_count = model.borrow().get_mesh_count();
                    // Resolve all per-mesh data from shared borrows BEFORE
                    // taking borrow_mut to avoid aliased borrow UB.
                    let materials: Vec<Option<Material>> = (0..mesh_count)
                        .map(|i| model.borrow().get_material(i).cloned())
                        .collect();
                    let vert_colors: Vec<bool> = (0..mesh_count)
                        .map(|i| model.borrow().has_vertex_colors(i))
                        .collect();

                    let mut model_ref = model.borrow_mut();

                    for i in 0..mesh_count {
                        let render_state = RenderState::new_preresolved(
                            self,
                            entity,
                            &camera_ref,
                            i,
                            materials[i].clone(),
                            vert_colors[i],
                        );
                        self.shader_program.update_per_instance_uniforms(&render_state);
                        model_ref.bind_and_configure(i);
                        model_ref.render(&render_state, i);
                        model_ref.unbind(i);
                    }

                    self.shader_program.unbind();
                }
            }

            unsafe { gl::Enable(gl::CULL_FACE); }
        }
    }

    fn any_processed(&self) -> bool { true }
    fn finish(&mut self) {}
}

impl<T: SceneObject> Drop for EntityRenderer<T> {
    fn drop(&mut self) {
        log::info!("EntityRenderer dropped");
    }
}
