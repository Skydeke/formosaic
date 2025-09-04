use winit::event_loop::EventLoop;

use glutin::{
    config::ConfigTemplateBuilder,
    context::{ContextApi, ContextAttributesBuilder, NotCurrentContext},
    display::GetGlDisplay,
    prelude::*,
    surface::{SurfaceAttributesBuilder, WindowSurface},
};
use glutin_winit::DisplayBuilder;
use std::{ffi::CStr, num::NonZeroU32};
use winit::event::ElementState;
use winit::event::MouseButton;
use winit::event::{Event as WinitEvent, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::keyboard::Key;
use winit::keyboard::NamedKey;
use winit::raw_window_handle::HasWindowHandle;
use winit::window::{Window, WindowAttributes};

use crate::input::Key as EngineKey;
use crate::{input::Event as EngineEvent, renderer::Renderer, Game};

static mut GAME: Option<Game> = None;
static mut RENDERER: Option<Renderer> = None;
static mut GL_CONTEXT: Option<glutin::context::PossiblyCurrentContext> = None;
static mut GL_SURFACE: Option<glutin::surface::Surface<glutin::surface::WindowSurface>> = None;
static mut WINDOW: Option<std::sync::Arc<Window>> = None;

pub fn run_engine(event_loop: EventLoop<()>) {
    let mut gl_initialized = false;
    let mut last_cursor_pos = (0.0_f32, 0.0_f32);
    let mut last_frame_time = std::time::Instant::now();

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);

        match event {
            WinitEvent::WindowEvent {
                event,
                window_id: _,
            } => match event {
                WindowEvent::CloseRequested => elwt.exit(),
                WindowEvent::Resized(size) => {
                    if gl_initialized {
                        unsafe {
                            if let (Some(gl_surface), Some(gl_context)) = (&GL_SURFACE, &GL_CONTEXT)
                            {
                                gl_surface.resize(
                                    gl_context,
                                    NonZeroU32::new(size.width.max(1)).unwrap(),
                                    NonZeroU32::new(size.height.max(1)).unwrap(),
                                );
                                gl::Viewport(0, 0, size.width as i32, size.height as i32);
                            }
                        }
                    }
                }
                WindowEvent::RedrawRequested => unsafe {
                    if gl_initialized {
                        if let (
                            Some(game),
                            Some(renderer),
                            Some(gl_surface),
                            Some(gl_context),
                            Some(window),
                        ) = (&mut GAME, &mut RENDERER, &GL_SURFACE, &GL_CONTEXT, &WINDOW)
                        {
                            let now = std::time::Instant::now();
                            let delta_time = (now - last_frame_time).as_secs_f32();
                            last_frame_time = now;

                            let size = window.inner_size();
                            game.update(delta_time);
                            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
                            game.render(renderer, size.width, size.height);
                            let err = unsafe { gl::GetError() };
                            if err != gl::NO_ERROR {
                                log::log!(
                                    log::Level::Error,
                                    "OpenGL error: 0x{:X} -- {}",
                                    err,
                                    gl_error_to_string(err)
                                );
                            }
                            let _ = gl_surface.swap_buffers(gl_context);
                        }
                    }
                },
                WindowEvent::MouseInput { state, button, .. } => unsafe {
                    if gl_initialized {
                        if let (Some(game), Some(window)) = (&mut GAME, &WINDOW) {
                            let size = window.inner_size();
                            match (state, button) {
                                (ElementState::Pressed, MouseButton::Left) => {
                                    game.handle_event(EngineEvent::MouseDown {
                                        x: last_cursor_pos.0,
                                        y: last_cursor_pos.1,
                                        width: size.width as f32,
                                        height: size.height as f32,
                                    });
                                }
                                (ElementState::Released, MouseButton::Left) => {
                                    game.handle_event(EngineEvent::MouseUp {
                                        x: last_cursor_pos.0,
                                        y: last_cursor_pos.1,
                                        width: size.width as f32,
                                        height: size.height as f32,
                                    });
                                }
                                _ => {}
                            }
                        }
                    }
                },
                WindowEvent::CursorMoved { position, .. } => unsafe {
                    last_cursor_pos = (position.x as f32, position.y as f32);
                    if gl_initialized {
                        if let (Some(game), Some(window)) = (&mut GAME, &WINDOW) {
                            let size = window.inner_size();
                            game.handle_event(EngineEvent::MouseMove {
                                x: last_cursor_pos.0,
                                y: last_cursor_pos.1,
                                width: size.width as f32,
                                height: size.height as f32,
                            });
                        }
                    }
                },
                WindowEvent::KeyboardInput { event, .. } => unsafe {
                    if gl_initialized {
                        if let Some(game) = &mut GAME {
                            let key = match event.logical_key {
                                Key::Named(NamedKey::Escape) => EngineKey::Escape,
                                Key::Character(ref s) if s.as_str() == "r" => EngineKey::R,
                                Key::Named(NamedKey::Space) => EngineKey::Space,
                                _ => EngineKey::Other,
                            };
                            if event.state == ElementState::Pressed {
                                game.handle_event(EngineEvent::KeyDown { key });
                            }
                        }
                    }
                },
                WindowEvent::Touch(touch) => unsafe {
                    if gl_initialized {
                        if let (Some(game), Some(window)) = (&mut GAME, &WINDOW) {
                            let size = window.inner_size();
                            match touch.phase {
                                winit::event::TouchPhase::Started => {
                                    game.handle_event(EngineEvent::TouchDown {
                                        id: touch.id,
                                        x: touch.location.x as f32,
                                        y: touch.location.y as f32,
                                        width: size.width as f32,
                                        height: size.height as f32,
                                    });
                                }
                                winit::event::TouchPhase::Moved => {
                                    game.handle_event(EngineEvent::TouchMove {
                                        id: touch.id,
                                        x: touch.location.x as f32,
                                        y: touch.location.y as f32,
                                        width: size.width as f32,
                                        height: size.height as f32,
                                    });
                                }
                                winit::event::TouchPhase::Ended
                                | winit::event::TouchPhase::Cancelled => {
                                    game.handle_event(EngineEvent::TouchUp { id: touch.id });
                                }
                            }
                        }
                    }
                },

                _ => {}
            },
            WinitEvent::AboutToWait => unsafe {
                if let Some(window) = &WINDOW {
                    window.request_redraw();
                }
            },
            WinitEvent::Resumed => {
                log::info!("App resumed, initializing OpenGL...");

                if !gl_initialized {
                    if let Err(e) = init_gl(elwt) {
                        log::error!("Failed to initialize OpenGL: {}", e);
                        elwt.exit();
                        return;
                    }
                    gl_initialized = true;
                    log::info!("OpenGL initialized successfully");
                }
            }
            WinitEvent::Suspended => {
                log::info!("App suspended, cleaning up OpenGL...");
                cleanup_gl();
                gl_initialized = false;
            }
            _ => {}
        }
    });
}

