use crate::opengl::textures::texture::Texture;

pub struct RenderOutputData {
    pub colour: Box<dyn Texture>,
    pub normal: Box<dyn Texture>,
    pub depth: Box<dyn Texture>,
    pub position: Box<dyn Texture>,
}

impl RenderOutputData {
    pub fn new(
        colour: Box<dyn Texture>,
        normal: Box<dyn Texture>,
        depth: Box<dyn Texture>,
        position: Box<dyn Texture>,
    ) -> Self {
        Self {
            colour,
            normal,
            depth,
            position,
        }
    }
}
