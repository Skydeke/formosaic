use crate::opengl::{
    fbos::{attachment::attachment_type::AttachmentType, fbo::Fbo},
    textures::texture::Texture,
};

// Remove Clone requirement to make trait object-safe
pub trait Attachment {
    fn get_attachment_point(&self) -> i32;
    fn get_attachment_type(&self) -> AttachmentType;
    fn get_texture(&self) -> Box<dyn Texture>;

    fn init(&mut self, fbo: &Fbo);
    fn resize(&mut self, width: i32, height: i32);
    fn delete(&mut self);

    // Add clone method that returns a new boxed attachment
    fn clone_attachment(&self) -> Box<dyn Attachment>;
}
