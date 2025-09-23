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
                gl::RGBA32F, // format
            );

            let pos_texture_id = deferred_fbo.get_attachments()[2].get_texture().get_id();
            gl::BindImageTexture(
                2,              // image unit
                pos_texture_id, // texture
                0,              // level
                gl::FALSE,      // layered
                0,              // layer
                gl::READ_ONLY,
                gl::RGBA32F, // format
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

        // Dispatch compute shader with proper work group sizing
        // Typically you want work groups of 8x8 or 16x16
        let group_size = 32;
        let groups_x = (scene_fbo.get_width() + group_size - 1) / group_size;
        let groups_y = (scene_fbo.get_height() + group_size - 1) / group_size;
        self.program.update_uniforms(&render_state);
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

        // Memory barrier to ensure compute shader writes are visible
        self.program
            .memory_barrier(gl::SHADER_IMAGE_ACCESS_BARRIER_BIT);

        self.program.unbind();
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
