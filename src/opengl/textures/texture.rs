use crate::opengl::{
    constants::{data_type::DataType, format_type::FormatType},
    textures::{texture_configs::TextureConfigs, texture_target::TextureTarget},
};

// Remove Clone from Texture trait to make it dyn-compatible
pub trait Texture {
    fn bind_to_unit(&self, unit: u32);
    fn bind(&self);
    fn unbind(&self);
    fn delete(&mut self);
    fn get_id(&self) -> u32;

    // Additional methods needed by FBO system
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
    );
    fn attach_to_fbo(&self, attachment_point: i32, level: i32);
    fn apply_configs(&self, configs: &TextureConfigs);

    // For cloning textures, we create a new texture with same properties
    fn clone_texture(&self) -> Box<dyn Texture>;
}
