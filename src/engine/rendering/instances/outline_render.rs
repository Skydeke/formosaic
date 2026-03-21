//! Outline (solved glow) renderer.
//!
//! Extrudes back-faces along normals to draw a pulsing shell around each model.
//!
//! All uniforms — both per-render constants (width, time, color, alpha) and
//! per-instance transforms (VP, Model) — go through the ShaderProgram<T>
//! uniform adapter system.  There are no raw gl::Uniform* calls and no
//! SimpleProgram proxy objects here.

use crate::engine::architecture::models::model::Model;
use crate::engine::architecture::scene::entity::scene_object::SceneObject;
use crate::engine::architecture::scene::entity::simple_entity::SimpleEntity;
use crate::engine::architecture::scene::node::node::NodeBehavior;
use crate::engine::architecture::scene::scene_context::SceneContext;
use crate::engine::rendering::abstracted::irenderer::IRenderer;
use crate::opengl::objects::attribute::Attribute;
use crate::opengl::shaders::{
    uniform::{UniformAdapter, UniformFloat, UniformVec3},
    RenderState, ShaderProgram, UniformMatrix4,
};
use cgmath::Vector3;
use std::cell::RefCell;
use std::rc::Rc;

/// Shared frame state read by the per-render uniform extractors.
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

        // ── Per-instance: camera VP and model matrix ──────────────────────
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

        // ── Per-render: width / time / color / alpha ──────────────────────
        // Constant across all instances in one draw call.  A shared FrameState
        // cell lets the renderer update them each frame without touching any
        // uniform locations directly.
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
        Ok(Self {
            shader,
            frame,
            outline_width: 0.02,
            time: 0.0,
            active: false,
            intensity: 0.0,
        })
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }
    pub fn set_intensity(&mut self, v: f32) {
        self.intensity = v.clamp(0.0, 1.0);
    }
    pub fn update(&mut self, delta_time: f32) {
        self.time += delta_time;
    }
}

impl IRenderer for OutlineRenderer {
    fn render(&mut self, context: &SceneContext) {
        if !self.active || self.intensity < 0.01 {
            return;
        }

        let camera = context.get_camera();
        let scene = match context.scene() {
            Some(s) => s,
            None => return,
        };
        let nodes = scene.collect_nodes_of_type::<SimpleEntity>();
        if nodes.is_empty() {
            return;
        }

        // Sync frame state so per-render extractors see current values.
        {
            let mut f    = self.frame.borrow_mut();
            f.outline_width = self.outline_width;
            f.time          = self.time;
            f.glow_color    = Vector3::new(0.35, 1.0, 0.55);
            f.alpha         = self.intensity * 0.8;
        }

        self.shader.bind();

        // Push per-render uniforms once for the whole draw call.
        // new_without_instance is fine — per-render extractors ignore the instance.
        let camera_ref   = camera.borrow();
        let render_state = RenderState::new_without_instance(self, &camera_ref);
        self.shader.update_per_render_uniforms(&render_state);
        drop(camera_ref);

        for node in &nodes {
            let node_ref = node.borrow();
            if let Some(entity) = node_ref.as_any().downcast_ref::<SimpleEntity>() {
                let camera_ref = camera.borrow();
                let model      = entity.model();
                let mesh_count = model.borrow().get_meshes().len();
                let mut model_ref = model.borrow_mut();

                for i in 0..mesh_count {
                    let render_state = RenderState::new(self, entity, &camera_ref, i);
                    self.shader.update_per_instance_uniforms(&render_state);
                    model_ref.bind_and_configure(i);
                    // UV attrib (loc 1) is absent from the outline shader —
                    // disable it after VAO bind so strict GLES drivers don't
                    // reject the draw.
                    Attribute::disable_index(1);
                    model_ref.render(&render_state, i);
                    Attribute::enable_index(1);
                    model_ref.unbind(i);
                }
            }
        }

        self.shader.unbind();
    }

    fn any_processed(&self) -> bool { self.active }
    fn finish(&mut self) {}
}
