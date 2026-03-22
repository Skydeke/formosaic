//! GameEngine — generic winit/glutin application host.
//!
//! Drives any `Application` implementation through the GL lifecycle,
//! input routing, and imgui frame management.  Contains zero game logic.
//!
//! Enabled by the `windowed` feature of `formosaic-engine`.

use std::cell::RefCell;
use std::ffi::CStr;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::Arc;

use glutin::{
    config::ConfigTemplateBuilder,
    context::{ContextApi, ContextAttributesBuilder, NotCurrentContext, PossiblyCurrentContext},
    display::{Display, GetGlDisplay},
    prelude::*,
    surface::{Surface, SurfaceAttributesBuilder, SwapInterval, WindowSurface},
};
use glutin_winit::DisplayBuilder;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{Key, NamedKey},
    raw_window_handle::HasWindowHandle,
    window::{Window, WindowAttributes, WindowId},
};

use crate::{
    app::application::Application,
    architecture::{models::model_cache::ModelCache, scene::scene_context::SceneContext},
    input::{Event as EngineEvent, Key as EngineKey},
    rendering::{instances::imgui_render::ImguiGlRenderer, pipeline::Pipeline},
};

pub struct GameEngine<A: Application> {
    app:             Option<A>,
    pipeline:        Option<Pipeline>,
    scene_context:   Option<Rc<RefCell<SceneContext>>>,
    gl_display:      Option<Display>,
    gl_context:      Option<PossiblyCurrentContext>,
    gl_surface:      Option<Surface<WindowSurface>>,
    window:          Option<Arc<Window>>,
    gl_initialized:  bool,
    last_cursor_pos: (f32, f32),
    last_frame_time: std::time::Instant,
    fps_accumulator: f32,
    frame_count:     u32,
}

impl<A: Application + 'static> GameEngine<A> {
    pub fn new(app: A) -> Self {
        Self {
            app: Some(app),
            pipeline: None, scene_context: None,
            gl_display: None, gl_context: None, gl_surface: None,
            window: None, gl_initialized: false,
            last_cursor_pos: (0.0, 0.0),
            last_frame_time: std::time::Instant::now(),
            fps_accumulator: 0.0, frame_count: 0,
        }
    }

    // ── GL lifecycle ──────────────────────────────────────────────────────

    fn init_gl(&mut self, elwt: &ActiveEventLoop) -> Result<(), Box<dyn std::error::Error>> {
        let title = self.app.as_ref().map(|a| a.title().to_owned())
                        .unwrap_or_else(|| "App".to_owned());

        if self.gl_display.is_none() {
            let wb = WindowAttributes::default().with_title(title);
            let template = ConfigTemplateBuilder::new().with_depth_size(24);
            let display_builder = DisplayBuilder::new().with_window_attributes(Some(wb));
            let (window_opt, gl_config) =
                display_builder.build(elwt, template, |mut c| c.next().unwrap())?;
            let window = window_opt.ok_or("Failed to create window")?;
            self.window     = Some(Arc::new(window));
            self.gl_display = Some(gl_config.display());
        }

        let window  = self.window.as_ref().unwrap();
        let display = self.gl_display.as_ref().unwrap();
        let wh      = window.window_handle()?.as_raw();

        let ctx_attrs = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::Gles(Some(glutin::context::Version::new(3, 1))))
            .build(Some(wh));

        let config = unsafe {
            display.find_configs(ConfigTemplateBuilder::new().with_depth_size(24).build())
        }?.next().ok_or("No GL config found")?;

        let not_current: NotCurrentContext =
            unsafe { display.create_context(&config, &ctx_attrs)? };

        let size  = window.inner_size();
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

        // imgui — app configures theme, fonts, DPI.
        // The renderer takes ownership of imgui+platform (no raw pointer escapes).
        let mut imgui = imgui::Context::create();
        let scale = window.scale_factor() as f32;
        if let Some(a) = &self.app { a.configure_imgui(&mut imgui, scale); }

        let mut platform = imgui_winit_support::WinitPlatform::new(&mut imgui);
        platform.attach_window(
            imgui.io_mut(), window, imgui_winit_support::HiDpiMode::Default,
        );

        let imgui_renderer = ImguiGlRenderer::new(imgui, platform)
            .map_err(|e| format!("imgui renderer: {}", e))?;

        // Scene context + pipeline
        let ctx = Rc::new(RefCell::new(SceneContext::new()));
        self.scene_context = Some(ctx.clone());

        if let Some(a) = self.app.as_mut() {
            a.on_init(&mut ctx.borrow_mut());
        }

        let mut pipeline = Pipeline::new(ctx);

        // App registers its own game-specific renderers
        if let Some(a) = self.app.as_mut() {
            a.register_renderers(&mut pipeline);
        }
        pipeline.add_imgui_renderer(imgui_renderer);
        self.pipeline = Some(pipeline);

        self.gl_context = Some(gl_context);
        self.gl_surface = Some(gl_surface);

        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Viewport(0, 0, size.width as i32, size.height as i32);
        }
        Ok(())
    }

    fn cleanup_gl(&mut self) {
        self.pipeline   = None;   // drops ImguiGlRenderer which owns imgui
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
            gl::NO_ERROR                      => "No error",
            gl::INVALID_ENUM                  => "GL_INVALID_ENUM",
            gl::INVALID_VALUE                 => "GL_INVALID_VALUE",
            gl::INVALID_OPERATION             => "GL_INVALID_OPERATION",
            gl::OUT_OF_MEMORY                 => "GL_OUT_OF_MEMORY",
            gl::INVALID_FRAMEBUFFER_OPERATION => "GL_INVALID_FRAMEBUFFER_OPERATION",
            _                                 => "Unknown GL error",
        }
    }
}

