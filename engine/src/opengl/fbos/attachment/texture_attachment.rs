
use crate::opengl::{
    fbos::{
        attachment::{
            abstract_attachment::AbstractAttachment, attachment::Attachment,
            attachment_type::AttachmentType,
        },
        fbo::Fbo,
        simple_texture::SimpleTexture,
    },
    textures::{texture::Texture, texture_configs::TextureConfigs},
};

pub struct TextureAttachment {
    base: AbstractAttachment,
    texture: Box<dyn Texture>,
    configs: TextureConfigs,
}

impl TextureAttachment {
    pub fn of_colour(index: i32, configs: TextureConfigs) -> Box<Self> {
        let texture = Box::new(SimpleTexture::create());

        let this = Box::new(Self {
            base: AbstractAttachment::new(AttachmentType::Colour, index),
            texture,
            configs,
        });

        this
    }

    pub fn of_depth(configs: TextureConfigs) -> Box<Self> {
        let texture = Box::new(SimpleTexture::create());

        let this = Box::new(Self {
            base: AbstractAttachment::new(AttachmentType::Depth, 0),
            texture,
            configs,
        });

        this
    }

    pub fn of_stencil() -> Box<Self> {
        let texture = Box::new(SimpleTexture::create());
        let configs = TextureConfigs::default();

        let this = Box::new(Self {
            base: AbstractAttachment::new(AttachmentType::Stencil, 0),
            texture,
            configs,
        });

        this
    }

    pub fn of_depth_stencil() -> Box<Self> {
        let texture = Box::new(SimpleTexture::create());
        let configs = TextureConfigs::default();

        let this = Box::new(Self {
            base: AbstractAttachment::new(AttachmentType::DepthStencil, 0),
            texture,
            configs,
        });

        this
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
        self.texture.clone_texture()
    }

    fn init(&mut self, fbo: &Fbo) {
        // Allocate texture storage if needed
        if let Some(simple_texture) = self.texture.as_any_mut().downcast_mut::<SimpleTexture>() {
            simple_texture.allocate_if_needed(fbo.get_width(), fbo.get_height(), &self.configs);
        }

        // Apply texture parameters
        self.texture.apply_configs(&self.configs);

        // Attach to framebuffer
        self.texture.attach_to_fbo(self.get_attachment_point(), 0);
    }

    fn resize(&mut self, width: i32, height: i32) {
        // Only reallocate if size actually changed
        if let Some(simple_texture) = self.texture.as_any_mut().downcast_mut::<SimpleTexture>() {
            simple_texture.allocate_if_needed(width, height, &self.configs);
        }

        // Reattach to framebuffer (attachment point doesn't change)
        self.texture.attach_to_fbo(self.get_attachment_point(), 0);
    }

    fn delete(&mut self) {
        self.texture.delete();
    }

    fn clone_attachment(&self) -> Box<dyn Attachment> {
        let cloned_texture = self.texture.clone_texture();

        Box::new(TextureAttachment {
            base: AbstractAttachment::new(
                self.base.get_attachment_type(),
                self.get_attachment_point() - (self.base.get_attachment_type().get() as i32),
            ),
            texture: cloned_texture,
            configs: self.configs.clone(),
        })
    }
}
