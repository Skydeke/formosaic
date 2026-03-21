use crate::{
    engine::{
        architecture::scene::{entity::simple_entity::SimpleEntity, scene_context::SceneContext},
        rendering::{
            abstracted::irenderer::{IRenderer, RenderPass},
            instances::{
                entity_render::EntityRenderer,
                hint_render::HintRenderer,
                imgui_render::ImguiGlRenderer,
                menu_render::MenuRenderer,
                outline_render::OutlineRenderer,
            },
        },
    },
    opengl::{
        constants::{data_type::DataType, format_type::FormatType},
        fbos::{
            attachment::texture_attachment::TextureAttachment,
            fbo::Fbo,
            fbo_target::FboTarget,
            scene_fbo::SceneFbo,
        },
        shaders::lighting_pass::LightingPass,
        textures::{
            parameters::{
                mag_filter_parameter::MagFilterParameter,
                min_filter_parameter::MinFilterParameter,
            },
            texture_configs::TextureConfigs,
        },
    },
};
use cgmath::Vector2;
use std::{cell::RefCell, rc::Rc};

/// All per-frame data any renderer might need.
/// Broadcast to every renderer via `IRenderer::prepare` before each draw.
/// Renderers ignore fields they don't use.
pub struct FrameData {
    pub warmth:          f32,
    pub warmth_color:    [f32; 3],
    pub hint_tier:       u8,
    pub solved:          bool,
    pub glow_intensity:  f32,
    pub time:            f32,
    pub delta_time:      f32,
    /// Framebuffer size in physical pixels — used by screen-space renderers.
    pub viewport_w:      f32,
    pub viewport_h:      f32,
    /// Whether the game is currently showing the menu screen.
    pub show_menu:       bool,
    /// Whether the platform is touch-only (Android).
    pub is_touch:        bool,
    /// Imgui draw data pointer — null when imgui has nothing to draw.
    /// Valid until the next `imgui::Context::render()` call.
    pub imgui_draw_data: *const imgui::DrawData,
}

impl Default for FrameData {
    fn default() -> Self {
        Self {
            warmth:          0.0,
            warmth_color:    [1.0, 1.0, 1.0],
            hint_tier:       0,
            solved:          false,
            glow_intensity:  0.0,
            time:            0.0,
            delta_time:      0.0,
            viewport_w:      1.0,
            viewport_h:      1.0,
            show_menu:       false,
            is_touch:        false,
            imgui_draw_data: std::ptr::null(),
        }
    }
}

// Backward-compat alias so game_engine.rs doesn't need changes.
pub type HintFrameData = FrameData;

pub struct Pipeline {
    renderers:     Vec<Box<dyn IRenderer>>,
    context:       Rc<RefCell<SceneContext>>,
    deferred_fbo:  Fbo,
    lighting_pass: LightingPass,
    scene_fbo:     SceneFbo,
}

impl Pipeline {
    pub fn new(context: Rc<RefCell<SceneContext>>) -> Self {
        let deferred_fbo  = Self::create_deferred_fbo(1, 1);
        let lighting_pass = LightingPass::new();
        let scene_fbo     = SceneFbo::new(1, 1);

        let mut pipeline = Self {
            renderers: Vec::new(),
            context,
            deferred_fbo,
            lighting_pass,
            scene_fbo,
        };

        pipeline.add_renderer(Box::new(
            EntityRenderer::<SimpleEntity>::new().expect("Can't create EntityRenderer."),
        ));

        match MenuRenderer::new() {
            Ok(r)  => pipeline.add_renderer(Box::new(r)),
            Err(e) => log::warn!("MenuRenderer failed to initialise: {e}"),
        }

        match OutlineRenderer::new() {
            Ok(r)  => pipeline.add_renderer(Box::new(r)),
            Err(e) => log::warn!("OutlineRenderer failed to initialise: {e}"),
        }

        match HintRenderer::new() {
            Ok(r)  => pipeline.add_renderer(Box::new(r)),
            Err(e) => log::warn!("HintRenderer failed to initialise: {e}"),
        }

        pipeline
    }

