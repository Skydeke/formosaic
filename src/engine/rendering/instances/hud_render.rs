//! Hint overlay renderer (Tier 1+).
//!
//! Renders a fullscreen-quad warmth compass.  All uniforms go through the
//! ShaderProgram<T> adapter system — no raw gl::Uniform* calls, no manually
//! cached location integers.

use crate::engine::architecture::scene::scene_context::SceneContext;
use crate::engine::rendering::abstracted::irenderer::IRenderer;
use crate::engine::rendering::abstracted::processable::NoopProcessable;
use crate::opengl::{
    constants::data_type::DataType,
    objects::{attribute::Attribute, data_buffer::DataBuffer, ivbo::IVbo, vao::Vao},
    shaders::{
        uniform::{UniformAdapter, UniformFloat, UniformVec3},
        RenderState, ShaderProgram,
    },
};
use cgmath::Vector3;
use std::cell::RefCell;
use std::rc::Rc;

/// Shared frame state read by the per-render uniform extractors.
struct FrameState {
    warmth_color: Vector3<f32>,
    warmth:       f32,
    hint_tier:    f32,
    time:         f32,
}

pub struct HudRenderer {
    vao:    Vao,
    shader: ShaderProgram<NoopProcessable>,
    frame:  Rc<RefCell<FrameState>>,
    pub warmth_color: [f32; 3],
    pub warmth:       f32,
    pub hint_tier:    f32,
    pub time:         f32,
    pub active:       bool,
}

impl HudRenderer {
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
        let attr    = Attribute::of(0, 2, DataType::Float, false);
        vao.load_data_buffer(Rc::new(buf) as Rc<dyn IVbo>, &[attr]);

        let vert_src = include_str!("../../../../assets/shaders/hud.vert.glsl");
        let frag_src = include_str!("../../../../assets/shaders/hud.frag.glsl");
        let mut shader = ShaderProgram::<NoopProcessable>::from_sources(vert_src, frag_src)?;

        let frame = Rc::new(RefCell::new(FrameState {
            warmth_color: Vector3::new(1.0, 1.0, 1.0),
            warmth:    0.0,
            hint_tier: 0.0,
            time:      0.0,
        }));

        {
            let f = Rc::clone(&frame);
            shader.add_per_render_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform: UniformVec3::new("uWarmthColor"),
                extractor: Box::new(move |_: &RenderState<NoopProcessable>| {
                    f.borrow().warmth_color
                }),
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

        Ok(Self {
            vao,
            shader,
            frame,
            warmth_color: [1.0, 1.0, 1.0],
            warmth:    0.0,
            hint_tier: 0.0,
            time:      0.0,
            active:    false,
        })
    }
}

impl IRenderer for HudRenderer {
    fn render(&mut self, _context: &SceneContext) {
        if !self.active || self.hint_tier < 0.5 {
            return;
        }

        // Sync public fields into the shared frame state the extractors read.
        {
            let mut f = self.frame.borrow_mut();
            f.warmth_color = Vector3::new(
                self.warmth_color[0],
                self.warmth_color[1],
                self.warmth_color[2],
            );
            f.warmth    = self.warmth;
            f.hint_tier = self.hint_tier;
            f.time      = self.time;
        }

        unsafe {
            gl::Disable(gl::DEPTH_TEST);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }

        self.shader.bind();
        // Screenspace pass: no camera, no scene instance.
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
        }
    }

    fn any_processed(&self) -> bool { self.active }
    fn finish(&mut self) {}
}
