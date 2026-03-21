use crate::{
    engine::{
        architecture::{models::simple_model::SimpleModel, scene::scene_context::SceneContext},
        rendering::abstracted::{irenderer::IRenderer, processable::Processable},
    },
    opengl::{
        fbos::fbo::Fbo,
        shaders::{compute_program::ComputeProgram, RenderState},
    },
};

pub struct CameraOnly;
#[allow(refining_impl_trait)]
impl Processable for CameraOnly {
    fn get_model(&self) -> &SimpleModel {
        panic!("CameraOnly does not have a model")
    }

    fn process(&mut self) {}
}

pub struct LightingPass {
    program: ComputeProgram<CameraOnly>,
}

impl LightingPass {
    pub fn new() -> Self {
        let src = include_str!("../../../assets/shaders/deferred_lighting.comp.glsl");
        let program =
            ComputeProgram::from_source(src).expect("Failed to compile lighting compute shader");

        Self { program }
    }

    pub fn execute(&self, deferred_fbo: &Fbo, scene_fbo: &mut Fbo, context: &SceneContext) {
        self.program.bind();
        let camera = context.get_camera();
        let camera_ref = camera.borrow();
        let render_state = RenderState::<CameraOnly>::new_without_instance(self, &camera_ref);

        unsafe {
            let albedo_texture_id = deferred_fbo.get_attachments()[0].get_texture().get_id();
            // TODO: I hate hard coding this, we should have the infos in the attachment wrapper.
            // We should also use our UniformSystem.
            gl::BindImageTexture(
                0,                 // image unit
                albedo_texture_id, // texture
                0,                 // level
                gl::FALSE,         // layered
                0,                 // layer
                gl::READ_ONLY,
                gl::RGBA16F, // format
            );

            let norm_texture_id = deferred_fbo.get_attachments()[1].get_texture().get_id();
            gl::BindImageTexture(
                1,               // image unit
                norm_texture_id, // texture
                0,               // level
                gl::FALSE,       // layered
                0,               // layer
                gl::READ_ONLY,
                gl::RGBA16F, // format
            );

            let pos_texture_id = deferred_fbo.get_attachments()[2].get_texture().get_id();
            gl::BindImageTexture(
                2,              // image unit
                pos_texture_id, // texture
                0,              // level
                gl::FALSE,      // layered
                0,              // layer
                gl::READ_ONLY,
                gl::RGBA16F, // format
            );

            let scene_texture_id = scene_fbo.get_attachments()[0].get_texture().get_id();
            gl::BindImageTexture(
                3,                // image unit
                scene_texture_id, // texture
                0,                // level
                gl::FALSE,        // layered
                0,                // layer
                gl::WRITE_ONLY,
                gl::RGBA16F, // format
            );
        }

        let group_size = 32;
        let groups_x = (scene_fbo.get_width() + group_size - 1) / group_size;
        let groups_y = (scene_fbo.get_height() + group_size - 1) / group_size;
        self.program.update_uniforms(&render_state);

        // Drain any stale errors before dispatch so the post-dispatch check
        // only catches errors from this call.
        unsafe { while gl::GetError() != gl::NO_ERROR {} }

        self.program.dispatch(
            groups_x.try_into().unwrap(),
            groups_y.try_into().unwrap(),
            1,
        );

        unsafe {
            let err = gl::GetError();
            if err != gl::NO_ERROR {
                log::error!("Dispatch failed: {:#X}", err);
            }
        }

        // Memory barrier so compute writes are visible to subsequent reads.
        self.program.memory_barrier(gl::SHADER_IMAGE_ACCESS_BARRIER_BIT);

        self.program.unbind();

        // ── Unbind image units immediately after dispatch ─────────────────
        // Image unit bindings persist across draw calls on GLES.  On Adreno
        // the driver validates *all* active image units against *all* active
        // texture sampler units at every draw call — even when the draw call
        // doesn't use compute at all.  If a texture is simultaneously bound
        // as an image (read/write) AND as a sampler (e.g. entity albedo on
        // TEXTURE0 == image unit 0), the driver raises GL_INVALID_OPERATION.
        // We clear all four units here, right after dispatch, so rasterisation
        // passes run with a clean image-unit table.
        unsafe {
            for unit in 0u32..4 {
                gl::BindImageTexture(unit, 0, 0, gl::FALSE, 0, gl::READ_ONLY, gl::RGBA8);
            }
        }
    }
}

impl IRenderer for LightingPass {
    fn render(&mut self, _context: &SceneContext) {}

    fn any_processed(&self) -> bool {
        true
    }

    fn finish(&mut self) {
        log::info!("Clening up Lighting...");
    }
}
