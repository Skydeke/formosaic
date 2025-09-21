use crate::{
    engine::{
        architecture::scene::{entity::simple_entity::SimpleEntity, scene_context::SceneContext},
        rendering::{abstracted::irenderer::IRenderer, instances::entity_render::EntityRenderer},
    },
    opengl::{
        constants::{data_type::DataType, format_type::FormatType, gl_buffer::GlBuffer},
        fbos::{
            attachment::texture_attachment::TextureAttachment, fbo::Fbo, fbo_target::FboTarget,
            scene_fbo::SceneFbo,
        },
        textures::{
            parameters::{
                mag_filter_parameter::MagFilterParameter, min_filter_parameter::MinFilterParameter,
            },
            texture_configs::TextureConfigs,
        },
    },
};
use cgmath::Vector2;
use std::{cell::RefCell, ffi::CStr, rc::Rc};

pub struct Pipeline {
    renderers: Vec<Box<dyn IRenderer>>,
    context: Rc<RefCell<SceneContext>>,
    deferred_fbo: Fbo,
}

impl Pipeline {
    pub fn new(context: Rc<RefCell<SceneContext>>) -> Self {
        // Create deferred FBO with G-buffer attachments
        let deferred_fbo = Self::create_deferred_fbo();

        let mut pipeline = Self {
            renderers: Vec::new(),
            context,
            deferred_fbo,
        };

        let simple_triangle_renderer: EntityRenderer<SimpleEntity> =
            EntityRenderer::new().expect("Cant create EntityRenderer.");
        pipeline.add_renderer(Box::new(simple_triangle_renderer));
        pipeline
    }

    fn create_deferred_fbo() -> Fbo {
        /*
         * G-Buffer layout:
         * layout (location = 0) out vec4 pos_vbo     - Position + Roughness
         * layout (location = 1) out vec4 norm_vbo    - Normal + Metalness
         * layout (location = 2) out vec4 albedo_vbo  - Albedo
         * + depth buffer
         *
         * __0__________8__________16_________24__________
         * |0| pos.x    | pos.y    | pos.z    | roughness|
         * |1| normal.r | normal.g | normal.b | metalness|
         * |2| albedo.rg| albedo.ba|          |          |
         * |_|__________|__________|__________|__________|
         */

        let mut pbr_fbo = Fbo::create(100, 100);
        pbr_fbo.bind(FboTarget::Framebuffer);

        // Position + Roughness buffer (RGBA32F)
        let mut pos_configs =
            TextureConfigs::new(FormatType::Rgba32F, FormatType::Rgba, DataType::Float);
        pos_configs.mag_filter = Some(MagFilterParameter::Linear);
        pos_configs.min_filter = Some(MinFilterParameter::Linear);
        pbr_fbo.add_attachment(TextureAttachment::of_colour(0, pos_configs));

        // Normal + Metalness buffer (RGBA32F)
        let mut normal_configs =
            TextureConfigs::new(FormatType::Rgba32F, FormatType::Rgba, DataType::Float);
        normal_configs.mag_filter = Some(MagFilterParameter::Linear);
        normal_configs.min_filter = Some(MinFilterParameter::Linear);
        pbr_fbo.add_attachment(TextureAttachment::of_colour(1, normal_configs));

        // Albedo buffer (RGBA16F)
        let mut albedo_configs =
            TextureConfigs::new(FormatType::Rgba16F, FormatType::Rgba, DataType::Float);
        albedo_configs.mag_filter = Some(MagFilterParameter::Linear);
        albedo_configs.min_filter = Some(MinFilterParameter::Linear);
        pbr_fbo.add_attachment(TextureAttachment::of_colour(2, albedo_configs));

        // Share color and depth attachments from SceneFbo
        let scene_fbo = SceneFbo::instance();
        if let Some(color_attachment) = scene_fbo.fbo.get_attachments().get(0) {
            pbr_fbo.add_attachment(color_attachment.clone_attachment());
        }
        if let Some(depth_attachment) = scene_fbo.fbo.get_depth_attachment() {
            pbr_fbo.add_attachment(depth_attachment.clone_attachment());
        }

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
        // Fix: Convert u32 to Vector2<u32> for set_resolution
        let resolution = Vector2::new(width, height);
        self.context.borrow_mut().set_resolution(resolution);

        // Resize Scene FBO if resolution changed
        let mut scene_fbo = &mut SceneFbo::instance().fbo;
        if !scene_fbo.is_sized(width as i32, height as i32) {
            scene_fbo.resize(width as i32, height as i32);
            log::info!("Resizing SceneFbo.");
        }
        // Resize deferred FBO if resolution changed
        if !self.deferred_fbo.is_sized(width as i32, height as i32) {
            self.deferred_fbo.resize(width as i32, height as i32);
            log::info!("Resizing deferred FBO.");
        }

        // Geometry pass - render to G-buffer
        self.geometry_pass();

        // Lighting pass - use G-buffer textures for lighting calculations
        self.lighting_pass();

        self.finish();
    }

    fn geometry_pass(&mut self) {
        // Bind deferred FBO for G-buffer rendering
        self.deferred_fbo.bind(FboTarget::Framebuffer);

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT);
        }

        self.context.borrow_mut().update();

        // Render geometry to G-buffer
        for renderer in &mut self.renderers {
            renderer.render(&self.context.borrow());
        }

        self.deferred_fbo.unbind(FboTarget::Framebuffer);
    }

    fn lighting_pass(&mut self) {
        // Here you would typically:
        // 1. Bind the scene FBO or screen framebuffer for final output
        // 2. Bind G-buffer textures as inputs to lighting shaders
        // 3. Perform lighting calculations using deferred shading
        // 4. Render final lit scene

        // Source: your deferred FBO
        self.deferred_fbo.bind(FboTarget::ReadFramebuffer);
        Fbo::blit_framebuffer(
            0,
            0,
            self.deferred_fbo.get_width(),
            self.deferred_fbo.get_height(),
            0,
            0,
            self.deferred_fbo.get_width(),
            self.deferred_fbo.get_height(),
            MagFilterParameter::Nearest,
            &[GlBuffer::Colour],
        );
        //SceneFbo::instance().blit_to_screen();
    }

    fn finish(&mut self) {
        for renderer in &mut self.renderers {
            renderer.finish();
        }
    }
}
