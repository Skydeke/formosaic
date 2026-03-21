//! Solve shine renderer — Overlay pass.
//!
//! Renders a fullscreen quad that samples the world-space position G-buffer
//! and draws a diagonal sweep highlight across the model surface.
//! Runs after the deferred lighting blit so it composites over the lit scene.

use crate::engine::architecture::scene::scene_context::SceneContext;
use crate::engine::rendering::abstracted::irenderer::{IRenderer, RenderPass};
use crate::engine::rendering::abstracted::processable::NoopProcessable;
use crate::engine::rendering::pipeline::FrameData;
use crate::opengl::{
    constants::data_type::DataType,
    objects::{attribute::Attribute, data_buffer::DataBuffer, ivbo::IVbo, vao::Vao},
    shaders::{
        uniform::{UniformAdapter, UniformFloat},
        RenderState, ShaderProgram,
    },
};
use std::cell::RefCell;
use std::rc::Rc;

struct FrameState {
    solved_timer: f32,   // -1.0 = not solved
    position_tex_id: u32,
}

pub struct ShineRenderer {
    vao:    Vao,
    shader: ShaderProgram<NoopProcessable>,
    frame:  Rc<RefCell<FrameState>>,
    loc_position: i32,
}

impl ShineRenderer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        #[rustfmt::skip]
        let verts: [f32; 12] = [
            -1.0, -1.0,
             1.0, -1.0,
             1.0,  1.0,
            -1.0, -1.0,
             1.0,  1.0,
            -1.0,  1.0,
        ];

        let buf     = DataBuffer::load_static(&verts);
        let mut vao = Vao::create();
        vao.load_data_buffer(
            Rc::new(buf) as Rc<dyn IVbo>,
            &[Attribute::of(0, 2, DataType::Float, false)],
        );

        let vert_src = include_str!("../../../../assets/shaders/shine.vert.glsl");
        let frag_src = include_str!("../../../../assets/shaders/shine.frag.glsl");
        let mut shader = ShaderProgram::<NoopProcessable>::from_sources(vert_src, frag_src)?;

        // Bind sampler unit 0 once at construction — position texture always on unit 0.
        let loc_position = shader.get_uniform_location("uPosition");

        let frame = Rc::new(RefCell::new(FrameState {
            solved_timer:    -1.0,
            position_tex_id: 0,
        }));

        {
            let f = Rc::clone(&frame);
            shader.add_per_render_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform: UniformFloat::new("uTime"),
                extractor: Box::new(move |_: &RenderState<NoopProcessable>| {
                    f.borrow().solved_timer
                }),
            })));
        }

        log::info!("ShineRenderer initialised");
        Ok(Self { vao, shader, frame, loc_position })
    }
}

impl IRenderer for ShineRenderer {
    fn pass(&self) -> RenderPass { RenderPass::Overlay }

    fn prepare(&mut self, data: &FrameData) {
        let mut f = self.frame.borrow_mut();
        f.solved_timer    = if data.solved { data.time } else { -1.0 };
        f.position_tex_id = data.position_tex_id;
    }

    fn render(&mut self, _context: &SceneContext) {
        let (timer, pos_tex) = {
            let f = self.frame.borrow();
            (f.solved_timer, f.position_tex_id)
        };

        if timer < 0.0 || pos_tex == 0 { return; }

        unsafe {
            gl::Disable(gl::DEPTH_TEST);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

            // Bind position texture to unit 0
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, pos_tex);
        }

        self.shader.bind();

        // Set sampler uniform directly — it's a constant (unit 0) so no adapter needed
        unsafe { gl::Uniform1i(self.loc_position, 0); }

        let state = RenderState::new_screenspace(self);
        self.shader.update_per_render_uniforms(&state);

        self.vao.bind();
        self.vao.enable_attributes();
        unsafe { gl::DrawArrays(gl::TRIANGLES, 0, 6); }
        self.vao.unbind();

        self.shader.unbind();

        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::Disable(gl::BLEND);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }

    fn any_processed(&self) -> bool {
        self.frame.borrow().solved_timer >= 0.0
    }

    fn finish(&mut self) {}
}