impl<A: Application + 'static> ApplicationHandler for GameEngine<A> {
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

    fn suspended(&mut self, _: &ActiveEventLoop) {
        log::info!("App suspended, cleaning up OpenGL...");
        self.cleanup_gl();
        self.gl_initialized = false;
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, wid: WindowId, event: WindowEvent) {
        // Feed to imgui first (routes through the renderer which owns the context)
        if let (Some(pipeline), Some(window)) = (&mut self.pipeline, &self.window) {
            if let Some(imgui_r) = pipeline.imgui_renderer_mut() {
                let wrapped = winit::event::Event::<()>::WindowEvent {
                    window_id: wid, event: event.clone(),
                };
                imgui_r.handle_event(window, &wrapped);
            }
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                if let (Some(surf), Some(ctx)) = (&self.gl_surface, &self.gl_context) {
                    surf.resize(ctx,
                        NonZeroU32::new(size.width.max(1)).unwrap(),
                        NonZeroU32::new(size.height.max(1)).unwrap(),
                    );
                    unsafe { gl::Viewport(0, 0, size.width as i32, size.height as i32); }
                }
            }

            WindowEvent::RedrawRequested => {
                if let (Some(app), Some(surf), Some(ctx), Some(window)) = (
                    &mut self.app, &self.gl_surface, &self.gl_context, &self.window,
                ) {
                    let now        = std::time::Instant::now();
                    let dt         = (now - self.last_frame_time).as_secs_f32();
                    self.last_frame_time = now;

                    self.fps_accumulator += dt;
                    self.frame_count     += 1;
                    if self.fps_accumulator >= 20.0 {
                        log::info!("FPS: {:.1}", self.frame_count as f32 / self.fps_accumulator);
                        self.fps_accumulator = 0.0;
                        self.frame_count     = 0;
                    }

                    let size = window.inner_size();
                    let pw   = size.width  as f32;
                    let ph   = size.height as f32;

                    let sc_rc = self.scene_context.clone().expect("no scene context");
                    app.on_update(dt, &mut sc_rc.borrow_mut());

                    {
                        let mut sc = sc_rc.borrow_mut();
                        app.populate_scene_context(&mut sc, dt);

                        // Let the imgui renderer run its new-frame + UiNode collection.
                        // It owns imgui::Context so DrawData lifetime is fully contained.
                        let scale = window.scale_factor() as f32;
                        if let Some(pipeline) = &mut self.pipeline {
                            if let Some(imgui_r) = pipeline.imgui_renderer_mut() {
                                imgui_r.prepare(window, pw, ph, scale, &mut sc);
                            }
                        }
                    }

                    self.pipeline.as_mut().unwrap().draw(size.width, size.height);

                    let err = unsafe { gl::GetError() };
                    if err != gl::NO_ERROR {
                        log::error!("[Pipeline] GL 0x{:X}: {}", err, Self::gl_error_to_string(err));
                    }

                    let _ = surf.swap_buffers(ctx);
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if let (Some(app), Some(window)) = (&mut self.app, &self.window) {
                    if !self.pipeline.as_ref().and_then(|p| p.imgui_renderer()).map(|r| r.wants_mouse()).unwrap_or(false) {
                        let size = window.inner_size();
                        let sc_rc = self.scene_context.clone().expect("no scene context");
                        match (state, button) {
                            (ElementState::Pressed, MouseButton::Left) => {
                                app.on_event(&EngineEvent::MouseDown {
                                    x: self.last_cursor_pos.0, y: self.last_cursor_pos.1,
                                    width: size.width as f32, height: size.height as f32,
                                }, &mut sc_rc.borrow_mut());
                            }
                            (ElementState::Released, MouseButton::Left) => {
                                app.on_event(&EngineEvent::MouseUp {
                                    x: self.last_cursor_pos.0, y: self.last_cursor_pos.1,
                                    width: size.width as f32, height: size.height as f32,
                                }, &mut sc_rc.borrow_mut());
                            }
                            _ => {}
                        }
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.last_cursor_pos = (position.x as f32, position.y as f32);
                if let (Some(app), Some(window)) = (&mut self.app, &self.window) {
                    if !self.pipeline.as_ref().and_then(|p| p.imgui_renderer()).map(|r| r.wants_mouse()).unwrap_or(false) {
                        let size  = window.inner_size();
                        let sc_rc = self.scene_context.clone().expect("no scene context");
                        app.on_event(&EngineEvent::MouseMove {
                            x: self.last_cursor_pos.0, y: self.last_cursor_pos.1,
                            width: size.width as f32, height: size.height as f32,
                        }, &mut sc_rc.borrow_mut());
                    }
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(app) = &mut self.app {
                    if !self.pipeline.as_ref().and_then(|p| p.imgui_renderer()).map(|r| r.wants_keyboard()).unwrap_or(false) {
                        if event.state == ElementState::Pressed {
                            let key = match event.logical_key {
                                Key::Named(NamedKey::Escape)                => EngineKey::Escape,
                                Key::Character(ref s) if s.as_str() == "h" => EngineKey::H,
                                Key::Character(ref s) if s.as_str() == "r" => EngineKey::R,
                                Key::Character(ref s) if s.as_str() == "n" => EngineKey::N,
                                Key::Character(ref s) if s.as_str() == "k" => EngineKey::K,
                                Key::Named(NamedKey::Space)                 => EngineKey::Space,
                                _                                           => EngineKey::Other,
                            };
                            let sc_rc = self.scene_context.clone().expect("no scene context");
                            app.on_event(&EngineEvent::KeyDown { key }, &mut sc_rc.borrow_mut());
                        }
                    }
                }
            }

            WindowEvent::Touch(touch) => {
                if let (Some(app), Some(window)) = (&mut self.app, &self.window) {
                    let size = window.inner_size();
                    let tx = touch.location.x as f32;
                    let ty = touch.location.y as f32;
                    let tw = size.width  as f32;
                    let th = size.height as f32;
                    let sc_rc = self.scene_context.clone().expect("no scene context");

                    match touch.phase {
                        winit::event::TouchPhase::Started => app.on_event(
                            &EngineEvent::TouchDown { id: touch.id, x: tx, y: ty, width: tw, height: th },
                            &mut sc_rc.borrow_mut(),
                        ),
                        winit::event::TouchPhase::Moved => app.on_event(
                            &EngineEvent::TouchMove { id: touch.id, x: tx, y: ty, width: tw, height: th },
                            &mut sc_rc.borrow_mut(),
                        ),
                        winit::event::TouchPhase::Ended
                        | winit::event::TouchPhase::Cancelled => app.on_event(
                            &EngineEvent::TouchUp { id: touch.id },
                            &mut sc_rc.borrow_mut(),
                        ),
                    }
                }
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(w) = &self.window { w.request_redraw(); }
    }
}
