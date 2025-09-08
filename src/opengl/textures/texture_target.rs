use gl;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TextureTarget {
    Texture2D,
    TextureCubeMap,
    Texture2DMultisample,

    TextureCubeMapPositiveX,
    TextureCubeMapNegativeX,
    TextureCubeMapPositiveY,
    TextureCubeMapNegativeY,
    TextureCubeMapPositiveZ,
    TextureCubeMapNegativeZ,
}

impl TextureTarget {
    pub fn get(&self) -> u32 {
        match self {
            TextureTarget::Texture2D => gl::TEXTURE_2D,
            TextureTarget::TextureCubeMap => gl::TEXTURE_CUBE_MAP,
            TextureTarget::Texture2DMultisample => gl::TEXTURE_2D_MULTISAMPLE,
            TextureTarget::TextureCubeMapPositiveX => gl::TEXTURE_CUBE_MAP_POSITIVE_X,
            TextureTarget::TextureCubeMapNegativeX => gl::TEXTURE_CUBE_MAP_NEGATIVE_X,
            TextureTarget::TextureCubeMapPositiveY => gl::TEXTURE_CUBE_MAP_POSITIVE_Y,
            TextureTarget::TextureCubeMapNegativeY => gl::TEXTURE_CUBE_MAP_NEGATIVE_Y,
            TextureTarget::TextureCubeMapPositiveZ => gl::TEXTURE_CUBE_MAP_POSITIVE_Z,
            TextureTarget::TextureCubeMapNegativeZ => gl::TEXTURE_CUBE_MAP_NEGATIVE_Z,
        }
    }

    pub fn of_cube_face(face: u32) -> Result<Self, String> {
        match face {
            0 => Ok(TextureTarget::TextureCubeMapPositiveX),
            1 => Ok(TextureTarget::TextureCubeMapNegativeX),
            2 => Ok(TextureTarget::TextureCubeMapPositiveY),
            3 => Ok(TextureTarget::TextureCubeMapNegativeY),
            4 => Ok(TextureTarget::TextureCubeMapPositiveZ),
            5 => Ok(TextureTarget::TextureCubeMapNegativeZ),
            _ => Err(format!("Cubes do not have {} faces", face)),
        }
    }
}
