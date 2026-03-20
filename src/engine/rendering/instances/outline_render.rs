use crate::engine::architecture::scene::entity::simple_entity::SimpleEntity;
use crate::engine::architecture::scene::entity::scene_object::SceneObject;
use crate::engine::architecture::scene::node::node::NodeBehavior;
use crate::engine::architecture::scene::scene_context::SceneContext;
use crate::engine::architecture::models::model::Model;
use crate::engine::rendering::abstracted::irenderer::IRenderer;
use crate::opengl::shaders::{
    uniform::UniformAdapter,
    RenderState, ShaderProgram, UniformMatrix4,
};
use cgmath::Vector3;
use std::cell::RefCell;
use std::rc::Rc;

/// Renders a thin glowing shell around the model to indicate the solved state.
///
/// Uses the outline vertex shader to extrude along normals and the outline
/// fragment shader to draw a pulsing colour.  Back-face culling is DISABLED
/// so only the extruded back-faces are visible (standard outline technique).
pub struct OutlineRenderer {
    shader: ShaderProgram<SimpleEntity>,
    /// World-space extrusion amount (scales with model size).
    outline_width: f32,
    /// Accumulated time for pulsing.
    time: f32,
    /// Whether to actually draw (set true when puzzle is solved/near-solved).
    active: bool,
    /// Glow intensity [0,1] — animated externally.
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

        // uOutlineWidth and uTime are per-render uniforms set via raw GL calls below.
        // uGlowColor + uAlpha same.
        log::info!("OutlineRenderer initialised");
        Ok(Self {
            shader,
            outline_width: 0.02,
            time: 0.0,
            active: false,
            intensity: 0.0,
        })
    }

    pub fn set_active(&mut self, active: bool) { self.active = active; }
    pub fn set_intensity(&mut self, v: f32)    { self.intensity = v.clamp(0.0, 1.0); }

    pub fn update(&mut self, delta_time: f32) {
        self.time += delta_time;
    }
}

impl IRenderer for OutlineRenderer {
    fn render(&mut self, context: &SceneContext) {
        if !self.active || self.intensity < 0.01 { return; }

        let camera   = context.get_camera();
        let scene    = match context.scene() { Some(s) => s, None => return };
        let nodes    = scene.collect_nodes_of_type::<SimpleEntity>();
        if nodes.is_empty() { return; }

        // Outline technique: extrude back-faces so only the shell is visible.
        // We save/restore cull state so the pipeline is left clean.
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthMask(gl::FALSE);          // don't write depth for the shell
            gl::Enable(gl::CULL_FACE);
            gl::CullFace(gl::FRONT);           // draw only back-faces (the extruded shell)
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }

        self.shader.bind();

        // Push per-render uniforms via raw GL (these aren't wired through UniformAdapter).
        let outline_loc = self.shader.get_uniform_location("uOutlineWidth");
        let time_loc    = self.shader.get_uniform_location("uTime");
        let color_loc   = self.shader.get_uniform_location("uGlowColor");
        let alpha_loc   = self.shader.get_uniform_location("uAlpha");

        let glow = Vector3::new(0.35_f32, 1.0_f32, 0.55_f32); // soft green
        unsafe {
            if outline_loc >= 0 {
                gl::Uniform1f(outline_loc, self.outline_width);
            }
            if time_loc >= 0 {
                gl::Uniform1f(time_loc, self.time);
            }
            if color_loc >= 0 {
                gl::Uniform3f(color_loc, glow.x, glow.y, glow.z);
            }
            if alpha_loc >= 0 {
                gl::Uniform1f(alpha_loc, self.intensity * 0.8);
            }
        }

        for node in &nodes {
            let node_ref = node.borrow();
            if let Some(entity) = node_ref.as_any().downcast_ref::<SimpleEntity>() {
                let camera_ref = camera.borrow();
                let mut render_state = RenderState::new(self, entity, &camera_ref, 0);
                self.shader.update_per_instance_uniforms(&render_state);

                let model     = entity.model();
                let mesh_count = model.borrow().get_meshes().len();
                let mut model_ref = model.borrow_mut();

                for i in 0..mesh_count {
                    render_state = RenderState::new(self, entity, &camera_ref, i);
                    self.shader.update_per_instance_uniforms(&render_state);
                    model_ref.bind_and_configure(i);
                    model_ref.render(&render_state, i);
                    model_ref.unbind(i);
                }
            }
        }

        self.shader.unbind();

        unsafe {
            gl::CullFace(gl::BACK);   // restore normal culling
            gl::DepthMask(gl::TRUE);
            gl::Disable(gl::BLEND);
        }
    }

    fn any_processed(&self) -> bool { self.active }
    fn finish(&mut self) {}
}
