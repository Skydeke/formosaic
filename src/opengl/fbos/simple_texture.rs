use cgmath::Array;

use crate::opengl::textures::{texture::Texture, texture_configs::TextureConfigs};
use std::any::Any;

#[derive(Debug)]
pub struct SimpleTexture {
    id: u32,
    width: i32,
    height: i32,
    allocated: bool,
}

impl SimpleTexture {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            width: 0,
            height: 0,
            allocated: false,
        }
    }

    pub fn create() -> Self {
        let mut id: u32 = 0;
        unsafe { gl::GenTextures(1, &mut id) };
        Self::new(id)
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
        self.allocated = false;
    }

    fn get_id(&self) -> u32 {
        self.id
    }

    fn allocate_if_needed(&mut self, width: i32, height: i32, configs: &TextureConfigs) {
        // Only allocate if size changed or not yet allocated
        if !self.allocated || self.width != width || self.height != height {
            // If already allocated, delete the old texture first
            if self.allocated {
                unsafe {
                    gl::DeleteTextures(1, &self.id);
                }
                // Generate a new texture ID
                unsafe {
                    gl::GenTextures(1, &mut self.id);
                }
            }

            self.width = width;
            self.height = height;
            self.allocated = true;

            unsafe {
                gl::BindTexture(gl::TEXTURE_2D, self.id);

                // Immutable allocation (ES3+ and desktop GL)
                gl::TexStorage2D(
                    gl::TEXTURE_2D,
                    1,
                    configs.internal_format.get(),
                    width,
                    height,
                );

                // Check for allocation errors
                let err = gl::GetError();
                if err != gl::NO_ERROR {
                    log::error!(
                        "Texture allocation failed: {:#X}, format: {:?}, size: {}x{}",
                        err,
                        configs.internal_format,
                        width,
                        height
                    );
                    self.allocated = false;
                }

                gl::BindTexture(gl::TEXTURE_2D, 0);
            }
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
            // Filters
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

            // Wrap modes
            if let Some(wrap_s) = &configs.wrap_s {
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, wrap_s.get() as i32);
            }
            if let Some(wrap_t) = &configs.wrap_t {
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, wrap_t.get() as i32);
            }

            // LOD bias
            if configs.level_of_detail_bias != 0.0 {
                gl::TexParameterf(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_LOD_BIAS,
                    configs.level_of_detail_bias,
                );
            }

            // Border color (desktop only)
            if configs.border_colour != cgmath::Vector4::new(0.0, 0.0, 0.0, 0.0) {
                gl::TexParameterfv(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_BORDER_COLOR,
                    configs.border_colour.as_ptr(),
                );
            }
        }
    }

    fn clone_texture(&self) -> Box<dyn Texture> {
        Box::new(SimpleTexture::new(self.id))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
