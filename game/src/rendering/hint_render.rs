//! Hint overlay renderer (Tier 1+).
//!
//! Uses ShaderProgram<NoopProcessable> + UniformAdapter — same pattern as EntityRenderer.

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
            uniform::{UniformAdapter, UniformFloat, UniformVec3},
            RenderState, ShaderProgram,
        },
    },
};
use cgmath::Vector3;
use std::{cell::RefCell, rc::Rc};

const DEFAULT_VERT: &str = include_str!("../../assets/shaders/hint.vert.glsl");
const DEFAULT_FRAG: &str = include_str!("../../assets/shaders/hint.frag.glsl");

struct FrameState {
    warmth_color: Vector3<f32>,
    warmth:       f32,
    hint_tier:    f32,
    time:         f32,
}

pub struct HintRenderer {
    vao:    Vao,
    shader: ShaderProgram<NoopProcessable>,
    frame:  Rc<RefCell<FrameState>>,
}

impl HintRenderer {
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

        let frame = Rc::new(RefCell::new(FrameState {
            warmth_color: Vector3::new(1.0, 1.0, 1.0),
            warmth: 0.0, hint_tier: 0.0, time: 0.0,
        }));

        let mut shader = ShaderProgram::<NoopProcessable>::from_sources(vert, frag)?;

        {
            let f = Rc::clone(&frame);
            shader.add_per_render_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform: UniformVec3::new("uWarmthColor"),
                extractor: Box::new(move |_: &RenderState<NoopProcessable>| f.borrow().warmth_color),
            })));
        }
        {
            let f = Rc::clone(&frame);
            shader.add_per_render_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform: UniformFloat::new("uWarmth"),
                extractor: Box::new(move |_: &RenderState<NoopProcessable>| f.borrow().warmth),
            })));
        }
        {
            let f = Rc::clone(&frame);
            shader.add_per_render_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform: UniformFloat::new("uHintTier"),
                extractor: Box::new(move |_: &RenderState<NoopProcessable>| f.borrow().hint_tier),
            })));
        }
        {
            let f = Rc::clone(&frame);
            shader.add_per_render_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform: UniformFloat::new("uTime"),
                extractor: Box::new(move |_: &RenderState<NoopProcessable>| f.borrow().time),
            })));
        }

        Ok(Self { vao, shader, frame })
    }
}

impl IRenderer for HintRenderer {
    fn pass(&self) -> RenderPass { RenderPass::Overlay }

    fn render(&mut self, context: &SceneContext) {
        let hints = match context.hints {
            Some(h) if h.tier >= 1 => h,
            _ => return,
        };

        {
            let mut f = self.frame.borrow_mut();
            f.warmth_color = Vector3::new(
                hints.warmth_color[0], hints.warmth_color[1], hints.warmth_color[2],
            );
            f.warmth    = hints.warmth;
            f.hint_tier = hints.tier as f32;
            f.time      = hints.time;
        }

        unsafe {
            gl::Disable(gl::DEPTH_TEST);
            gl::Disable(gl::CULL_FACE);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
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
            gl::Enable(gl::CULL_FACE);
            gl::Disable(gl::BLEND);
        }
    }

    fn any_processed(&self) -> bool { true }
    fn finish(&mut self) {}
}
