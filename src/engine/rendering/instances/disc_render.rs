//! Axis-plane disc renderer (Hint Tier 2).
//!
//! Renders a translucent dashed disc perpendicular to the solution axis
//! in world space, giving the player a visual cue about which plane to
//! look along — without revealing *which* of the two axial directions is correct.

use crate::engine::architecture::scene::scene_context::SceneContext;
use crate::engine::rendering::abstracted::irenderer::IRenderer;
use crate::opengl::{
    constants::data_type::DataType,
    objects::{attribute::Attribute, data_buffer::DataBuffer, ivbo::IVbo, vao::Vao},
    shaders::shader::Shader,
};
use cgmath::{Matrix, Vector3};
use std::f32::consts::PI;
use std::rc::Rc;

pub struct DiscRenderer {
    vao: Vao,
    vert_count: i32,
    shader_id: u32,
    loc_vp:          i32,
    loc_disc_center: i32,
    loc_disc_normal: i32,
    loc_disc_radius: i32,
    loc_time:        i32,
    pub disc_center: Vector3<f32>,
    pub disc_normal: Vector3<f32>,
    pub disc_radius: f32,
    pub time:        f32,
    pub active:      bool,
}

impl DiscRenderer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Build a unit disc mesh (triangle fan).
        let segments = 64usize;
        let mut verts: Vec<f32> = Vec::with_capacity((segments + 2) * 2);

        // Centre vertex.
        verts.push(0.0);
        verts.push(0.0);

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
        let vert     = Shader::vertex(vert_src)?;
        let frag     = Shader::fragment(frag_src)?;

        let prog_id = unsafe {
            let id = gl::CreateProgram();
            vert.attach(id);
            frag.attach(id);
            gl::LinkProgram(id);
            let mut ok = 0i32;
            gl::GetProgramiv(id, gl::LINK_STATUS, &mut ok);
            if ok != gl::TRUE as i32 {
                return Err("Disc shader link failed".into());
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
            vert_count,
            shader_id:       prog_id,
            loc_vp:          loc("uVP"),
            loc_disc_center: loc("uDiscCenter"),
            loc_disc_normal: loc("uDiscNormal"),
            loc_disc_radius: loc("uDiscRadius"),
            loc_time:        loc("uTime"),
            disc_center: Vector3::new(0.0, 0.0, 0.0),
            disc_normal: Vector3::new(0.0, 1.0, 0.0),
            disc_radius: 1.5,
            time:        0.0,
            active:      false,
        })
    }
}

impl IRenderer for DiscRenderer {
    fn render(&mut self, context: &SceneContext) {
        if !self.active { return; }

        let camera = context.get_camera();
        let cam    = camera.borrow();
        let vp     = cam.get_projection_view_matrix();

        unsafe {
            gl::Disable(gl::CULL_FACE);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            // Write to colour but not depth so it doesn't occlude the model.
            gl::DepthMask(gl::FALSE);

            gl::UseProgram(self.shader_id);

            if self.loc_vp >= 0 {
                gl::UniformMatrix4fv(self.loc_vp, 1, gl::FALSE, vp.as_ptr());
            }
            if self.loc_disc_center >= 0 {
                gl::Uniform3f(self.loc_disc_center,
                    self.disc_center.x, self.disc_center.y, self.disc_center.z);
            }
            if self.loc_disc_normal >= 0 {
                gl::Uniform3f(self.loc_disc_normal,
                    self.disc_normal.x, self.disc_normal.y, self.disc_normal.z);
            }
            if self.loc_disc_radius >= 0 {
                gl::Uniform1f(self.loc_disc_radius, self.disc_radius);
            }
            if self.loc_time >= 0 {
                gl::Uniform1f(self.loc_time, self.time);
            }

            self.vao.bind();
            self.vao.enable_attributes();
            gl::DrawArrays(gl::TRIANGLE_FAN, 0, self.vert_count);
            self.vao.unbind();

            gl::UseProgram(0);
            gl::DepthMask(gl::TRUE);
            gl::Enable(gl::CULL_FACE);
            gl::Disable(gl::BLEND);
        }
    }

    fn any_processed(&self) -> bool { self.active }
    fn finish(&mut self) {}
}

impl Drop for DiscRenderer {
    fn drop(&mut self) {
        unsafe { gl::DeleteProgram(self.shader_id); }
    }
}
