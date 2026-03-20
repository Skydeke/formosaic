use crate::{
    engine::{
        architecture::scene::{entity::simple_entity::SimpleEntity, scene_context::SceneContext},
        rendering::{
            abstracted::irenderer::IRenderer,
            instances::{
                disc_render::DiscRenderer,
                entity_render::EntityRenderer,
                hud_render::HudRenderer,
                menu_render::{MenuFrameData, MenuPanel, MenuRenderer},
                outline_render::OutlineRenderer,
            },
        },
    },
    level::storage::LevelMeta,
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

/// Per-frame hint/overlay data passed from game logic into the render pipeline.
#[derive(Clone)]
pub struct HintFrameData {
    pub warmth:        f32,
    pub warmth_color:  [f32; 3],
    pub hint_tier:     u8,
    pub show_disc:     bool,
    pub disc_normal:   cgmath::Vector3<f32>,
    pub disc_center:   cgmath::Vector3<f32>,
    pub disc_radius:   f32,
    pub solved:        bool,
    pub glow_intensity: f32,
    pub time:          f32,
    // Menu
    pub show_menu:      bool,
    pub menu_panel:     MenuPanel,
    pub levels:         Vec<LevelMeta>,
    pub selected_level: usize,
    pub is_downloading: bool,
    /// Show on-screen touch buttons during gameplay (for Android/touch)
    pub show_touch_btns: bool,
}

impl Default for HintFrameData {
    fn default() -> Self {
        Self {
            warmth: 0.0,
            warmth_color: [1.0, 1.0, 1.0],
            hint_tier: 0,
            show_disc: false,
            disc_normal: cgmath::Vector3::new(0.0, 1.0, 0.0),
            disc_center: cgmath::Vector3::new(0.0, 0.0, 0.0),
            disc_radius: 1.5,
            solved: false,
            glow_intensity: 0.0,
            time: 0.0,
            show_menu:     false,
            menu_panel:    MenuPanel::Levels,
            levels:        Vec::new(),
            selected_level: 0,
            is_downloading: false,
            show_touch_btns: false,
        }
    }
}

pub struct Pipeline {
    renderers:    Vec<Box<dyn IRenderer>>,
    context:      Rc<RefCell<SceneContext>>,
    deferred_fbo: Fbo,
    lighting_pass: LightingPass,
    scene_fbo:    SceneFbo,
    outline:      Option<OutlineRenderer>,
    disc:         Option<DiscRenderer>,
    hud:          Option<HudRenderer>,
    menu:         Option<MenuRenderer>,
}

impl Pipeline {
    pub fn new(context: Rc<RefCell<SceneContext>>) -> Self {
        let deferred_fbo  = Self::create_deferred_fbo(1, 1);
        let lighting_pass = LightingPass::new();
        let scene_fbo     = SceneFbo::new(1, 1);

        let outline = OutlineRenderer::new()
            .map_err(|e| log::warn!("OutlineRenderer: {}", e)).ok();
        let disc    = DiscRenderer::new()
            .map_err(|e| log::warn!("DiscRenderer: {}", e)).ok();
        let hud     = HudRenderer::new()
            .map_err(|e| log::warn!("HudRenderer: {}", e)).ok();
        let menu    = MenuRenderer::new()
            .map_err(|e| log::warn!("MenuRenderer: {}", e)).ok();

        let mut pipeline = Self {
            renderers: Vec::new(),
            context,
            deferred_fbo,
            lighting_pass,
            scene_fbo,
            outline,
            disc,
            hud,
            menu,
        };

        pipeline.add_renderer(Box::new(
            EntityRenderer::<SimpleEntity>::new().expect("Can't create EntityRenderer."),
        ));

        pipeline
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

    pub fn add_renderer(&mut self, r: Box<dyn IRenderer>) { self.renderers.push(r); }
    pub fn get_deferred_fbo(&self)         -> &Fbo     { &self.deferred_fbo }
    pub fn get_deferred_fbo_mut(&mut self) -> &mut Fbo { &mut self.deferred_fbo }

    /// Draw a frame.  `hint` carries all overlay / HUD state.
    pub fn draw(&mut self, width: u32, height: u32, hint: &HintFrameData) {
        self.context.borrow_mut().set_resolution(Vector2::new(width, height));

        if !self.scene_fbo.fbo.is_sized(width as i32, height as i32) {
            self.scene_fbo.fbo.resize(width as i32, height as i32);
        }
        if !self.deferred_fbo.is_sized(width as i32, height as i32) {
            self.deferred_fbo.resize(width as i32, height as i32);
        }

        self.geometry_pass();
        self.lighting_pass();
        self.overlay_pass(hint);
        self.finish_pass();
    }

    /// Legacy no-hint draw (keeps game_engine.rs compiling unchanged).
    pub fn draw_legacy(&mut self, width: u32, height: u32) {
        self.draw(width, height, &HintFrameData::default());
    }

    fn geometry_pass(&mut self) {
        self.deferred_fbo.bind(FboTarget::DrawFramebuffer);
        unsafe { gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT); }
        self.context.borrow_mut().update();
        for r in &mut self.renderers { r.render(&self.context.borrow()); }
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

    fn overlay_pass(&mut self, hint: &HintFrameData) {
        // ── Reset GL state after deferred compute + blit ─────────────────
        // Compute pass leaves image units active and ReadFramebuffer bound.
        // Reset to clean state before any forward overlay draw calls.
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::MemoryBarrier(gl::ALL_BARRIER_BITS);
            gl::UseProgram(0);
            // Unbind compute shader image units to prevent DrawArrays conflicts
            for unit in 0u32..4 {
                gl::BindImageTexture(unit, 0, 0, gl::FALSE, 0, gl::READ_ONLY, gl::RGBA8);
            }
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthMask(gl::TRUE);
            gl::Enable(gl::CULL_FACE);
            gl::CullFace(gl::BACK);
            gl::Disable(gl::BLEND);
        }

        // Outline glow
        if let Some(o) = &mut self.outline {
            o.update(0.016);
            o.set_active(hint.solved || hint.glow_intensity > 0.01);
            o.set_intensity(hint.glow_intensity);
            o.render(&self.context.borrow());
        }

        // Axis-plane disc (Tier 2)
        if let Some(d) = &mut self.disc {
            d.active      = hint.show_disc;
            d.disc_normal = hint.disc_normal;
            d.disc_center = hint.disc_center;
            d.disc_radius = hint.disc_radius;
            d.time        = hint.time;
            d.render(&self.context.borrow());
        }

        // HUD compass (Tier 1+)
        if let Some(h) = &mut self.hud {
            h.active       = hint.hint_tier >= 1;
            h.warmth       = hint.warmth;
            h.warmth_color = hint.warmth_color;
            h.hint_tier    = hint.hint_tier as f32;
            h.time         = hint.time;
            h.render(&self.context.borrow());
        }

        // In-game menu overlay
        if hint.show_menu {
            let res = self.context.borrow().get_camera().borrow().resolution;
            let (w, h_px) = (res.x as f32, res.y as f32);
            if let Some(menu) = &mut self.menu {
                menu.update(hint.time, w, h_px);
                let menu_data = MenuFrameData {
                    active:         true,
                    panel:          hint.menu_panel,
                    levels:         &hint.levels,
                    selected_level: hint.selected_level,
                    is_downloading: hint.is_downloading,
                    time:           hint.time,
                };
                menu.render_menu(&menu_data, w, h_px);
            }
        }

        // On-screen touch buttons (shown during gameplay on touch devices)
        if hint.show_touch_btns && !hint.show_menu {
            let res = self.context.borrow().get_camera().borrow().resolution;
            let (w, h_px) = (res.x as f32, res.y as f32);
            if let Some(menu) = &mut self.menu {
                menu.render_touch_buttons(w, h_px);
            }
        }
    }

    fn finish_pass(&mut self) {
        for r in &mut self.renderers { r.finish(); }
    }
}
