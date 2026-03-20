//! Game engine — winit/glutin application handler with Dear ImGui for UI.
//!
//! # UI Design
//! - **PC / Desktop**: Full imgui window menus (keyboard + mouse)
//! - **Android / Touch**: Same imgui menus but with large touch-friendly buttons
//!   and on-screen game controls (HINT / AGAIN / MENU) rendered as overlay buttons
//!   via the existing MenuRenderer::render_touch_buttons()

use winit::application::ApplicationHandler;
use winit::event_loop::ActiveEventLoop;

use glutin::context::PossiblyCurrentContext;
use glutin::{
    config::ConfigTemplateBuilder,
    context::{ContextApi, ContextAttributesBuilder, NotCurrentContext},
    display::{Display, GetGlDisplay},
    prelude::*,
    surface::{Surface, SurfaceAttributesBuilder, SwapInterval, WindowSurface},
};
use glutin_winit::DisplayBuilder;
use winit::keyboard::{Key, NamedKey};

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::{ffi::CStr, num::NonZeroU32};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::raw_window_handle::HasWindowHandle;
use winit::window::{Window, WindowAttributes, WindowId};

use crate::engine::architecture::models::model_cache::ModelCache;
use crate::engine::architecture::scene::scene_context::SceneContext;
use crate::engine::rendering::instances::menu_render::{MenuPanel, MenuRenderer};
use crate::engine::rendering::pipeline::{HintFrameData, Pipeline};
use crate::formosaic::Application;
use crate::imgui_renderer::ImguiGlRenderer;
use crate::input::Key as EngineKey;
use crate::{input::Event as EngineEvent, Formosaic};

pub struct GameEngine {
    game: Option<Formosaic>,
    pipeline: Option<Pipeline>,
    scene_context: Option<Rc<RefCell<SceneContext>>>,
    gl_display: Option<Display>,
    gl_context: Option<PossiblyCurrentContext>,
    gl_surface: Option<Surface<WindowSurface>>,
    window: Option<Arc<Window>>,
    gl_initialized: bool,
    last_cursor_pos: (f32, f32),
    last_frame_time: std::time::Instant,
    fps_accumulator: f32,
    frame_count: u32,
    // imgui
    imgui_ctx: Option<imgui::Context>,
    imgui_platform: Option<imgui_winit_support::WinitPlatform>,
    imgui_renderer: Option<ImguiGlRenderer>,
    // Touch button renderer (Android)
    touch_renderer: Option<MenuRenderer>,
}