    /// Add any renderer. Its `pass()` determines when it runs.
    pub fn add_renderer(&mut self, r: Box<dyn IRenderer>) {
        self.renderers.push(r);
    }

    /// Register the imgui renderer. Called once from `GameEngine::init_gl`.
    pub fn add_imgui_renderer(&mut self, r: ImguiGlRenderer) {
        self.renderers.push(Box::new(r));
    }

    pub fn get_deferred_fbo(&self)         -> &Fbo     { &self.deferred_fbo }
    pub fn get_deferred_fbo_mut(&mut self) -> &mut Fbo { &mut self.deferred_fbo }

    pub fn draw(&mut self, width: u32, height: u32, frame: &FrameData) {
        self.context.borrow_mut().set_resolution(Vector2::new(width, height));

        if !self.scene_fbo.fbo.is_sized(width as i32, height as i32) {
            self.scene_fbo.fbo.resize(width as i32, height as i32);
        }
        if !self.deferred_fbo.is_sized(width as i32, height as i32) {
            self.deferred_fbo.resize(width as i32, height as i32);
        }

        // Broadcast per-frame data to every renderer before any rendering begins.
        for r in &mut self.renderers { r.prepare(frame); }

        self.geometry_pass();
        self.lighting_pass();
        self.late_pass();
        self.overlay_pass();
        self.finish_pass();
    }

    fn geometry_pass(&mut self) {
        self.deferred_fbo.bind(FboTarget::DrawFramebuffer);
        unsafe { gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT); }
        self.context.borrow_mut().update();
        let ctx = self.context.borrow();
        for r in &mut self.renderers {
            if r.pass() == RenderPass::Geometry { r.render(&ctx); }
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

    fn late_pass(&mut self) {
        // Default framebuffer is already bound after scene_fbo.blit_to_screen().
        let ctx = self.context.borrow();
        for r in &mut self.renderers {
            if r.pass() == RenderPass::Late { r.render(&ctx); }
        }
    }

    fn overlay_pass(&mut self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::UseProgram(0);
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthMask(gl::TRUE);
            gl::Enable(gl::CULL_FACE);
            gl::CullFace(gl::BACK);
            gl::Disable(gl::BLEND);
        }
        let ctx = self.context.borrow();
        for r in &mut self.renderers {
            if r.pass() == RenderPass::Overlay { r.render(&ctx); }
        }
    }

    fn finish_pass(&mut self) {
        for r in &mut self.renderers { r.finish(); }
    }

    fn create_deferred_fbo(w: i32, h: i32) -> Fbo {
        let mut fbo = Fbo::create(w, h);
        fbo.bind(FboTarget::Framebuffer);

        let mut albedo = TextureConfigs::new(FormatType::Rgba16F, FormatType::Rgba, DataType::Float);
        albedo.mag_filter = Some(MagFilterParameter::Nearest);
        albedo.min_filter = Some(MinFilterParameter::Nearest);
        fbo.add_attachment(TextureAttachment::of_colour(0, albedo));

        let mut normal = TextureConfigs::new(FormatType::Rgba32F, FormatType::Rgba, DataType::Float);
        normal.mag_filter = Some(MagFilterParameter::Linear);
        normal.min_filter = Some(MinFilterParameter::Linear);
        fbo.add_attachment(TextureAttachment::of_colour(1, normal));

        let mut pos = TextureConfigs::new(FormatType::Rgba32F, FormatType::Rgba, DataType::Float);
        pos.mag_filter = Some(MagFilterParameter::Linear);
        pos.min_filter = Some(MinFilterParameter::Linear);
        fbo.add_attachment(TextureAttachment::of_colour(2, pos));

        let mut depth = TextureConfigs::new(
            FormatType::DepthComponent24, FormatType::DepthComponent, DataType::UInt,
        );
        depth.mag_filter = Some(MagFilterParameter::Linear);
        depth.min_filter = Some(MinFilterParameter::Linear);
        fbo.add_attachment(TextureAttachment::of_depth(depth));

        fbo.unbind(FboTarget::Framebuffer);
        fbo
    }
}
