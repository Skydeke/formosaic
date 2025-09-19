use cgmath::Vector3;

use crate::engine::architecture::models::model::Model;
use crate::engine::architecture::scene::entity::scene_object::SceneObject;
use crate::engine::architecture::scene::scene_context::SceneContext;
use crate::engine::rendering::abstracted::irenderer::IRenderer;
use crate::opengl::shaders::uniform::{UniformBoolean, UniformTexture};
use crate::opengl::shaders::UniformVec3;
use crate::opengl::shaders::{uniform::UniformAdapter, RenderState, ShaderProgram, UniformMatrix4};
use std::cell::RefCell;
use std::rc::Rc;

pub struct EntityRenderer<T: SceneObject> {
    shader_program: ShaderProgram<T>,
}

impl<T: SceneObject + 'static> EntityRenderer<T> {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        log::info!("Initializing EntityRenderer");
        let vertex_src = include_str!("../../../../assets/shaders/basic.vert.glsl");
        let fragment_src = include_str!("../../../../assets/shaders/basic.frag.glsl");
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
                state
                    .mesh()
                    .and_then(|m| m.material())
                    .and_then(|mat| mat.diffuse_texture.clone())
            }),
        })));

        shader_program.add_per_instance_uniform(Rc::new(RefCell::new(UniformAdapter {
            uniform: UniformVec3::new("albedoConst"),
            extractor: Box::new(|state: &RenderState<T>| {
                state
                    .mesh()
                    .and_then(|m| m.material())
                    .map(|mat| mat.diffuse_color.truncate())
                    .unwrap_or(Vector3::new(1.0, 1.0, 1.0)) // fallback white
            }),
        })));

        shader_program.add_per_instance_uniform(Rc::new(RefCell::new(UniformAdapter {
            uniform: UniformBoolean::new("isAlbedoMapped"),
            extractor: Box::new(|state: &RenderState<T>| {
                state
                    .mesh()
                    .and_then(|m| m.material())
                    .and_then(|mat| mat.diffuse_texture.as_ref())
                    .is_some()
            }),
        })));

        log::info!("EntityRenderer initialized successfully");
        Ok(Self { shader_program })
    }
}

impl<T: SceneObject + 'static> IRenderer for EntityRenderer<T> {
    fn render(&mut self, context: &SceneContext) {
        let camera = context.get_camera();

        // Get entities from the scenegraph
        if let Some(scene) = context.scene() {
            let entity_nodes = scene.collect_nodes_of_type::<T>();

            for node in &entity_nodes {
                // Downcast to the specific entity type
                let node_ref = node.borrow();
                if let Some(entity) = node_ref.as_any().downcast_ref::<T>() {
                    // Extract camera borrow to live longer
                    let camera_ref = camera.borrow();

                    // Bind shader before creating RenderState
                    self.shader_program.bind();

                    // Create RenderState with the longer-lived camera reference
                    let mut render_state = RenderState::new(self, entity, &camera_ref, 0);
                    self.shader_program
                        .update_per_instance_uniforms(&render_state);

                    // Extract model to live longer
                    let model = entity.model();
                    let mesh_count = model.borrow().get_meshes().len();
                    let mut model_ref = model.borrow_mut();

                    for i in 0..mesh_count {
                        render_state = RenderState::new(self, entity, &camera_ref, i);
                        self.shader_program
                            .update_per_instance_uniforms(&render_state);
                        model_ref.bind_and_configure(i);
                        model_ref.render(&render_state, i);
                        model_ref.unbind(i);
                    }

                    self.shader_program.unbind();
                }
            }
        }
    }

    fn any_processed(&self) -> bool {
        // Always return true for now, or implement logic to check if there are entities to render
        true
    }

    fn finish(&mut self) {
        // Nothing to clear since we're not maintaining a render list
    }
}

impl<T: SceneObject> Drop for EntityRenderer<T> {
    fn drop(&mut self) {
        log::info!("EntityRenderer dropped");
    }
}
