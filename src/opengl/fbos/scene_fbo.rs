use gl;

use crate::{
    // Fix: Remove app_context import if it doesn't exist
    engine::rendering::pipeline::Pipeline,
    opengl::{
        constants::{data_type::DataType, format_type::FormatType, gl_buffer::GlBuffer},
        fbos::{
            attachment::texture_attachment::TextureAttachment, fbo::Fbo, fbo_target::FboTarget,
        },
        textures::{
            parameters::{
                mag_filter_parameter::MagFilterParameter, min_filter_parameter::MinFilterParameter,
                wrap_parameter::WrapParameter,
            },
            texture_configs::TextureConfigs,
        },
    },
};

pub struct SceneFbo {
    pub fbo: Fbo, // Make public for access
}

// Singleton instance
static mut INSTANCE: Option<SceneFbo> = None;

impl SceneFbo {
    pub fn instance() -> &'static mut SceneFbo {
        unsafe {
            if INSTANCE.is_none() {
                let mut scene_fbo = SceneFbo {
                    fbo: Fbo::create(100, 100),
                };

                scene_fbo.fbo.bind(FboTarget::Framebuffer);

                // Color texture attachment
                let mut scene_configs =
                    TextureConfigs::new(FormatType::Rgba16F, FormatType::Rgba, DataType::Float);
                scene_configs.mag_filter = Some(MagFilterParameter::Linear);
                scene_configs.min_filter = Some(MinFilterParameter::Linear);
                scene_fbo
                    .fbo
                    .add_attachment(TextureAttachment::of_colour(0, scene_configs));

                // Depth texture attachment
                let mut depth_configs = TextureConfigs::new(
                    FormatType::DepthComponent24,
                    FormatType::DepthComponent,
                    DataType::UInt,
                );
                depth_configs.mag_filter = Some(MagFilterParameter::Linear);
                depth_configs.min_filter = Some(MinFilterParameter::Linear);
                scene_fbo
                    .fbo
                    .add_attachment(TextureAttachment::of_depth(depth_configs));

                scene_fbo.fbo.unbind(FboTarget::Framebuffer);
                INSTANCE = Some(scene_fbo);
            }
            INSTANCE.as_mut().unwrap()
        }
    }

    pub fn blit_to_screen(&mut self) {
        self.fbo.bind(FboTarget::ReadFramebuffer);

        Fbo::blit_framebuffer(
            0,
            0,
            self.fbo.get_width(),
            self.fbo.get_height(),
            0,
            0,
            self.fbo.get_width(),
            self.fbo.get_height(),
            MagFilterParameter::Nearest,
            &[GlBuffer::Colour],
        );
    }
}
