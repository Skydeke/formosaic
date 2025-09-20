use crate::opengl::textures::parameters::texture_parameter::TextureParameter;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MagFilterParameter {
    Nearest,
    Linear,
}

impl MagFilterParameter {
    pub fn get(&self) -> u32 {
        match self {
            MagFilterParameter::Nearest => gl::NEAREST,
            MagFilterParameter::Linear => gl::LINEAR,
        }
    }
}
