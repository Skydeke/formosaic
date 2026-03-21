//! Outline (solved glow) renderer.
//!
//! Classic back-face extrusion technique: renders only back-faces with normals
//! pushed outward by `uOutlineWidth`, producing a coloured shell around each
//! model without touching the model's own geometry pass.
//!
//! Shader inputs (must match VAO layout set by `bind_and_configure`):
//!   location 0 — vec3 pos   (always present)
//!   location 1 — vec2 uv    (disabled per draw — not used by outline shader)
//!   location 2 — vec3 norm  (always present)
//!
//! All uniforms go through the ShaderProgram<T> adapter system.

use crate::engine::architecture::models::model::Model;
use crate::engine::architecture::scene::entity::scene_object::SceneObject;
use crate::engine::architecture::scene::entity::simple_entity::SimpleEntity;
use crate::engine::architecture::scene::node::node::NodeBehavior;
use crate::engine::architecture::scene::scene_context::SceneContext;
use crate::engine::rendering::abstracted::irenderer::{IRenderer, RenderPass};
use crate::engine::rendering::pipeline::FrameData;
use crate::opengl::objects::attribute::Attribute;
use crate::opengl::shaders::{
    uniform::{UniformAdapter, UniformFloat, UniformVec3},
    RenderState, ShaderProgram, UniformMatrix4,
};
use cgmath::Vector3;
use std::cell::RefCell;
use std::rc::Rc;

struct FrameState {
    outline_width: f32,
    time:          f32,
    glow_color:    Vector3<f32>,
    alpha:         f32,
}

pub struct OutlineRenderer {
    shader:        ShaderProgram<SimpleEntity>,
    frame:         Rc<RefCell<FrameState>>,
    outline_width: f32,
    time:          f32,
    pub active:    bool,
    pub intensity: f32,
}

impl OutlineRenderer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let vert_src = include_str!("../../../../assets/shaders/outline.vert.glsl");
        let frag_src = include_str!("../../../../assets/shaders/outline.frag.glsl");
        let mut shader = ShaderProgram::<SimpleEntity>::from_sources(vert_src, frag_src)?;

        // Per-instance: VP and model matrix vary per entity.
        shader.add_per_instance_uniform(Rc::new(RefCell::new(UniformAdapter {
            uniform: UniformMatrix4::new("uVP"),
            extractor: Box::new(|s: &RenderState<SimpleEntity>| {
                *s.camera().get_projection_view_matrix()
            }),
        })));
        shader.add_per_instance_uniform(Rc::new(RefCell::new(UniformAdapter {
            uniform: UniformMatrix4::new("uModel"),
            extractor: Box::new(|s: &RenderState<SimpleEntity>| {
                s.instance().unwrap().transform().get_matrix()
            }),
        })));

        // Per-render: constant across all entities for the draw call.
        let frame = Rc::new(RefCell::new(FrameState {
            outline_width: 0.02,
            time:          0.0,
            glow_color:    Vector3::new(0.35, 1.0, 0.55),
            alpha:         0.0,
        }));

        {
            let f = Rc::clone(&frame);
            shader.add_per_render_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform: UniformFloat::new("uOutlineWidth"),
                extractor: Box::new(move |_: &RenderState<SimpleEntity>| {
                    f.borrow().outline_width
                }),
            })));
        }
        {
            let f = Rc::clone(&frame);
            shader.add_per_render_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform: UniformFloat::new("uTime"),
                extractor: Box::new(move |_: &RenderState<SimpleEntity>| f.borrow().time),
            })));
        }
        {
            let f = Rc::clone(&frame);
            shader.add_per_render_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform: UniformVec3::new("uGlowColor"),
                extractor: Box::new(move |_: &RenderState<SimpleEntity>| f.borrow().glow_color),
            })));
        }
        {
            let f = Rc::clone(&frame);
            shader.add_per_render_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform: UniformFloat::new("uAlpha"),
                extractor: Box::new(move |_: &RenderState<SimpleEntity>| f.borrow().alpha),
            })));
        }

        log::info!("OutlineRenderer initialised");
        Ok(Self { shader, frame, outline_width: 0.02, time: 0.0, active: false, intensity: 0.0 })
    }

    pub fn set_active(&mut self, active: bool)  { self.active    = active; }
    pub fn set_intensity(&mut self, v: f32)     { self.intensity = v.clamp(0.0, 1.0); }
    /// Advance the shader time accumulator. `elapsed` is total game seconds.
    pub fn tick(&mut self, elapsed: f32)        { self.time = elapsed; }
}

impl IRenderer for OutlineRenderer {
    fn pass(&self) -> RenderPass { RenderPass::Overlay }

    fn prepare(&mut self, data: &FrameData) {
        self.set_active(data.solved || data.glow_intensity > 0.01);
        self.set_intensity(data.glow_intensity);
        self.tick(data.time);
    }

    fn render(&mut self, context: &SceneContext) {
        if !self.active || self.intensity < 0.01 { return; }

        let camera = context.get_camera();
        let scene  = match context.scene() { Some(s) => s, None => return };
        let nodes  = scene.collect_nodes_of_type::<SimpleEntity>();
        if nodes.is_empty() { return; }

        // Sync per-render frame state.
        {
            let mut f   = self.frame.borrow_mut();
            f.outline_width = self.outline_width;
            f.time          = self.time;
            f.glow_color    = Vector3::new(0.35, 1.0, 0.55);
            f.alpha         = self.intensity * 0.8;
        }

        // Back-face extrusion technique: cull front faces so only the
        // expanded back shell is visible, giving a clean outline silhouette.
        unsafe {
            gl::Enable(gl::CULL_FACE);
            gl::CullFace(gl::FRONT);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }

        self.shader.bind();

        // Push per-render uniforms once before the entity loop.
        let camera_ref   = camera.borrow();
        let render_state = RenderState::new_without_instance(self, &camera_ref);
        self.shader.update_per_render_uniforms(&render_state);
        drop(camera_ref);

        for node in &nodes {
            let node_ref = node.borrow();
            if let Some(entity) = node_ref.as_any().downcast_ref::<SimpleEntity>() {
                let camera_ref    = camera.borrow();
                let model         = entity.model();
                let mesh_count    = model.borrow().get_meshes().len();
                let mut model_ref = model.borrow_mut();

                for i in 0..mesh_count {
                    let render_state = RenderState::new(self, entity, &camera_ref, i);
                    self.shader.update_per_instance_uniforms(&render_state);
                    model_ref.bind_and_configure(i);
                    // Disable UV attrib (loc 1) — not declared in outline shader.
                    // Must be done after bind_and_configure re-enables it.
                    Attribute::disable_index(1);
                    model_ref.render(&render_state, i);
                    Attribute::enable_index(1);
                    model_ref.unbind(i);
                }
            }
        }

        self.shader.unbind();

        // Restore raster state.
        unsafe {
            gl::CullFace(gl::BACK);
            gl::Disable(gl::BLEND);
        }
    }

    fn any_processed(&self) -> bool { self.active }
    fn finish(&mut self) {}
}