impl ApplicationHandler for GameEngine {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("App resumed, initializing OpenGL...");
        if !self.gl_initialized {
            if let Err(e) = self.init_gl(event_loop) {
                log::error!("Failed to initialize OpenGL: {}", e);
                event_loop.exit();
                return;
            }
            self.gl_initialized = true;
            log::info!("OpenGL initialized successfully");
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        log::info!("App suspended, cleaning up OpenGL...");
        self.cleanup_gl();
        self.gl_initialized = false;
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // Feed event to imgui platform
        if let (Some(platform), Some(ctx), Some(window)) =
            (&mut self.imgui_platform, &mut self.imgui_ctx, &self.window)
        {
            // imgui-winit-support 0.14 expects winit::event::Event<T>, not WindowEvent
            let wrapped: winit::event::Event<imgui::Key> = winit::event::Event::WindowEvent {
                window_id: _window_id,
                event: event.clone(),
            };
            platform.handle_event(ctx.io_mut(), window, &wrapped);
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                if let (Some(gl_surface), Some(gl_context)) = (&self.gl_surface, &self.gl_context) {
                    gl_surface.resize(
                        gl_context,
                        NonZeroU32::new(size.width.max(1)).unwrap(),
                        NonZeroU32::new(size.height.max(1)).unwrap(),
                    );
                    unsafe {
                        gl::Viewport(0, 0, size.width as i32, size.height as i32);
                    }
                }
            }

            WindowEvent::RedrawRequested => {
                if let (Some(game), Some(gl_surface), Some(gl_context), Some(window)) = (
                    &mut self.game,
                    &self.gl_surface,
                    &self.gl_context,
                    &self.window,
                ) {
                    let now = std::time::Instant::now();
                    let delta_time = (now - self.last_frame_time).as_secs_f32();
                    self.last_frame_time = now;

                    self.fps_accumulator += delta_time;
                    self.frame_count += 1;
                    if self.fps_accumulator >= 20.0 {
                        let fps = self.frame_count as f32 / self.fps_accumulator;
                        log::info!("Formosaic - FPS: {:.1}", fps);
                        self.fps_accumulator = 0.0;
                        self.frame_count = 0;
                    }

                    let size = window.inner_size();

                    // ── Game update ───────────────────────────────────────
                    game.on_update(
                        delta_time,
                        &mut self
                            .scene_context
                            .clone()
                            .expect("No scene context.")
                            .borrow_mut(),
                    );

                    // ── Build hint frame data ─────────────────────────────
                    let glow = if game.is_solved() {
                        (game.elapsed_secs() * 0.5).min(1.0)
                    } else {
                        0.0
                    };

                    let show_menu = game.is_in_menu();
                    let menu_panel = game.menu_panel();
                    let levels = game.saved_levels().to_vec();
                    let sel = game.selected_level_idx();
                    let is_touch = cfg!(target_os = "android");

                    let hint = if let Some(o) = game.last_hint_output() {
                        HintFrameData {
                            warmth: o.warmth,
                            warmth_color: o.warmth_color,
                            hint_tier: o.tier.as_u8(),
                            show_disc: o.show_disc,
                            disc_normal: o.disc_normal,
                            disc_center: cgmath::Vector3::new(0.0_f32, 0.0, 0.0),
                            disc_radius: 1.5,
                            solved: game.is_solved(),
                            glow_intensity: glow,
                            time: game.elapsed_secs(),
                            show_menu: false, // imgui handles menu now
                            menu_panel: MenuPanel::Levels,
                            levels: vec![],
                            selected_level: 0,
                            is_downloading: false,
                            show_touch_btns: false, // rendered separately below
                        }
                    } else {
                        HintFrameData {
                            solved: game.is_solved(),
                            glow_intensity: glow,
                            time: game.elapsed_secs(),
                            ..Default::default()
                        }
                    };

                    // ── 3D render pass ────────────────────────────────────
                    self.pipeline
                        .as_mut()
                        .unwrap()
                        .draw(size.width, size.height, &hint);

                    // ── imgui UI ──────────────────────────────────────────
                    if let (Some(platform), Some(ctx), Some(renderer)) = (
                        &mut self.imgui_platform,
                        &mut self.imgui_ctx,
                        &self.imgui_renderer,
                    ) {
                        platform
                            .prepare_frame(ctx.io_mut(), window)
                            .expect("imgui frame prep failed");
                        let ui = ctx.frame();

                        Self::build_ui(
                            ui,
                            game,
                            size.width as f32,
                            size.height as f32,
                            &mut self
                                .scene_context
                                .clone()
                                .expect("No scene context.")
                                .borrow_mut(),
                            show_menu,
                            is_touch,
                        );

                        platform.prepare_render(ui, window);
                        let draw_data = ctx.render();

                        unsafe {
                            gl::Enable(gl::BLEND);
                            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
                        }
                        renderer.render(draw_data);
                        unsafe {
                            gl::Disable(gl::BLEND);
                            gl::Enable(gl::DEPTH_TEST);
                        }
                    }

                    // ── Android touch buttons ─────────────────────────────
                    if is_touch && !show_menu {
                        if let Some(tr) = &mut self.touch_renderer {
                            tr.render_touch_buttons(size.width as f32, size.height as f32);
                        }
                    }

                    let err = unsafe { gl::GetError() };
                    if err != gl::NO_ERROR {
                        log::error!(
                            "OpenGL error: 0x{:X} -- {}",
                            err,
                            Self::gl_error_to_string(err)
                        );
                    }
                    let _ = gl_surface.swap_buffers(gl_context);
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if let (Some(game), Some(window)) = (&mut self.game, &self.window) {
                    // Don't pass mouse to game when imgui is using it
                    let imgui_wants_mouse = self
                        .imgui_ctx
                        .as_ref()
                        .map(|c| c.io().want_capture_mouse)
                        .unwrap_or(false);
                    if !imgui_wants_mouse {
                        let size = window.inner_size();
                        match (state, button) {
                            (ElementState::Pressed, MouseButton::Left) => {
                                game.on_event(
                                    &EngineEvent::MouseDown {
                                        x: self.last_cursor_pos.0,
                                        y: self.last_cursor_pos.1,
                                        width: size.width as f32,
                                        height: size.height as f32,
                                    },
                                    &mut self
                                        .scene_context
                                        .clone()
                                        .expect("No scene context.")
                                        .borrow_mut(),
                                );
                            }
                            (ElementState::Released, MouseButton::Left) => {
                                game.on_event(
                                    &EngineEvent::MouseUp {
                                        x: self.last_cursor_pos.0,
                                        y: self.last_cursor_pos.1,
                                        width: size.width as f32,
                                        height: size.height as f32,
                                    },
                                    &mut self
                                        .scene_context
                                        .clone()
                                        .expect("No scene context.")
                                        .borrow_mut(),
                                );
                            }
                            _ => {}
                        }
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.last_cursor_pos = (position.x as f32, position.y as f32);
                if let (Some(game), Some(window)) = (&mut self.game, &self.window) {
                    let imgui_wants = self
                        .imgui_ctx
                        .as_ref()
                        .map(|c| c.io().want_capture_mouse)
                        .unwrap_or(false);
                    if !imgui_wants {
                        let size = window.inner_size();
                        game.on_event(
                            &EngineEvent::MouseMove {
                                x: self.last_cursor_pos.0,
                                y: self.last_cursor_pos.1,
                                width: size.width as f32,
                                height: size.height as f32,
                            },
                            &mut self
                                .scene_context
                                .clone()
                                .expect("No scene context.")
                                .borrow_mut(),
                        );
                    }
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(game) = &mut self.game {
                    let imgui_wants = self
                        .imgui_ctx
                        .as_ref()
                        .map(|c| c.io().want_capture_keyboard)
                        .unwrap_or(false);
                    if !imgui_wants {
                        let key = match event.logical_key {
                            Key::Named(NamedKey::Escape) => EngineKey::Escape,
                            Key::Character(ref s) if s.as_str() == "h" => EngineKey::H,
                            Key::Character(ref s) if s.as_str() == "r" => EngineKey::R,
                            Key::Character(ref s) if s.as_str() == "n" => EngineKey::N,
                            Key::Character(ref s) if s.as_str() == "k" => EngineKey::K,
                            Key::Character(ref s) if s.as_str() == "l" => EngineKey::L,
                            Key::Named(NamedKey::Space) => EngineKey::Space,
                            _ => EngineKey::Other,
                        };
                        if event.state == ElementState::Pressed {
                            game.on_event(
                                &EngineEvent::KeyDown { key },
                                &mut self
                                    .scene_context
                                    .clone()
                                    .expect("No scene context.")
                                    .borrow_mut(),
                            );
                        }
                    }
                }
            }

            WindowEvent::Touch(touch) => {
                if let (Some(game), Some(window)) = (&mut self.game, &self.window) {
                    let size = window.inner_size();
                    let tx = touch.location.x as f32;
                    let ty = touch.location.y as f32;
                    let tw = size.width as f32;
                    let th = size.height as f32;

                    match touch.phase {
                        winit::event::TouchPhase::Started => {
                            // Check touch buttons first
                            let key = MenuRenderer::hit_test_touch_buttons(tx, ty, tw, th);
                            if let Some(k) = key {
                                let ek = match k {
                                    "Escape" => EngineKey::Escape,
                                    "h" => EngineKey::H,
                                    "l" => EngineKey::L,
                                    _ => EngineKey::Other,
                                };
                                game.on_event(
                                    &EngineEvent::KeyDown { key: ek },
                                    &mut self
                                        .scene_context
                                        .clone()
                                        .expect("No scene context.")
                                        .borrow_mut(),
                                );
                            } else {
                                game.on_event(
                                    &EngineEvent::TouchDown {
                                        id: touch.id,
                                        x: tx,
                                        y: ty,
                                        width: tw,
                                        height: th,
                                    },
                                    &mut self
                                        .scene_context
                                        .clone()
                                        .expect("No scene context.")
                                        .borrow_mut(),
                                );
                            }
                        }
                        winit::event::TouchPhase::Moved => {
                            game.on_event(
                                &EngineEvent::TouchMove {
                                    id: touch.id,
                                    x: tx,
                                    y: ty,
                                    width: tw,
                                    height: th,
                                },
                                &mut self
                                    .scene_context
                                    .clone()
                                    .expect("No scene context.")
                                    .borrow_mut(),
                            );
                        }
                        winit::event::TouchPhase::Ended | winit::event::TouchPhase::Cancelled => {
                            game.on_event(
                                &EngineEvent::TouchUp { id: touch.id },
                                &mut self
                                    .scene_context
                                    .clone()
                                    .expect("No scene context.")
                                    .borrow_mut(),
                            );
                        }
                    }
                }
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl GameEngine {
    pub fn new() -> Self {
        Self {
            game: None,
            pipeline: None,
            scene_context: None,
            gl_display: None,
            gl_context: None,
            gl_surface: None,
            window: None,
            gl_initialized: false,
            last_cursor_pos: (0.0, 0.0),
            last_frame_time: std::time::Instant::now(),
            fps_accumulator: 0.0,
            frame_count: 0,
            imgui_ctx: None,
            imgui_platform: None,
            imgui_renderer: None,
            touch_renderer: None,
        }
    }

    // ── Dear ImGui UI ─────────────────────────────────────────────────────
    fn build_ui(
        ui: &imgui::Ui,
        game: &mut Formosaic,
        w: f32,
        h: f32,
        ctx: &mut SceneContext,
        show_menu: bool,
        is_touch: bool,
    ) {
        use imgui::*;

        // ── In-game HUD (always visible, top-right corner) ────────────────
        let hud_flags = WindowFlags::NO_DECORATION
            | WindowFlags::NO_MOVE
            | WindowFlags::NO_BACKGROUND
            | WindowFlags::NO_INPUTS
            | WindowFlags::NO_SAVED_SETTINGS;

        ui.window("##hud")
            .flags(hud_flags)
            .position([w - 220.0, 10.0], Condition::Always)
            .size([210.0, 100.0], Condition::Always)
            .build(|| {
                if let Some(r) = game.entropy_report() {
                    use crate::puzzle::entropy::difficulty_label;
                    ui.text_colored(
                        [0.8, 0.85, 0.95, 0.9],
                        format!(
                            "{:.1}s  |  {}",
                            game.elapsed_secs(),
                            difficulty_label(r.difficulty)
                        ),
                    );
                    let hints = game.hint_system().hint_count();
                    if hints > 0 {
                        ui.text_colored([0.9, 0.6, 0.2, 0.8], format!("{} hint(s)", hints));
                    }
                }
                if game.is_solved() {
                    ui.text_colored([0.2, 0.9, 0.5, 1.0], "SOLVED!");
                }
                if game.is_downloading() {
                    ui.text_colored([0.4, 0.7, 1.0, 0.8], "Fetching...");
                }
            });

        // ── Hint tier indicator ───────────────────────────────────────────
        if game.hint_system().hint_count() > 0
            || game
                .last_hint_output()
                .map(|o| o.warmth > 0.6)
                .unwrap_or(false)
        {
            ui.window("##hints")
                .flags(hud_flags)
                .position([10.0, h - 80.0], Condition::Always)
                .size([200.0, 70.0], Condition::Always)
                .build(|| {
                    let warmth = game.last_hint_output().map(|o| o.warmth).unwrap_or(0.0);
                    let cold = [0.2_f32, 0.4, 0.9, 1.0];
                    let hot = [0.9_f32, 0.2, 0.1, 1.0];
                    let col = [
                        cold[0] + (hot[0] - cold[0]) * warmth,
                        cold[1] + (hot[1] - cold[1]) * warmth,
                        cold[2] + (hot[2] - cold[2]) * warmth,
                        1.0,
                    ];
                    let label = if warmth > 0.8 {
                        "HOT"
                    } else if warmth > 0.5 {
                        "WARM"
                    } else {
                        "COLD"
                    };
                    ui.text_colored(col, format!(">> {} <<", label));
                    let tier = game.hint_system().tier();
                    ui.text_colored([0.5, 0.5, 0.5, 0.7], format!("Hint: {:?}", tier));
                });
        }

        if !show_menu {
            return;
        }

        // ── Level select menu (full overlay) ─────────────────────────────
        let menu_flags =
            WindowFlags::NO_DECORATION | WindowFlags::NO_MOVE | WindowFlags::NO_SAVED_SETTINGS;

        let btn_size = if is_touch {
            [180.0_f32, 56.0]
        } else {
            [140.0_f32, 36.0]
        };

        let menu_w = w.min(900.0);
        let menu_h = h.min(600.0);
        let menu_x = (w - menu_w) * 0.5;
        let menu_y = (h - menu_h) * 0.5;

        // Dark backdrop
        let draw = ui.get_background_draw_list();
        draw.add_rect([0.0, 0.0], [w, h], [0.02, 0.03, 0.05, 0.94_f32])
            .filled(true)
            .build();

        ui.window("FORMOSAIC")
            .flags(menu_flags)
            .position([menu_x, menu_y], Condition::Always)
            .size([menu_w, menu_h], Condition::Always)
            .build(|| {
                // ── Title + subtitle ──────────────────────────────────────
                ui.text_colored([0.8, 0.85, 0.95, 1.0], "FORMOSAIC");
                ui.same_line();
                ui.text_colored([0.35, 0.38, 0.50, 0.7], "  puzzle game  v0.1");
                ui.separator();
                ui.spacing();

                // Two-column layout via child windows
                let sidebar_w = menu_w * 0.28 - 10.0;

                // ── Sidebar child window ──────────────────────────────────
                ui.child_window("##sidebar")
                    .size([sidebar_w, menu_h - 100.0])
                    .border(false)
                    .build(|| {
                        ui.spacing();
                        ui.spacing();
                        if ui.button_with_size("[L] Levels", btn_size) {}
                        ui.spacing();
                        if ui.button_with_size("[N] Fetch Online", btn_size) {
                            game.on_event(&EngineEvent::KeyDown { key: EngineKey::N }, ctx);
                        }
                        ui.spacing();
                        if ui.button_with_size("[R] Random Saved", btn_size) {
                            game.on_event(&EngineEvent::KeyDown { key: EngineKey::R }, ctx);
                        }
                        ui.spacing();
                        ui.separator();
                        ui.spacing();
                        if game.is_downloading() {
                            ui.text_colored([0.4, 0.7, 1.0, 1.0], "Fetching...");
                        }
                    });

                ui.same_line();
                // Collect levels before the closure to avoid borrow conflict with game.on_event
                let levels: Vec<_> = game.saved_levels().to_vec();

                ui.child_window("##content")
                    .size([menu_w - sidebar_w - 30.0, menu_h - 100.0])
                    .border(false)
                    .build(|| {
                        ui.spacing();

                        if levels.is_empty() {
                            ui.spacing();
                            ui.spacing();
                            ui.text_colored([0.35, 0.38, 0.50, 1.0], "No saved levels yet.");
                            ui.spacing();
                            ui.text_colored(
                                [0.35, 0.38, 0.50, 0.8],
                                "Press [N] to fetch a random model",
                            );
                            ui.text_colored([0.35, 0.38, 0.50, 0.8], "from Poly Pizza.");
                        } else {
                            let avail_w = ui.content_region_avail()[0];
                            let card_w = ((avail_w - 20.0) / 3.0).max(100.0);
                            let card_h = 90.0_f32;
                            let mut col = 0usize;

                            for level in &levels {
                                let diff_col = match level.difficulty {
                                    d if d < 0.25 => [0.18, 0.75, 0.50, 1.0_f32],
                                    d if d < 0.50 => [0.75, 0.63, 0.18, 1.0],
                                    d if d < 0.75 => [0.75, 0.19, 0.29, 1.0],
                                    _ => [0.63, 0.18, 0.75, 1.0],
                                };
                                let diff_str = match level.difficulty {
                                    d if d < 0.25 => "Easy",
                                    d if d < 0.50 => "Medium",
                                    d if d < 0.75 => "Hard",
                                    _ => "Expert",
                                };

                                let id = format!("##card_{}", level.id);
                                ui.child_window(id)
                                    .size([card_w, card_h])
                                    .border(true)
                                    .build(|| {
                                        ui.text_colored(
                                            [0.82, 0.85, 0.94, 1.0],
                                            if level.name.len() > 14 {
                                                format!("{:.14}..", level.name)
                                            } else {
                                                level.name.clone()
                                            },
                                        );
                                        ui.text_colored(
                                            [0.35, 0.38, 0.50, 0.8],
                                            if level.author.len() > 16 {
                                                format!("{:.16}..", level.author)
                                            } else {
                                                level.author.clone()
                                            },
                                        );
                                        ui.text_colored(diff_col, diff_str);
                                        if let Some(t) = level.best_time_secs {
                                            ui.same_line();
                                            ui.text_colored(
                                                [0.35, 0.38, 0.50, 0.7],
                                                format!("  {:.1}s", t),
                                            );
                                        }
                                        if ui.button_with_size("Play", [card_w - 16.0, 22.0]) {
                                            game.on_event(
                                                &EngineEvent::KeyDown {
                                                    key: EngineKey::Escape,
                                                },
                                                ctx,
                                            );
                                        }
                                    });

                                col += 1;
                                if col < 3 {
                                    ui.same_line();
                                } else {
                                    col = 0;
                                }
                            }
                        }

                        // Attribution notice at bottom
                        ui.spacing();
                        ui.separator();
                        ui.text_colored(
                            [0.25, 0.27, 0.36, 0.7],
                            "Models from Poly Pizza (poly.pizza) - CC-BY licenced",
                        );
                    }); // end content child

                // Attribution at window bottom
                ui.spacing();
                ui.separator();
                ui.text_colored(
                    [0.25, 0.27, 0.36, 0.7],
                    "Models from Poly Pizza (poly.pizza) - CC-BY licenced",
                );
            });
    }

    // ── GL lifecycle ──────────────────────────────────────────────────────
    fn init_gl(
        &mut self,
        elwt: &winit::event_loop::ActiveEventLoop,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.gl_display.is_none() {
            let wb = WindowAttributes::default().with_title("Formosaic");
            let template = ConfigTemplateBuilder::new().with_depth_size(24);
            let display_builder = DisplayBuilder::new().with_window_attributes(Some(wb));
            let (window_opt, gl_config) =
                display_builder.build(elwt, template, |mut c| c.next().unwrap())?;
            let window = window_opt.ok_or("Failed to create window")?;
            self.window = Some(Arc::new(window));
            self.gl_display = Some(gl_config.display());
        }

        let window = self.window.as_ref().unwrap();
        let display = self.gl_display.as_ref().unwrap();
        let wh = window.window_handle()?.as_raw();

        let ctx_attrs = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::Gles(Some(glutin::context::Version::new(3, 1))))
            .build(Some(wh));

        let config = unsafe {
            display.find_configs(ConfigTemplateBuilder::new().with_depth_size(24).build())
        }?
        .next()
        .ok_or("No GL config found")?;

        let not_current: NotCurrentContext =
            unsafe { display.create_context(&config, &ctx_attrs)? };

        let size = window.inner_size();
        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            wh,
            NonZeroU32::new(size.width).unwrap(),
            NonZeroU32::new(size.height).unwrap(),
        );
        let gl_surface = unsafe { display.create_window_surface(&config, &attrs)? };
        let gl_context = not_current.make_current(&gl_surface)?;
        gl_surface.set_swap_interval(&gl_context, SwapInterval::DontWait)?;
        gl::load_with(|s| display.get_proc_address(&std::ffi::CString::new(s).unwrap()));

        if let Some(r) = Self::get_gl_string(gl::RENDERER) {
            log::info!("Running on {}", r.to_string_lossy());
        }

        // ── imgui setup ───────────────────────────────────────────────────
        let mut imgui = imgui::Context::create();
        imgui.set_ini_filename(None); // no .ini persistence
        let style = imgui.style_mut();
        // Dark theme matching our aesthetic
        style.window_rounding = 4.0;
        style.frame_rounding = 3.0;
        style.popup_rounding = 3.0;
        style.scrollbar_rounding = 3.0;
        style.grab_rounding = 3.0;
        style.window_border_size = 1.0;
        style[imgui::StyleColor::WindowBg] = [0.06, 0.07, 0.10, 0.96];
        style[imgui::StyleColor::TitleBg] = [0.04, 0.05, 0.08, 1.0];
        style[imgui::StyleColor::TitleBgActive] = [0.08, 0.09, 0.14, 1.0];
        style[imgui::StyleColor::Border] = [0.12, 0.13, 0.21, 1.0];
        style[imgui::StyleColor::FrameBg] = [0.10, 0.11, 0.18, 1.0];
        style[imgui::StyleColor::FrameBgHovered] = [0.15, 0.17, 0.26, 1.0];
        style[imgui::StyleColor::Button] = [0.19, 0.31, 0.75, 0.75];
        style[imgui::StyleColor::ButtonHovered] = [0.25, 0.40, 0.90, 0.90];
        style[imgui::StyleColor::ButtonActive] = [0.75, 0.19, 0.29, 1.0];
        style[imgui::StyleColor::Header] = [0.19, 0.31, 0.75, 0.45];
        style[imgui::StyleColor::HeaderHovered] = [0.19, 0.31, 0.75, 0.80];
        style[imgui::StyleColor::CheckMark] = [0.18, 0.75, 0.50, 1.0];
        style[imgui::StyleColor::Text] = [0.82, 0.85, 0.94, 1.0];
        style[imgui::StyleColor::TextDisabled] = [0.35, 0.38, 0.50, 1.0];
        style[imgui::StyleColor::Separator] = [0.12, 0.13, 0.21, 1.0];
        style[imgui::StyleColor::SeparatorHovered] = [0.40, 0.50, 0.80, 0.78];

        // Touch-friendly sizing on Android
        #[cfg(target_os = "android")]
        {
            imgui.io_mut().font_global_scale = 2.0;
            style.touch_extra_padding = [8.0, 8.0];
        }

        let mut platform = imgui_winit_support::WinitPlatform::new(&mut imgui);
        platform.attach_window(
            imgui.io_mut(),
            window,
            imgui_winit_support::HiDpiMode::Default,
        );

        let imgui_renderer =
            ImguiGlRenderer::new(&mut imgui).map_err(|e| format!("imgui renderer: {}", e))?;

        // ── Game + pipeline setup ─────────────────────────────────────────
        self.scene_context = Some(Rc::new(RefCell::new(SceneContext::new())));
        self.game = Some(Formosaic::new());
        if let Some(g) = self.game.as_mut() {
            g.on_init(
                &mut self
                    .scene_context
                    .clone()
                    .expect("No scene context.")
                    .borrow_mut(),
            );
        }
        self.pipeline = Some(Pipeline::new(
            self.scene_context.clone().expect("No scene context."),
        ));

        // Touch button renderer (only needed on Android, but init everywhere for simplicity)
        self.touch_renderer = MenuRenderer::new()
            .map_err(|e| {
                log::warn!("TouchRenderer init failed: {}", e);
                e
            })
            .ok();

        self.gl_context = Some(gl_context);
        self.gl_surface = Some(gl_surface);
        self.imgui_ctx = Some(imgui);
        self.imgui_platform = Some(platform);
        self.imgui_renderer = Some(imgui_renderer);

        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::ClearColor(0.02, 0.03, 0.05, 1.0);
            gl::Viewport(0, 0, size.width as i32, size.height as i32);
        }
        Ok(())
    }

    fn cleanup_gl(&mut self) {
        self.imgui_renderer = None;
        self.imgui_platform = None;
        self.imgui_ctx = None;
        self.touch_renderer = None;
        self.pipeline = None;
        self.game = None;
        self.gl_context = None;
        self.gl_surface = None;
        ModelCache::clear();
    }

    fn get_gl_string(variant: gl::types::GLenum) -> Option<&'static CStr> {
        unsafe {
            let s = gl::GetString(variant);
            (!s.is_null()).then(|| CStr::from_ptr(s.cast()))
        }
    }

    fn gl_error_to_string(err: gl::types::GLenum) -> &'static str {
        match err {
            gl::NO_ERROR => "No error",
            gl::INVALID_ENUM => "GL_INVALID_ENUM",
            gl::INVALID_VALUE => "GL_INVALID_VALUE",
            gl::INVALID_OPERATION => "GL_INVALID_OPERATION",
            gl::OUT_OF_MEMORY => "GL_OUT_OF_MEMORY",
            gl::INVALID_FRAMEBUFFER_OPERATION => "GL_INVALID_FRAMEBUFFER_OPERATION",
            _ => "Unknown GL error",
        }
    }
}

impl Default for GameEngine {
    fn default() -> Self {
        Self::new()
    }
}
