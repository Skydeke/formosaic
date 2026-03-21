//! Axis-plane disc renderer (Hint Tier 2).
//!
//! Renders a translucent disc perpendicular to the solution axis.
//! Uses SimpleProgram for all shader/uniform operations.

use crate::engine::architecture::scene::scene_context::SceneContext;
use crate::engine::rendering::abstracted::irenderer::IRenderer;
use crate::opengl::{
    constants::data_type::DataType,
    objects::{attribute::Attribute, data_buffer::DataBuffer, ivbo::IVbo, vao::Vao},
    shaders::SimpleProgram,
};
use cgmath::{Matrix, Vector3};
use std::f32::consts::PI;
use std::rc::Rc;

pub struct DiscRenderer {
    vao:             Vao,
    vert_count:      i32,
    program:         SimpleProgram,
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
        let segments = 64usize;
        let mut verts: Vec<f32> = Vec::with_capacity((segments + 2) * 2);
        verts.push(0.0); verts.push(0.0);
        for i in 0..=segments {
            let angle = 2.0 * PI * i as f32 / segments as f32;
            verts.push(angle.cos());
            verts.push(angle.sin());
        }
        let vert_count = (segments + 2) as i32;

        let buf  = DataBuffer::load_static(&verts);
        let mut vao  = Vao::create();
        let attr = Attribute::of(0, 2, DataType::Float, false);
        vao.load_data_buffer(Rc::new(buf) as Rc<dyn IVbo>, &[attr]);

        let vert_src = include_str!("../../../../assets/shaders/disc.vert.glsl");
        let frag_src = include_str!("../../../../assets/shaders/disc.frag.glsl");
        let program  = SimpleProgram::from_sources(vert_src, frag_src)
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;

        let loc_vp          = program.uniform_location("uVP");
        let loc_disc_center = program.uniform_location("uDiscCenter");
        let loc_disc_normal = program.uniform_location("uDiscNormal");
        let loc_disc_radius = program.uniform_location("uDiscRadius");
        let loc_time        = program.uniform_location("uTime");

        Ok(Self {
            vao, vert_count, program,
            loc_vp, loc_disc_center, loc_disc_normal, loc_disc_radius, loc_time,
            disc_center: Vector3::new(0.0, 0.0, 0.0),
            disc_normal: Vector3::new(0.0, 1.0, 0.0),
            disc_radius: 1.5,
            time: 0.0, active: false,
        })
    }
}

impl IRenderer for DiscRenderer {
    fn render(&mut self, context: &SceneContext) {
        if !self.active { return; }

        let camera = context.get_camera();
        let cam    = camera.borrow();
        let vp     = *cam.get_projection_view_matrix();

        // cgmath Matrix4 is column-major, same layout as GLSL mat4.
        let vp_arr: &[f32; 16] = unsafe { &*(vp.as_ptr() as *const [f32; 16]) };

        unsafe {
            gl::Disable(gl::CULL_FACE);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::DepthMask(gl::FALSE);
        }

        self.program.bind();
        self.program.set_uniform_mat4(self.loc_vp, vp_arr);
        self.program.set_uniform_vec3(self.loc_disc_center,
            self.disc_center.x, self.disc_center.y, self.disc_center.z);
        self.program.set_uniform_vec3(self.loc_disc_normal,
            self.disc_normal.x, self.disc_normal.y, self.disc_normal.z);
        self.program.set_uniform_float(self.loc_disc_radius, self.disc_radius);
        self.program.set_uniform_float(self.loc_time,        self.time);

        self.vao.bind();
        self.vao.enable_attributes();
        unsafe { gl::DrawArrays(gl::TRIANGLE_FAN, 0, self.vert_count); }
        self.vao.unbind();
        self.program.unbind();

        unsafe {
            gl::DepthMask(gl::TRUE);
            gl::Enable(gl::CULL_FACE);
            gl::Disable(gl::BLEND);
        }
    }

    fn any_processed(&self) -> bool { self.active }
    fn finish(&mut self) {}
}
