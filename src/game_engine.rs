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
use crate::engine::rendering::pipeline::Pipeline;
use crate::formosaic::Application;
use crate::input::Key as EngineKey;
use crate::{input::Event as EngineEvent, Formosaic};

pub struct GameEngine {
    game: Option<Formosaic>,
    pipeline: Option<Pipeline>,
    scene_context: Option<Rc<RefCell<SceneContext>>>,
    gl_display: Option<Display>,                // persistent
    gl_context: Option<PossiblyCurrentContext>, // recreated
    gl_surface: Option<Surface<WindowSurface>>, // recreated
    window: Option<Arc<Window>>,
    gl_initialized: bool,
    last_cursor_pos: (f32, f32),
    last_frame_time: std::time::Instant,
    fps_accumulator: f32,
    frame_count: u32,
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
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => _event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let (Some(gl_surface), Some(gl_context)) = (&self.gl_surface, &self.gl_context) {
                    gl_surface.resize(
                        gl_context,
                        NonZeroU32::new(size.width.max(1)).unwrap(),
                        NonZeroU32::new(size.height.max(1)).unwrap(),
                    );
                    unsafe { gl::Viewport(0, 0, size.width as i32, size.height as i32) };
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

                    // Update FPS counter
                    self.fps_accumulator += delta_time;
                    self.frame_count += 1;

                    if self.fps_accumulator >= 20.0 {
                        let fps = self.frame_count as f32 / self.fps_accumulator;
                        log::info!("Formosaic - FPS: {:.1}", fps);
                        self.fps_accumulator = 0.0;
                        self.frame_count = 0;
                    }

                    let size = window.inner_size();
                    game.on_update(
                        delta_time,
                        &mut self
                            .scene_context
                            .clone()
                            .expect("No scene context.")
                            .borrow_mut(),
                    );
                    self.pipeline
                        .as_mut()
                        .unwrap()
                        .draw(size.width, size.height);

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
            WindowEvent::CursorMoved { position, .. } => {
                self.last_cursor_pos = (position.x as f32, position.y as f32);
                if let (Some(game), Some(window)) = (&mut self.game, &self.window) {
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
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(game) = &mut self.game {
                    let key = match event.logical_key {
                        Key::Named(NamedKey::Escape) => EngineKey::Escape,
                        Key::Character(ref s) if s.as_str() == "r" => EngineKey::R,
                        Key::Character(ref s) if s.as_str() == "n" => EngineKey::N,
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
            WindowEvent::Touch(touch) => {
                if let (Some(game), Some(window)) = (&mut self.game, &self.window) {
                    let size = window.inner_size();
                    match touch.phase {
                        winit::event::TouchPhase::Started => {
                            game.on_event(
                                &EngineEvent::TouchDown {
                                    id: touch.id,
                                    x: touch.location.x as f32,
                                    y: touch.location.y as f32,
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
                        winit::event::TouchPhase::Moved => {
                            game.on_event(
                                &EngineEvent::TouchMove {
                                    id: touch.id,
                                    x: touch.location.x as f32,
                                    y: touch.location.y as f32,
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
        }
    }

    fn init_gl(
        &mut self,
        elwt: &winit::event_loop::ActiveEventLoop,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create display once
        if self.gl_display.is_none() {
            let wb = WindowAttributes::default().with_title("Formosaic");
            let template = ConfigTemplateBuilder::new().with_depth_size(24);
            let display_builder = DisplayBuilder::new().with_window_attributes(Some(wb));

            let (window_opt, gl_config) =
                display_builder.build(elwt, template, |mut configs| configs.next().unwrap())?;
            let window = window_opt.ok_or("Failed to create window")?;

            self.window = Some(Arc::new(window));
            self.gl_display = Some(gl_config.display());
            // store config too if you need consistent formats across resumes
        }

        let window = self.window.as_ref().unwrap();
        let display = self.gl_display.as_ref().unwrap();

        let window_handle = window.window_handle()?.as_raw();

        let context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::Gles(Some(glutin::context::Version::new(3, 1))))
            .build(Some(window_handle));

        // Pick config from display
        let config = unsafe {
            display.find_configs(ConfigTemplateBuilder::new().with_depth_size(24).build())
        }?
        .next()
        .ok_or("No GL config found")?;

        let not_current_gl_context: NotCurrentContext =
            unsafe { display.create_context(&config, &context_attributes)? };

        let size = window.inner_size();
        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            window_handle,
            NonZeroU32::new(size.width).unwrap(),
            NonZeroU32::new(size.height).unwrap(),
        );

        let gl_surface = unsafe { display.create_window_surface(&config, &attrs)? };

        let gl_context = not_current_gl_context.make_current(&gl_surface)?;
        gl_surface.set_swap_interval(&gl_context, SwapInterval::DontWait)?;

        gl::load_with(|s| display.get_proc_address(&std::ffi::CString::new(s).unwrap()));

        if let Some(renderer) = Self::get_gl_string(gl::RENDERER) {
            log::info!("Running on {}", renderer.to_string_lossy());
        }

        self.scene_context = Some(Rc::new(RefCell::new(SceneContext::new())));
        self.game = Some(Formosaic::new());
        if let Some(game) = self.game.as_mut() {
            game.on_init(
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
        self.gl_context = Some(gl_context);
        self.gl_surface = Some(gl_surface);

        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::ClearColor(0.2, 0.3, 0.3, 1.0);
            let sz = window.inner_size();
            gl::Viewport(0, 0, sz.width as i32, sz.height as i32);
        }

        Ok(())
    }

    fn cleanup_gl(&mut self) {
        self.pipeline = None;
        self.game = None;
        self.gl_context = None;
        self.gl_surface = None;
        // Models are remoced from Memory when OpenGL is destroyed
        ModelCache::clear();
        // Keep display + window alive!
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
            gl::INVALID_ENUM => {
                "GL_INVALID_ENUM: An unacceptable value is specified for an enumerated argument."
            }
            gl::INVALID_VALUE => "GL_INVALID_VALUE: A numeric argument is out of range.",
            gl::INVALID_OPERATION => {
                "GL_INVALID_OPERATION: The specified operation is not allowed in the current state."
            }
            gl::STACK_OVERFLOW => "GL_STACK_OVERFLOW: This command would cause a stack overflow.",
            gl::STACK_UNDERFLOW => {
                "GL_STACK_UNDERFLOW: This command would cause a stack underflow."
            }
            gl::OUT_OF_MEMORY => {
                "GL_OUT_OF_MEMORY: There is not enough memory left to execute the command."
            }
            gl::INVALID_FRAMEBUFFER_OPERATION => {
                "GL_INVALID_FRAMEBUFFER_OPERATION: The framebuffer object is not complete."
            }
            _ => "Unknown OpenGL error",
        }
    }
}

impl Default for GameEngine {
    fn default() -> Self {
        Self::new()
    }
}
