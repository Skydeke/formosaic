//! Generic render pipeline — orchestrates geometry → lighting → late → overlay
//! passes each frame, then imgui on top.
//!
//! Pass order and registration:
//!   `add_renderer()`       — geometry, late, or overlay renderers (sorted by pass)
//!   `add_imgui_renderer()` — the Dear ImGui renderer; always runs last, after overlay

use crate::{
    architecture::scene::{entity::simple_entity::SimpleEntity, scene_context::SceneContext},
    rendering::{
        abstracted::irenderer::{IRenderer, RenderPass},
        instances::{
            entity_render::EntityRenderer,
            imgui_render::ImguiGlRenderer,
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

pub struct Pipeline {
    /// General renderers — geometry, late, overlay — sorted by pass internally.
    renderers:     Vec<Box<dyn IRenderer>>,
    /// Dedicated imgui renderer — always runs after all other passes.
    /// Stored separately so it can be accessed directly without any downcast.
    imgui:         Option<ImguiGlRenderer>,
    context:       Rc<RefCell<SceneContext>>,
    deferred_fbo:  Fbo,
    lighting_pass: LightingPass,
    scene_fbo:     SceneFbo,
}

impl Pipeline {
    pub fn new(context: Rc<RefCell<SceneContext>>) -> Self {
        let mut pipeline = Self {
            renderers:     Vec::new(),
            imgui:         None,
            context,
            deferred_fbo:  Self::create_deferred_fbo(1, 1),
            lighting_pass: LightingPass::new(),
            scene_fbo:     SceneFbo::new(1, 1),
        };
        pipeline.add_renderer(Box::new(
            EntityRenderer::<SimpleEntity>::new().expect("Can't create EntityRenderer."),
        ));
        pipeline
    }

    /// Register a geometry / late / overlay renderer.
    pub fn add_renderer(&mut self, r: Box<dyn IRenderer>) {
        self.renderers.push(r);
    }

    /// Register the Dear ImGui renderer — runs after all other passes every frame.
    /// Only one imgui renderer is supported; a second call replaces the first.
    pub fn add_imgui_renderer(&mut self, r: ImguiGlRenderer) {
        self.imgui = Some(r);
    }

    /// Direct access to the imgui renderer (no downcast needed).
    pub fn imgui_renderer(&self) -> Option<&ImguiGlRenderer> {
        self.imgui.as_ref()
    }

    /// Direct mutable access to the imgui renderer.
    pub fn imgui_renderer_mut(&mut self) -> Option<&mut ImguiGlRenderer> {
        self.imgui.as_mut()
    }

    /// Draw one frame. `SceneContext` must already reflect current game state.
    pub fn draw(&mut self, width: u32, height: u32) {
        self.context.borrow_mut().set_resolution(Vector2::new(width, height));

        if !self.scene_fbo.fbo.is_sized(width as i32, height as i32) {
            self.scene_fbo.fbo.resize(width as i32, height as i32);
        }
        if !self.deferred_fbo.is_sized(width as i32, height as i32) {
            self.deferred_fbo.resize(width as i32, height as i32);
        }

        {
            use crate::rendering::render_output_data::RenderOutputData;
            use crate::opengl::fbos::simple_texture::SimpleTexture;
            let a = self.deferred_fbo.get_attachments();
            if a.len() >= 3 {
                let colour_id   = a[0].get_texture().get_id();
                let normal_id   = a[1].get_texture().get_id();
                let position_id = a[2].get_texture().get_id();
                let depth_id    = self.deferred_fbo
                    .get_depth_attachment()
                    .map(|d| d.get_texture().get_id())
                    .unwrap_or(0);
                self.context.borrow_mut().set_output_data(RenderOutputData::new(
                    Box::new(SimpleTexture::new(colour_id)),
                    Box::new(SimpleTexture::new(normal_id)),
                    Box::new(SimpleTexture::new(depth_id)),
                    Box::new(SimpleTexture::new(position_id)),
                ));
            }
        }

        unsafe { while gl::GetError() != gl::NO_ERROR {} }

        let clear_color = self.context.borrow().lights.clear_color;
        self.geometry_pass(clear_color);
        self.lighting_pass();
        self.late_pass();
        self.overlay_pass();
        self.imgui_pass();
        self.finish_pass();
    }

    fn geometry_pass(&mut self, clear_color: [f32; 3]) {
        self.deferred_fbo.bind(FboTarget::DrawFramebuffer);
        let [r, g, b] = clear_color;
        unsafe {
            gl::ClearColor(0.0, 0.0, 0.0, 0.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT);
            gl::ClearColor(r, g, b, 1.0);
        }
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

    /// Imgui pass — always last, after overlay.
    fn imgui_pass(&mut self) {
        let ctx = self.context.borrow();
        if let Some(r) = &mut self.imgui {
            r.render(&ctx);
        }
    }

    fn finish_pass(&mut self) {
        for r in &mut self.renderers { r.finish(); }
        if let Some(r) = &mut self.imgui { r.finish(); }
    }

    fn create_deferred_fbo(w: i32, h: i32) -> Fbo {
        let mut fbo = Fbo::create(w, h);
        fbo.bind(FboTarget::Framebuffer);

        let mut albedo = TextureConfigs::new(FormatType::Rgba16F, FormatType::Rgba, DataType::Float);
        albedo.mag_filter = Some(MagFilterParameter::Nearest);
        albedo.min_filter = Some(MinFilterParameter::Nearest);
        fbo.add_attachment(TextureAttachment::of_colour(0, albedo));

        let mut normal = TextureConfigs::new(FormatType::Rgba16F, FormatType::Rgba, DataType::Float);
        normal.mag_filter = Some(MagFilterParameter::Linear);
        normal.min_filter = Some(MinFilterParameter::Linear);
        fbo.add_attachment(TextureAttachment::of_colour(1, normal));

        let mut pos = TextureConfigs::new(FormatType::Rgba16F, FormatType::Rgba, DataType::Float);
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
