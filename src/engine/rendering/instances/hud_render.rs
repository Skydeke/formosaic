//! HUD overlay renderer (Hint Tier 1+).
//!
//! Renders a fullscreen-quad warmth compass using SimpleProgram.
//! All GL state changes go through the abstraction layer.

use crate::engine::architecture::scene::scene_context::SceneContext;
use crate::engine::rendering::abstracted::irenderer::IRenderer;
use crate::opengl::{
    constants::data_type::DataType,
    objects::{attribute::Attribute, data_buffer::DataBuffer, ivbo::IVbo, vao::Vao},
    shaders::SimpleProgram,
};
use std::rc::Rc;

pub struct HudRenderer {
    vao: Vao,
    program: SimpleProgram,
    loc_warmth_color: i32,
    loc_warmth: i32,
    loc_hint_tier: i32,
    loc_time: i32,
    pub warmth_color: [f32; 3],
    pub warmth: f32,
    pub hint_tier: f32,
    pub time: f32,
    pub active: bool,
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

        let buf = DataBuffer::load_static(&verts);
        let mut vao = Vao::create();
        let attr = Attribute::of(0, 2, DataType::Float, false);
        vao.load_data_buffer(Rc::new(buf) as Rc<dyn IVbo>, &[attr]);

        let vert_src = include_str!("../../../../assets/shaders/hud.vert.glsl");
        let frag_src = include_str!("../../../../assets/shaders/hud.frag.glsl");
        let program = SimpleProgram::from_sources(vert_src, frag_src)
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

        let loc_warmth_color = program.uniform_location("uWarmthColor");
        let loc_warmth = program.uniform_location("uWarmth");
        let loc_hint_tier = program.uniform_location("uHintTier");
        let loc_time = program.uniform_location("uTime");

        Ok(Self {
            vao,
            program,
            loc_warmth_color,
            loc_warmth,
            loc_hint_tier,
            loc_time,
            warmth_color: [1.0, 1.0, 1.0],
            warmth: 0.0,
            hint_tier: 0.0,
            time: 0.0,
            active: false,
        })
    }
}

impl IRenderer for HudRenderer {
    fn render(&mut self, _context: &SceneContext) {
        if !self.active || self.hint_tier < 0.5 {
            return;
        }

        unsafe {
            gl::Disable(gl::DEPTH_TEST);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }

        self.program.bind();
        self.program.set_uniform_vec3(
            self.loc_warmth_color,
            self.warmth_color[0],
            self.warmth_color[1],
            self.warmth_color[2],
        );
        self.program.set_uniform_float(self.loc_warmth, self.warmth);
        self.program
            .set_uniform_float(self.loc_hint_tier, self.hint_tier);
        self.program.set_uniform_float(self.loc_time, self.time);

        self.vao.bind();
        self.vao.enable_attributes();
        unsafe {
            gl::DrawArrays(gl::TRIANGLES, 0, 6);
        }
        self.vao.unbind();

        self.program.unbind();
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::Disable(gl::BLEND);
        }
    }

    fn any_processed(&self) -> bool {
        self.active
    }
    fn finish(&mut self) {}
}
