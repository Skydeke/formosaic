use crate::opengl::{
    constants::gl_buffer::GlBuffer, fbos::fbo_target::FboTarget,
    textures::parameters::mag_filter_parameter::MagFilterParameter,
};
use std::any::Any;

pub trait IFbo: Any {
    fn blit_fbo(&self, fbo: &dyn IFbo, filter: MagFilterParameter, buffers: &[GlBuffer]);

    fn get_width(&self) -> i32;
    fn get_height(&self) -> i32;
    fn resize(&mut self, width: i32, height: i32);

    fn bind(&self, target: FboTarget);
    fn unbind(&self, target: FboTarget);
    fn delete(&mut self);

    // Default methods
    fn blit_to(&self, fbo: &dyn IFbo) {
        self.blit_fbo(
            fbo,
            MagFilterParameter::Nearest,
            &[GlBuffer::Colour, GlBuffer::Depth],
        );
    }

    fn bind_default(&self) {
        self.bind(FboTarget::Framebuffer);
    }

    fn unbind_default(&self) {
        self.unbind(FboTarget::Framebuffer);
    }
}
