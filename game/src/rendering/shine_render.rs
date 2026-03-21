//! Solve shine renderer — Overlay pass.
//!
//! Reads solved state and G-buffer textures from SceneContext at render time.
//! Accepts shader source at construction; `new()` uses the game's default shaders.

use formosaic_engine::{
    architecture::scene::scene_context::SceneContext,
    rendering::abstracted::{
        irenderer::{IRenderer, RenderPass},
        processable::NoopProcessable,
    },
    opengl::{
        constants::data_type::DataType,
        objects::{attribute::Attribute, data_buffer::DataBuffer, ivbo::IVbo, vao::Vao},
        shaders::{
            uniform::{UniformAdapter, UniformFloat, UniformInt},
            RenderState, ShaderProgram,
        },
    },
};
use std::{cell::RefCell, rc::Rc};

const DEFAULT_VERT: &str = include_str!("../../assets/shaders/shine.vert.glsl");
const DEFAULT_FRAG: &str = include_str!("../../assets/shaders/shine.frag.glsl");

struct FrameState {
    solved_timer:    f32,  // -1.0 = not solved
    albedo_unit:     i32,  // GL texture unit index for albedo
    position_unit:   i32,  // GL texture unit index for position
}

pub struct ShineRenderer {
    vao:    Vao,
    shader: ShaderProgram<NoopProcessable>,
    frame:  Rc<RefCell<FrameState>>,
}

impl ShineRenderer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_shaders(DEFAULT_VERT, DEFAULT_FRAG)
    }

    pub fn with_shaders(vert: &str, frag: &str) -> Result<Self, Box<dyn std::error::Error>> {
        #[rustfmt::skip]
        let verts: [f32; 12] = [
            -1.0, -1.0,  1.0, -1.0,  1.0,  1.0,
            -1.0, -1.0,  1.0,  1.0, -1.0,  1.0,
        ];
        let buf = DataBuffer::load_static(&verts);
        let mut vao = Vao::create();
        vao.load_data_buffer(
            Rc::new(buf) as Rc<dyn IVbo>,
            &[Attribute::of(0, 2, DataType::Float, false)],
        );

        // Position G-buffer on unit 0, albedo on unit 1 — fixed, set once.
        let frame = Rc::new(RefCell::new(FrameState {
            solved_timer:  -1.0,
            albedo_unit:   1,
            position_unit: 0,
        }));

        let mut shader = ShaderProgram::<NoopProcessable>::from_sources(vert, frag)?;

        // uTime — drives the sweep animation, updates every frame.
        {
            let f = Rc::clone(&frame);
            shader.add_per_render_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform: UniformFloat::new("uTime"),
                extractor: Box::new(move |_: &RenderState<NoopProcessable>| {
                    f.borrow().solved_timer
                }),
            })));
        }
        // Sampler uniforms — constant unit indices, but registered through the
        // same uniform system for consistency.
        {
            let f = Rc::clone(&frame);
            shader.add_per_render_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform: UniformInt::new("uPosition"),
                extractor: Box::new(move |_: &RenderState<NoopProcessable>| {
                    f.borrow().position_unit
                }),
            })));
        }
        {
            let f = Rc::clone(&frame);
            shader.add_per_render_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform: UniformInt::new("uAlbedo"),
                extractor: Box::new(move |_: &RenderState<NoopProcessable>| {
                    f.borrow().albedo_unit
                }),
            })));
        }

        log::info!("ShineRenderer initialised");
        Ok(Self { vao, shader, frame })
    }
}

impl IRenderer for ShineRenderer {
    fn pass(&self) -> RenderPass { RenderPass::Overlay }

    fn render(&mut self, context: &SceneContext) {
        let timer = match context.solved_timer { Some(t) => t, None => return };

        let (albedo_tex, pos_tex) = match context.output_data() {
            Some(d) => (d.colour.get_id(), d.position.get_id()),
            None    => return,
        };
        if pos_tex == 0 { return; }

        self.frame.borrow_mut().solved_timer = timer;

        unsafe {
            gl::Disable(gl::DEPTH_TEST);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

            // Bind G-buffer textures to the units the uniforms declare.
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, pos_tex);
            gl::ActiveTexture(gl::TEXTURE1);
            gl::BindTexture(gl::TEXTURE_2D, albedo_tex);
        }

        self.shader.bind();
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
            gl::ActiveTexture(gl::TEXTURE1);
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }

    fn any_processed(&self) -> bool { self.frame.borrow().solved_timer >= 0.0 }
    fn finish(&mut self) {}
}
