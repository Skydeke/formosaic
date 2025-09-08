use cgmath::Vector4;

use crate::opengl::{
    constants::{data_type::DataType, format_type::FormatType},
    textures::parameters::{
        mag_filter_parameter::MagFilterParameter, min_filter_parameter::MinFilterParameter,
        wrap_parameter::WrapParameter,
    },
};

pub struct TextureConfigs {
    pub internal_format: FormatType,
    pub format: FormatType,
    pub data_type: DataType,

    pub border_colour: Vector4<f32>,
    pub min_filter: Option<MinFilterParameter>,
    pub mag_filter: Option<MagFilterParameter>,
    pub wrap_s: Option<WrapParameter>,
    pub wrap_t: Option<WrapParameter>,
    pub level_of_detail_bias: f32,
    pub anisotropic_filter: f32,
    pub mipmap: bool,
}

impl TextureConfigs {
    pub fn new(internal_format: FormatType, format: FormatType, data_type: DataType) -> Self {
        Self {
            internal_format,
            format,
            data_type,
            border_colour: Vector4::new(0.0, 0.0, 0.0, 0.0),
            min_filter: None,
            mag_filter: None,
            wrap_s: None,
            wrap_t: None,
            level_of_detail_bias: 0.0,
            anisotropic_filter: 0.0,
            mipmap: true,
        }
    }

    pub fn default() -> Self {
        Self::new(FormatType::Rgba16F, FormatType::Rgba, DataType::UByte)
    }

    pub fn copy(&self) -> Self {
        Self {
            internal_format: self.internal_format,
            format: self.format,
            data_type: self.data_type,
            border_colour: self.border_colour.clone(),
            min_filter: self.min_filter,
            mag_filter: self.mag_filter,
            wrap_s: self.wrap_s,
            wrap_t: self.wrap_t,
            level_of_detail_bias: self.level_of_detail_bias,
            anisotropic_filter: self.anisotropic_filter,
            mipmap: self.mipmap,
        }
    }
}
