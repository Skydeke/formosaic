use crate::{
    engine::{
        architecture::scene::{entity::simple_entity::SimpleEntity, scene_context::SceneContext},
        rendering::{abstracted::irenderer::IRenderer, instances::entity_render::EntityRenderer},
    },
    opengl::{
        constants::{data_type::DataType, format_type::FormatType},
        fbos::{
            attachment::texture_attachment::TextureAttachment, fbo::Fbo, fbo_target::FboTarget,
            scene_fbo::SceneFbo,
        },
        shaders::lighting_pass::LightingPass,
        textures::{
            parameters::{
                mag_filter_parameter::MagFilterParameter, min_filter_parameter::MinFilterParameter,
            },
            texture_configs::TextureConfigs,
        },
    },
};
use cgmath::Vector2;
use std::{cell::RefCell, rc::Rc};

pub struct Pipeline {
    renderers: Vec<Box<dyn IRenderer>>,
    context: Rc<RefCell<SceneContext>>,
    deferred_fbo: Fbo,
    lighting_pass: LightingPass,
    scene_fbo: SceneFbo,
}

impl Pipeline {
    pub fn new(context: Rc<RefCell<SceneContext>>) -> Self {
        // Create deferred FBO with G-buffer attachments
        let deferred_fbo = Self::create_deferred_fbo(1, 1);
        let lighting_pass = LightingPass::new();

        // Create scene FBO
        let scene_fbo = SceneFbo::new(1, 1);

        let mut pipeline = Self {
            renderers: Vec::new(),
            context,
            deferred_fbo,
            lighting_pass,
            scene_fbo,
        };

        let simple_triangle_renderer: EntityRenderer<SimpleEntity> =
            EntityRenderer::new().expect("Can't create EntityRenderer.");
        pipeline.add_renderer(Box::new(simple_triangle_renderer));

        pipeline
    }

    fn create_deferred_fbo(width: i32, height: i32) -> Fbo {
        let mut pbr_fbo = Fbo::create(width, height);
        pbr_fbo.bind(FboTarget::Framebuffer);

        // Albedo buffer (RGBA16F)
        let mut albedo_configs =
            TextureConfigs::new(FormatType::Rgba16F, FormatType::Rgba, DataType::Float);
        albedo_configs.mag_filter = Some(MagFilterParameter::Nearest);
        albedo_configs.min_filter = Some(MinFilterParameter::Nearest);
        pbr_fbo.add_attachment(TextureAttachment::of_colour(0, albedo_configs));

        // Normal + Metalness buffer (RGBA32F)
        let mut normal_configs =
            TextureConfigs::new(FormatType::Rgba32F, FormatType::Rgba, DataType::Float);
        normal_configs.mag_filter = Some(MagFilterParameter::Linear);
        normal_configs.min_filter = Some(MinFilterParameter::Linear);
        pbr_fbo.add_attachment(TextureAttachment::of_colour(1, normal_configs));

        // Position + Roughness buffer (RGBA32F)
        let mut pos_configs =
            TextureConfigs::new(FormatType::Rgba32F, FormatType::Rgba, DataType::Float);
        pos_configs.mag_filter = Some(MagFilterParameter::Linear);
        pos_configs.min_filter = Some(MinFilterParameter::Linear);
        pbr_fbo.add_attachment(TextureAttachment::of_colour(2, pos_configs));

        // Depth texture attachment
        let mut depth_configs = TextureConfigs::new(
            FormatType::DepthComponent24,
            FormatType::DepthComponent,
            DataType::UInt,
        );
        depth_configs.mag_filter = Some(MagFilterParameter::Linear);
        depth_configs.min_filter = Some(MinFilterParameter::Linear);
        pbr_fbo.add_attachment(TextureAttachment::of_depth(depth_configs));

        pbr_fbo.unbind(FboTarget::Framebuffer);
        pbr_fbo
    }

    pub fn add_renderer(&mut self, renderer: Box<dyn IRenderer>) {
        self.renderers.push(renderer);
    }

    pub fn get_deferred_fbo(&self) -> &Fbo {
        &self.deferred_fbo
    }

    pub fn get_deferred_fbo_mut(&mut self) -> &mut Fbo {
        &mut self.deferred_fbo
    }

    pub fn draw(&mut self, width: u32, height: u32) {
        let resolution = Vector2::new(width, height);
        self.context.borrow_mut().set_resolution(resolution);

        // Resize scene FBO if resolution changed
        if !self.scene_fbo.fbo.is_sized(width as i32, height as i32) {
            self.scene_fbo.fbo.resize(width as i32, height as i32);
        }

        // Resize deferred FBO if resolution changed
        if !self.deferred_fbo.is_sized(width as i32, height as i32) {
            self.deferred_fbo.resize(width as i32, height as i32);
        }

        self.geometry_pass();
        self.lighting_pass();
        self.finish();
    }

    fn geometry_pass(&mut self) {
        self.deferred_fbo.bind(FboTarget::DrawFramebuffer);

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT);
        }

        self.context.borrow_mut().update();

        for renderer in &mut self.renderers {
            renderer.render(&self.context.borrow());
        }

        self.deferred_fbo.unbind(FboTarget::DrawFramebuffer);
    }

    fn lighting_pass(&mut self) {
        self.lighting_pass.execute(
            &self.deferred_fbo,
            &mut self.scene_fbo.fbo,
            &self.context.borrow(),
        );

        self.scene_fbo.blit_to_screen();
    }

    fn finish(&mut self) {
        for renderer in &mut self.renderers {
            renderer.finish();
        }
    }
}
