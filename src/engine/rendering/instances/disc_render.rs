//! Axis-plane disc renderer (Hint Tier 2).
//!
//! Renders a translucent disc perpendicular to the solution axis.  All uniforms
//! go through the ShaderProgram<T> adapter system — no raw gl::Uniform* calls,
//! no manually cached location integers.

use crate::engine::architecture::scene::scene_context::SceneContext;
use crate::engine::rendering::abstracted::irenderer::IRenderer;
use crate::engine::rendering::abstracted::processable::NoopProcessable;
use crate::opengl::{
    constants::data_type::DataType,
    objects::{attribute::Attribute, data_buffer::DataBuffer, ivbo::IVbo, vao::Vao},
    shaders::{
        uniform::{UniformAdapter, UniformFloat, UniformVec3},
        RenderState, ShaderProgram, UniformMatrix4,
    },
};
use cgmath::Vector3;
use std::cell::RefCell;
use std::f32::consts::PI;
use std::rc::Rc;

/// Shared frame state read by the per-render uniform extractors.
struct FrameState {
    vp:          cgmath::Matrix4<f32>,
    disc_center: Vector3<f32>,
    disc_normal: Vector3<f32>,
    disc_radius: f32,
    time:        f32,
}

pub struct DiscRenderer {
    vao:        Vao,
    vert_count: i32,
    shader:     ShaderProgram<NoopProcessable>,
    frame:      Rc<RefCell<FrameState>>,
    pub disc_center: Vector3<f32>,
    pub disc_normal: Vector3<f32>,
    pub disc_radius: f32,
    pub time:        f32,
    pub active:      bool,
}

impl DiscRenderer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let segments = 64usize;
        let mut verts: Vec<f32> = Vec::with_capacity((segments + 2) * 2);
        verts.push(0.0); verts.push(0.0);
        for i in 0..=segments {
            let angle = 2.0 * PI * i as f32 / segments as f32;
            verts.push(angle.cos());
            verts.push(angle.sin());
        }
        let vert_count = (segments + 2) as i32;

        let buf     = DataBuffer::load_static(&verts);
        let mut vao = Vao::create();
        let attr    = Attribute::of(0, 2, DataType::Float, false);
        vao.load_data_buffer(Rc::new(buf) as Rc<dyn IVbo>, &[attr]);

        let vert_src = include_str!("../../../../assets/shaders/disc.vert.glsl");
        let frag_src = include_str!("../../../../assets/shaders/disc.frag.glsl");
        let mut shader = ShaderProgram::<NoopProcessable>::from_sources(vert_src, frag_src)?;

        let frame = Rc::new(RefCell::new(FrameState {
            vp:          cgmath::Matrix4::from_scale(1.0),
            disc_center: Vector3::new(0.0, 0.0, 0.0),
            disc_normal: Vector3::new(0.0, 1.0, 0.0),
            disc_radius: 1.5,
            time:        0.0,
        }));

        {
            let f = Rc::clone(&frame);
            shader.add_per_render_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform: UniformMatrix4::new("uVP"),
                extractor: Box::new(move |_: &RenderState<NoopProcessable>| f.borrow().vp),
            })));
        }
        {
            let f = Rc::clone(&frame);
            shader.add_per_render_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform: UniformVec3::new("uDiscCenter"),
                extractor: Box::new(move |_: &RenderState<NoopProcessable>| {
                    f.borrow().disc_center
                }),
            })));
        }
        {
            let f = Rc::clone(&frame);
            shader.add_per_render_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform: UniformVec3::new("uDiscNormal"),
                extractor: Box::new(move |_: &RenderState<NoopProcessable>| {
                    f.borrow().disc_normal
                }),
            })));
        }
        {
            let f = Rc::clone(&frame);
            shader.add_per_render_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform: UniformFloat::new("uDiscRadius"),
                extractor: Box::new(move |_: &RenderState<NoopProcessable>| {
                    f.borrow().disc_radius
                }),
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
            vert_count,
            shader,
            frame,
            disc_center: Vector3::new(0.0, 0.0, 0.0),
            disc_normal: Vector3::new(0.0, 1.0, 0.0),
            disc_radius: 1.5,
            time:   0.0,
            active: false,
        })
    }
}

impl IRenderer for DiscRenderer {
    fn render(&mut self, context: &SceneContext) {
        if !self.active { return; }

        let camera = context.get_camera();
        let vp     = *camera.borrow().get_projection_view_matrix();

        // Sync public fields into the shared frame state the extractors read.
        {
            let mut f    = self.frame.borrow_mut();
            f.vp          = vp;
            f.disc_center = self.disc_center;
            f.disc_normal = self.disc_normal;
            f.disc_radius = self.disc_radius;
            f.time        = self.time;
        }

        unsafe {
            gl::Disable(gl::CULL_FACE);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::DepthMask(gl::FALSE);
        }

        self.shader.bind();
        // Screenspace pass: no scene instance; the VP comes from FrameState.
        let state = RenderState::new_screenspace(self);
        self.shader.update_per_render_uniforms(&state);

        self.vao.bind();
        self.vao.enable_attributes();
        unsafe { gl::DrawArrays(gl::TRIANGLE_FAN, 0, self.vert_count); }
        self.vao.unbind();
        self.shader.unbind();

        unsafe {
            gl::DepthMask(gl::TRUE);
            gl::Enable(gl::CULL_FACE);
            gl::Disable(gl::BLEND);
        }
    }

    fn any_processed(&self) -> bool { self.active }
    fn finish(&mut self) {}
}
