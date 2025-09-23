use crate::opengl::{
    constants::{data_type::DataType, format_type::FormatType},
    textures::{texture_configs::TextureConfigs, texture_target::TextureTarget},
};
use std::any::Any;

pub trait Texture: Any {
    fn bind_to_unit(&self, unit: u32);
    fn bind(&self);
    fn unbind(&self);
    fn delete(&mut self);
    fn get_id(&self) -> u32;

    fn allocate_if_needed(&mut self, width: i32, height: i32, configs: &TextureConfigs);

    fn attach_to_fbo(&self, attachment_point: i32, level: i32);
    fn apply_configs(&self, configs: &TextureConfigs);
    fn clone_texture(&self) -> Box<dyn Texture>;

    // Enable downcasting
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
