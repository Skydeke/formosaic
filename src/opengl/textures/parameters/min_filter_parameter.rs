use crate::opengl::textures::parameters::texture_parameter::TextureParameter;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MinFilterParameter {
    Nearest,
    Linear,
    NearestMipmapNearest,
    LinearMipmapNearest,
    NearestMipmapLinear,
    LinearMipmapLinear,
}

impl MinFilterParameter {
    pub fn get(&self) -> u32 {
        match self {
            MinFilterParameter::Nearest => gl::NEAREST,
            MinFilterParameter::Linear => gl::LINEAR,
            MinFilterParameter::NearestMipmapNearest => gl::NEAREST_MIPMAP_NEAREST,
            MinFilterParameter::LinearMipmapNearest => gl::LINEAR_MIPMAP_NEAREST,
            MinFilterParameter::NearestMipmapLinear => gl::NEAREST_MIPMAP_LINEAR,
            MinFilterParameter::LinearMipmapLinear => gl::LINEAR_MIPMAP_LINEAR,
        }
    }
}

impl TextureParameter for MinFilterParameter {
    fn get(&self) -> u32 {
        self.get()
    }
}
