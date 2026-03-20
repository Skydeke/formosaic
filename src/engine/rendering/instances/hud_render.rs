//! HUD overlay renderer.
//!
//! Renders a fullscreen-quad-based warmth compass in the bottom-left corner
//! using the `hud.vert/frag.glsl` shaders.  The compass is a 2D overlay — it
//! does not write depth and is drawn after all 3D passes.

use crate::engine::architecture::scene::scene_context::SceneContext;
use crate::engine::rendering::abstracted::irenderer::IRenderer;
use crate::opengl::{
    constants::data_type::DataType,
    objects::{attribute::Attribute, data_buffer::DataBuffer, ivbo::IVbo, vao::Vao},
    shaders::shader::Shader,
};
use std::rc::Rc;

pub struct HudRenderer {
    vao: Vao,
    shader_id: u32,
    /// Uniforms cached by location.
    loc_warmth_color: i32,
    loc_warmth:       i32,
    loc_hint_tier:    i32,
    loc_time:         i32,
    /// State updated each frame.
    pub warmth_color: [f32; 3],
    pub warmth:       f32,
    pub hint_tier:    f32,   // 0..3
    pub time:         f32,
    pub active:       bool,
}

impl HudRenderer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Full-screen triangle pair covering NDC [-1,1]².
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
        let vert     = Shader::vertex(vert_src)?;
        let frag     = Shader::fragment(frag_src)?;

        // Compile manually so we can grab the raw program id for uniform locations.
        // We don't use ShaderProgram<T> here because the HUD doesn't need per-instance uniforms.
        let prog_id  = unsafe {
            let id = gl::CreateProgram();
            vert.attach(id);
            frag.attach(id);
            gl::LinkProgram(id);
            let mut ok = 0i32;
            gl::GetProgramiv(id, gl::LINK_STATUS, &mut ok);
            if ok != gl::TRUE as i32 {
                return Err("HUD shader link failed".into());
            }
            vert.detach(id); vert.delete();
            frag.detach(id); frag.delete();
            id
        };

        let loc = |name: &str| -> i32 {
            let c = std::ffi::CString::new(name).unwrap();
            unsafe { gl::GetUniformLocation(prog_id, c.as_ptr()) }
        };

        Ok(Self {
            vao,
            shader_id:        prog_id,
            loc_warmth_color: loc("uWarmthColor"),
            loc_warmth:       loc("uWarmth"),
            loc_hint_tier:    loc("uHintTier"),
            loc_time:         loc("uTime"),
            warmth_color:     [1.0, 1.0, 1.0],
            warmth:           0.0,
            hint_tier:        0.0,
            time:             0.0,
            active:           false,
        })
    }
}

impl IRenderer for HudRenderer {
    fn render(&mut self, _context: &SceneContext) {
        if !self.active || self.hint_tier < 0.5 { return; }

        unsafe {
            // 2D overlay: no depth test, additive-ish blend.
            gl::Disable(gl::DEPTH_TEST);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

            gl::UseProgram(self.shader_id);

            if self.loc_warmth_color >= 0 {
                gl::Uniform3f(self.loc_warmth_color,
                    self.warmth_color[0], self.warmth_color[1], self.warmth_color[2]);
            }
            if self.loc_warmth    >= 0 { gl::Uniform1f(self.loc_warmth, self.warmth); }
            if self.loc_hint_tier >= 0 { gl::Uniform1f(self.loc_hint_tier, self.hint_tier); }
            if self.loc_time      >= 0 { gl::Uniform1f(self.loc_time, self.time); }

            self.vao.bind();
            self.vao.enable_attributes();
            gl::DrawArrays(gl::TRIANGLES, 0, 6);
            self.vao.unbind();

            gl::UseProgram(0);
            gl::Enable(gl::DEPTH_TEST);
            gl::Disable(gl::BLEND);
        }
    }

    fn any_processed(&self) -> bool { self.active }
    fn finish(&mut self) {}
}

impl Drop for HudRenderer {
    fn drop(&mut self) {
        unsafe { gl::DeleteProgram(self.shader_id); }
    }
}