fn init_gl(elwt: &winit::event_loop::ActiveEventLoop) -> Result<(), Box<dyn std::error::Error>> {
    let wb = WindowAttributes::default().with_title("Formosaic");
    let template = ConfigTemplateBuilder::new().with_depth_size(24);
    let display_builder = DisplayBuilder::new().with_window_attributes(Some(wb));

    let (window_opt, gl_config) =
        display_builder.build(elwt, template, |mut configs| configs.next().unwrap())?;

    let window = window_opt.ok_or("Failed to create window")?;

    let window_handle = loop {
        match window.window_handle() {
            Ok(handle) => break handle.as_raw(),
            Err(_) => {
                log::debug!("Window handle not ready, waiting...");
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            }
        }
    };

    let context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::Gles(Some(glutin::context::Version::new(3, 0))))
        .build(Some(window_handle));

    let not_current_gl_context: NotCurrentContext = unsafe {
        gl_config
            .display()
            .create_context(&gl_config, &context_attributes)?
    };

    let size = window.inner_size();
    let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
        window_handle,
        NonZeroU32::new(size.width).unwrap(),
        NonZeroU32::new(size.height).unwrap(),
    );

    let gl_surface = unsafe {
        gl_config
            .display()
            .create_window_surface(&gl_config, &attrs)?
    };

    let gl_context = not_current_gl_context.make_current(&gl_surface)?;

    gl::load_with(|s| {
        gl_config
            .display()
            .get_proc_address(&std::ffi::CString::new(s).unwrap())
    });

    if let Some(renderer) = get_gl_string(gl::RENDERER) {
        log::info!("Running on {}", renderer.to_string_lossy());
    }
    if let Some(version) = get_gl_string(gl::VERSION) {
        log::info!("OpenGL Version {}", version.to_string_lossy());
    }
    if let Some(shaders_version) = get_gl_string(gl::SHADING_LANGUAGE_VERSION) {
        log::info!("Shaders version {}", shaders_version.to_string_lossy());
    }

    unsafe {
        RENDERER = Some(Renderer::new()?);
        GAME = Some(Game::new()?);
        GL_CONTEXT = Some(gl_context);
        GL_SURFACE = Some(gl_surface);
        WINDOW = Some(std::sync::Arc::new(window));

        gl::Enable(gl::DEPTH_TEST);
        gl::ClearColor(0.2, 0.3, 0.3, 1.0);
        let sz = WINDOW.as_ref().unwrap().inner_size();
        gl::Viewport(0, 0, sz.width as i32, sz.height as i32);
    }

    Ok(())
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
        gl::STACK_UNDERFLOW => "GL_STACK_UNDERFLOW: This command would cause a stack underflow.",
        gl::OUT_OF_MEMORY => {
            "GL_OUT_OF_MEMORY: There is not enough memory left to execute the command."
        }
        gl::INVALID_FRAMEBUFFER_OPERATION => {
            "GL_INVALID_FRAMEBUFFER_OPERATION: The framebuffer object is not complete."
        }
        _ => "Unknown OpenGL error",
    }
}
fn cleanup_gl() {
    unsafe {
        GAME = None;
        RENDERER = None;
        GL_CONTEXT = None;
        GL_SURFACE = None;
        WINDOW = None;
    }
}
