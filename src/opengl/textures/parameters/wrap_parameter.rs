use crate::opengl::textures::parameters::texture_parameter::TextureParameter;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WrapParameter {
    // Clamp,
    Repeat,
    ClampToEdge,
    ClampToBorder,
    MirroredRepeat,
}

impl WrapParameter {
    pub fn get(&self) -> u32 {
        match self {
            //WrapParameter::Clamp => gl::CLAMP,
            WrapParameter::Repeat => gl::REPEAT,
            WrapParameter::ClampToEdge => gl::CLAMP_TO_EDGE,
            WrapParameter::ClampToBorder => gl::CLAMP_TO_BORDER,
            WrapParameter::MirroredRepeat => gl::MIRRORED_REPEAT,
        }
    }
}

impl TextureParameter for WrapParameter {
    fn get(&self) -> u32 {
        self.get()
    }
}

