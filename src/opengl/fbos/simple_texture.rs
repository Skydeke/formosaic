use crate::opengl::{
    constants::{data_type::DataType, format_type::FormatType},
    textures::{texture::Texture, texture_configs::TextureConfigs, texture_target::TextureTarget},
};
use std::ptr;

#[derive(Debug)]
pub struct SimpleTexture {
    id: u32,
}

impl SimpleTexture {
    pub fn new(id: u32) -> Self {
        Self { id }
    }
}

impl Texture for SimpleTexture {
    fn bind_to_unit(&self, unit: u32) {
        unsafe {
            gl::ActiveTexture(gl::TEXTURE0 + unit);
            gl::BindTexture(gl::TEXTURE_2D, self.id);
        }
    }

    fn bind(&self) {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.id);
        }
    }

    fn unbind(&self) {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }

    fn delete(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.id);
        }
    }

    fn get_id(&self) -> u32 {
        self.id
    }

    fn allocate(
        &self,
        target: TextureTarget,
        level: i32,
        internal_format: FormatType,
        width: i32,
        height: i32,
        border: i32,
        format: FormatType,
        data_type: DataType,
        data: *const std::ffi::c_void,
    ) {
        unsafe {
            gl::BindTexture(target.get(), self.id);
            gl::TexImage2D(
                target.get(),
                level,
                internal_format.get() as i32,
                width,
                height,
                border,
                format.get(),
                data_type.value(),
                data,
            );
        }
    }

    fn attach_to_fbo(&self, attachment_point: i32, level: i32) {
        unsafe {
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                attachment_point as u32,
                gl::TEXTURE_2D,
                self.id,
                level,
            );
        }
    }

    fn apply_configs(&self, configs: &TextureConfigs) {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.id);

            if let Some(mag_filter) = &configs.mag_filter {
                gl::TexParameteri(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_MAG_FILTER,
                    mag_filter.get() as i32,
                );
            }

            if let Some(min_filter) = &configs.min_filter {
                gl::TexParameteri(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_MIN_FILTER,
                    min_filter.get() as i32,
                );
            }
        }
    }

    fn clone_texture(&self) -> Box<dyn Texture> {
        Box::new(SimpleTexture::new(self.id))
    }
}
