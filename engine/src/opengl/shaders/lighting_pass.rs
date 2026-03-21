//! Deferred lighting compute pass.
//!
//! Lighting uniforms are registered via the standard UniformAdapter system —
//! no raw gl::GetUniformLocation calls.  The LightingPass owns an
//! Rc<RefCell<LightConfig>> that execute() writes into each frame; the
//! UniformAdapter closures read from it when update_uniforms() fires.

use std::{cell::RefCell, rc::Rc};
use cgmath::Vector3;

use crate::{
    architecture::{models::simple_model::SimpleModel, scene::scene_context::SceneContext},
    rendering::{
        abstracted::{irenderer::IRenderer, processable::Processable},
        render_state::LightConfig,
    },
    opengl::{
        fbos::fbo::Fbo,
        shaders::{
            compute_program::ComputeProgram,
            uniform::{UniformAdapter, UniformFloat, UniformVec3},
            RenderState,
        },
    },
};

// CameraOnly satisfies the Processable bound for RenderState but is never
// used as an actual scene object — get_model() must never be called on it.
pub struct CameraOnly;
#[allow(refining_impl_trait)]
impl Processable for CameraOnly {
    fn get_model(&self) -> &SimpleModel { panic!("CameraOnly does not have a model") }
    fn process(&mut self) {}
}

pub struct LightingPass {
    program: ComputeProgram<CameraOnly>,
    /// Shared lighting state written by execute() and read by uniform closures.
    lights:  Rc<RefCell<LightConfig>>,
}

impl LightingPass {
    pub fn new() -> Self {
        let src     = include_str!("../../../assets/shaders/deferred_lighting.comp.glsl");
        let mut program = ComputeProgram::from_source(src)
            .expect("Failed to compile lighting compute shader");

        let lights = Rc::new(RefCell::new(LightConfig::default()));

        // Register every lighting uniform through the standard adapter system.
        {
            let l = Rc::clone(&lights);
            program.add_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform:   UniformVec3::new("uClearColor"),
                extractor: Box::new(move |_: &RenderState<CameraOnly>| {
                    let c = l.borrow().clear_color;
                    Vector3::new(c[0], c[1], c[2])
                }),
            })));
        }
        {
            let l = Rc::clone(&lights);
            program.add_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform:   UniformVec3::new("uSunDir"),
                extractor: Box::new(move |_: &RenderState<CameraOnly>| {
                    let d = l.borrow().sun_dir;
                    Vector3::new(d[0], d[1], d[2])
                }),
            })));
        }
        {
            let l = Rc::clone(&lights);
            program.add_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform:   UniformVec3::new("uSunColor"),
                extractor: Box::new(move |_: &RenderState<CameraOnly>| {
                    let c = l.borrow().sun_color;
                    Vector3::new(c[0], c[1], c[2])
                }),
            })));
        }
        {
            let l = Rc::clone(&lights);
            program.add_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform:   UniformVec3::new("uSkyColor"),
                extractor: Box::new(move |_: &RenderState<CameraOnly>| {
                    let c = l.borrow().sky_color;
                    Vector3::new(c[0], c[1], c[2])
                }),
            })));
        }
        {
            let l = Rc::clone(&lights);
            program.add_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform:   UniformFloat::new("uAmbientMin"),
                extractor: Box::new(move |_: &RenderState<CameraOnly>| {
                    l.borrow().ambient_min
                }),
            })));
        }

        Self { program, lights }
    }

    pub fn execute(&self, deferred_fbo: &Fbo, scene_fbo: &mut Fbo, context: &SceneContext) {
        // Write current frame's lighting into the shared state so extractors see it.
        *self.lights.borrow_mut() = context.lights;

        self.program.bind();

        unsafe {
            let albedo_texture_id = deferred_fbo.get_attachments()[0].get_texture().get_id();
            gl::BindImageTexture(0, albedo_texture_id, 0, gl::FALSE, 0, gl::READ_ONLY,  gl::RGBA16F);

            let norm_texture_id = deferred_fbo.get_attachments()[1].get_texture().get_id();
            gl::BindImageTexture(1, norm_texture_id,   0, gl::FALSE, 0, gl::READ_ONLY,  gl::RGBA16F);

            let pos_texture_id = deferred_fbo.get_attachments()[2].get_texture().get_id();
            gl::BindImageTexture(2, pos_texture_id,    0, gl::FALSE, 0, gl::READ_ONLY,  gl::RGBA16F);

            let scene_texture_id = scene_fbo.get_attachments()[0].get_texture().get_id();
            gl::BindImageTexture(3, scene_texture_id,  0, gl::FALSE, 0, gl::WRITE_ONLY, gl::RGBA16F);
        }

        let group_size = 32;
        let groups_x = (scene_fbo.get_width()  + group_size - 1) / group_size;
        let groups_y = (scene_fbo.get_height() + group_size - 1) / group_size;

        let camera       = context.get_camera();
        let camera_ref   = camera.borrow();
        let render_state = RenderState::<CameraOnly>::new_without_instance(self, &camera_ref);
        self.program.update_uniforms(&render_state);

        unsafe { while gl::GetError() != gl::NO_ERROR {} }

        self.program.dispatch(
            groups_x.try_into().unwrap(),
            groups_y.try_into().unwrap(),
            1,
        );

        unsafe {
            let err = gl::GetError();
            if err != gl::NO_ERROR { log::error!("Lighting dispatch failed: {:#X}", err); }
        }

        self.program.memory_barrier(gl::SHADER_IMAGE_ACCESS_BARRIER_BIT);
        self.program.unbind();

        // Clear image unit bindings — GLES validates all active image units on
        // every draw call, even non-compute ones.  Stale bindings cause errors.
        unsafe {
            for unit in 0u32..4 {
                gl::BindImageTexture(unit, 0, 0, gl::FALSE, 0, gl::READ_ONLY, gl::RGBA8);
            }
        }
    }
}

impl IRenderer for LightingPass {
    fn render(&mut self, _context: &SceneContext) {}
    fn any_processed(&self) -> bool { true }
    fn finish(&mut self) {}
}
