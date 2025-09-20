use cgmath::Array;

use crate::opengl::{
    constants::{data_type::DataType, format_type::FormatType},
    fbos::{
        attachment::{
            abstract_attachment::AbstractAttachment, attachment::Attachment,
            attachment_type::AttachmentType,
        },
        fbo::Fbo,
        simple_texture::SimpleTexture,
    },
    textures::{texture::Texture, texture_configs::TextureConfigs, texture_target::TextureTarget},
};
use std::ptr;

pub struct TextureAttachment {
    base: AbstractAttachment,
    texture_id: u32,
    configs: TextureConfigs,
}

impl TextureAttachment {
    pub fn of_colour_without_config(index: i32) -> Box<Self> {
        Self::of_colour(index, TextureConfigs::default())
    }

    pub fn of_colour(index: i32, configs: TextureConfigs) -> Box<Self> {
        let mut texture_id: u32 = 0;
        unsafe { gl::GenTextures(1, &mut texture_id) };

        Box::new(Self {
            base: AbstractAttachment::new(AttachmentType::Colour, index),
            texture_id,
            configs,
        })
    }

    pub fn of_depth(configs: TextureConfigs) -> Box<Self> {
        let mut texture_id: u32 = 0;
        unsafe { gl::GenTextures(1, &mut texture_id) };

        Box::new(Self {
            base: AbstractAttachment::new(AttachmentType::Depth, 0),
            texture_id,
            configs,
        })
    }

    pub fn of_stencil() -> Box<Self> {
        let mut texture_id: u32 = 0;
        unsafe { gl::GenTextures(1, &mut texture_id) };
        let configs = TextureConfigs::default();

        Box::new(Self {
            base: AbstractAttachment::new(AttachmentType::Stencil, 0),
            texture_id,
            configs,
        })
    }

    pub fn of_depth_stencil() -> Box<Self> {
        let mut texture_id: u32 = 0;
        unsafe { gl::GenTextures(1, &mut texture_id) };
        let configs = TextureConfigs::default();

        Box::new(Self {
            base: AbstractAttachment::new(AttachmentType::DepthStencil, 0),
            texture_id,
            configs,
        })
    }

    fn allocate_texture(
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
            gl::BindTexture(target.get(), self.texture_id);
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
                self.texture_id,
                level,
            );
        }
    }

    pub fn apply_configs(&self) {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.texture_id);

            // Filters
            if let Some(mag_filter) = &self.configs.mag_filter {
                gl::TexParameteri(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_MAG_FILTER,
                    mag_filter.get() as i32,
                );
            }
            if let Some(min_filter) = &self.configs.min_filter {
                gl::TexParameteri(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_MIN_FILTER,
                    min_filter.get() as i32,
                );
            }

            // Wrap modes
            if let Some(wrap_s) = &self.configs.wrap_s {
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, wrap_s.get() as i32);
            }
            if let Some(wrap_t) = &self.configs.wrap_t {
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, wrap_t.get() as i32);
            }

            // LOD bias
            if self.configs.level_of_detail_bias != 0.0 {
                gl::TexParameterf(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_LOD_BIAS,
                    self.configs.level_of_detail_bias,
                );
            }

            // Border color (desktop only)
            if self.configs.border_colour != cgmath::Vector4::new(0.0, 0.0, 0.0, 0.0) {
                gl::TexParameterfv(
                    gl::TEXTURE_2D,
                    gl::TEXTURE_BORDER_COLOR,
                    self.configs.border_colour.as_ptr(),
                );
            }
        }
    }
}

impl Attachment for TextureAttachment {
    fn get_attachment_point(&self) -> i32 {
        self.base.get_attachment_point()
    }

    fn get_attachment_type(&self) -> AttachmentType {
        self.base.get_attachment_type()
    }

    fn get_texture(&self) -> Box<dyn Texture> {
        Box::new(SimpleTexture::new(self.texture_id))
    }

    fn init(&mut self, fbo: &Fbo) {
        self.allocate_texture(
            TextureTarget::Texture2D,
            0,
            self.configs.internal_format,
            fbo.get_width(),
            fbo.get_height(),
            0,
            self.configs.format,
            self.configs.data_type,
            ptr::null(),
        );
        self.attach_to_fbo(self.get_attachment_point(), 0);
        self.apply_configs();
    }

    fn resize(&mut self, width: i32, height: i32) {
        self.allocate_texture(
            TextureTarget::Texture2D,
            0,
            self.configs.internal_format,
            width,
            height,
            0,
            self.configs.format,
            self.configs.data_type,
            ptr::null(),
        );
        self.attach_to_fbo(self.get_attachment_point(), 0);
        self.apply_configs();
    }

    fn delete(&mut self) {
        unsafe { gl::DeleteTextures(1, &self.texture_id) };
    }

    fn clone_attachment(&self) -> Box<dyn Attachment> {
        // Create a new texture with the same configuration
        let mut new_texture_id: u32 = 0;
        unsafe { gl::GenTextures(1, &mut new_texture_id) };

        Box::new(TextureAttachment {
            base: AbstractAttachment::new(
                self.base.get_attachment_type(),
                self.get_attachment_point() - (self.base.get_attachment_type().get() as i32),
            ),
            texture_id: new_texture_id,
            configs: self.configs.clone(),
        })
    }
}
