use gl;

use crate::opengl::constants::format_type::FormatType;

pub struct RenderBuffer {
    id: u32,
    deleted: bool,
}
// Tracks currently bound render buffer to avoid redundant binds
static mut BOUND_RENDERBUFFER: u32 = 0;

impl RenderBuffer {
    fn new(id: u32) -> Self {
        let mut rb = Self { id, deleted: false };
        rb.bind();
        rb
    }

    pub fn create() -> Self {
        let mut id: u32 = 0;
        unsafe { gl::GenRenderbuffers(1, &mut id) };
        Self::new(id)
    }

    pub fn load_storage(&self, width: i32, height: i32, i_format: FormatType) {
        unsafe {
            gl::RenderbufferStorage(gl::RENDERBUFFER, i_format.get(), width, height);
        }
    }

    pub fn load_storage_multisample(
        &self,
        width: i32,
        height: i32,
        i_format: FormatType,
        samples: i32,
    ) {
        unsafe {
            gl::RenderbufferStorageMultisample(
                gl::RENDERBUFFER,
                samples,
                i_format.get(),
                width,
                height,
            );
        }
    }

    pub fn attach_to_fbo(&self, attachment: u32) {
        unsafe {
            gl::FramebufferRenderbuffer(gl::FRAMEBUFFER, attachment, gl::RENDERBUFFER, self.id);
        }
    }

    pub fn bind(&self) {
        unsafe {
            if self::BOUND_RENDERBUFFER != self.id {
                self::BOUND_RENDERBUFFER = self.id;
                gl::BindRenderbuffer(gl::RENDERBUFFER, self.id);
            }
        }
    }

    pub fn unbind(&self) {
        unsafe {
            if self::BOUND_RENDERBUFFER != 0 {
                self::BOUND_RENDERBUFFER = 0;
                gl::BindRenderbuffer(gl::RENDERBUFFER, 0);
            }
        }
    }

    pub fn delete(&mut self) {
        if !self.deleted {
            self.deleted = true;
            unsafe { gl::DeleteRenderbuffers(1, &self.id) };
        }
    }
}
