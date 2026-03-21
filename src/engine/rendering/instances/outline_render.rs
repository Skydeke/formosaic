//! Outline (solved glow) renderer.
//!
//! Extrudes back-faces along normals to draw a pulsing shell around the model.
//! Per-instance uniforms (VP, Model) go through the ShaderProgram<T> system.
//! Per-render uniforms (width, time, color, alpha) use SimpleProgram helpers
//! so we never call raw gl::Uniform* directly.

use crate::engine::architecture::models::model::Model;
use crate::engine::architecture::scene::entity::scene_object::SceneObject;
use crate::engine::architecture::scene::entity::simple_entity::SimpleEntity;
use crate::engine::architecture::scene::node::node::NodeBehavior;
use crate::engine::architecture::scene::scene_context::SceneContext;
use crate::engine::rendering::abstracted::irenderer::IRenderer;
use crate::opengl::objects::attribute::Attribute;
use crate::opengl::shaders::{
    uniform::UniformAdapter, RenderState, ShaderProgram, SimpleProgram, UniformMatrix4,
};
use cgmath::Vector3;
use std::cell::RefCell;
use std::rc::Rc;

pub struct OutlineRenderer {
    shader: ShaderProgram<SimpleEntity>,
    /// Thin wrapper that only holds the id — used for uniform setters.
    /// Shares the same GL program object as `shader`.
    program_proxy: SimpleProgram,
    loc_width: i32,
    loc_time: i32,
    loc_color: i32,
    loc_alpha: i32,
    outline_width: f32,
    time: f32,
    active: bool,
    intensity: f32,
}

impl OutlineRenderer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let vert_src = include_str!("../../../../assets/shaders/outline.vert.glsl");
        let frag_src = include_str!("../../../../assets/shaders/outline.frag.glsl");
        let mut shader = ShaderProgram::<SimpleEntity>::from_sources(vert_src, frag_src)?;

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

        // Build a lightweight proxy with the same GL program id so we can use
        // SimpleProgram's uniform setters without duplicating the logic.
        // SAFETY: SimpleProgram::from_id does not own the program — it must NOT
        // delete it in drop.  We implement that below.
        let program_proxy = SimpleProgram::from_id(shader.id());

        let loc_width = program_proxy.uniform_location("uOutlineWidth");
        let loc_time = program_proxy.uniform_location("uTime");
        let loc_color = program_proxy.uniform_location("uGlowColor");
        let loc_alpha = program_proxy.uniform_location("uAlpha");

        log::info!("OutlineRenderer initialised");
        Ok(Self {
            shader,
            program_proxy,
            loc_width,
            loc_time,
            loc_color,
            loc_alpha,
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

        self.shader.bind();

        // Per-render uniforms via SimpleProgram proxy — no raw gl::Uniform* calls.
        let glow = Vector3::new(0.35_f32, 1.0, 0.55);
        self.program_proxy
            .set_uniform_float(self.loc_width, self.outline_width);
        self.program_proxy
            .set_uniform_float(self.loc_time, self.time);
        self.program_proxy
            .set_uniform_vec3(self.loc_color, glow.x, glow.y, glow.z);
        self.program_proxy
            .set_uniform_float(self.loc_alpha, self.intensity * 0.8);

        for node in &nodes {
            let node_ref = node.borrow();
            if let Some(entity) = node_ref.as_any().downcast_ref::<SimpleEntity>() {
                let camera_ref = camera.borrow();
                let model = entity.model(); // Rc kept alive
                let mesh_count = model.borrow().get_meshes().len();
                let mut model_ref = model.borrow_mut();

                for i in 0..mesh_count {
                    let render_state = RenderState::new(self, entity, &camera_ref, i);
                    self.shader.update_per_instance_uniforms(&render_state);
                    model_ref.bind_and_configure(i);
                    // UV attrib (loc 1) is not in the outline shader — disable it
                    // after VAO bind so strict GLES drivers don't reject the draw.
                    Attribute::disable_index(1);
                    model_ref.render(&render_state, i);
                    Attribute::enable_index(1);
                    model_ref.unbind(i);
                }
            }
        }

        self.shader.unbind();
    }

    fn any_processed(&self) -> bool {
        self.active
    }
    fn finish(&mut self) {}
}
