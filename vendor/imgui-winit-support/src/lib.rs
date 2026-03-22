use imgui::{self, BackendFlags, ConfigFlags, Context, Io, Key, Ui};
use std::cmp::Ordering;

pub use winit;
use winit::{
    dpi::{LogicalPosition, LogicalSize},
    keyboard::{Key as WinitKey, KeyLocation, NamedKey},
};
use winit::{
    error::ExternalError,
    event::{ElementState, Event, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent},
    window::{CursorIcon as MouseCursor, Window},
};

#[derive(Debug)]
pub struct WinitPlatform {
    hidpi_mode: ActiveHiDpiMode,
    hidpi_factor: f64,
    cursor_cache: Option<CursorSettings>,
    /// Last touch Y position for synthesizing scroll from finger drag.
    touch_last_y: Option<f32>,
}

impl WinitPlatform {
    pub fn new(imgui: &mut Context) -> WinitPlatform {
        let io = imgui.io_mut();
        io.backend_flags.insert(BackendFlags::HAS_MOUSE_CURSORS);
        io.backend_flags.insert(BackendFlags::HAS_SET_MOUSE_POS);
        imgui.set_platform_name(Some(format!(
            "imgui-winit-support {}",
            env!("CARGO_PKG_VERSION")
        )));
        WinitPlatform {
            hidpi_mode: ActiveHiDpiMode::Default,
            hidpi_factor: 1.0,
            cursor_cache: None,
            touch_last_y: None,
        }
    }

    pub fn attach_window(&mut self, io: &mut Io, window: &Window, hidpi_mode: HiDpiMode) {
        let (hidpi_mode, hidpi_factor) = hidpi_mode.apply(window.scale_factor());
        self.hidpi_mode = hidpi_mode;
        self.hidpi_factor = hidpi_factor;
        io.display_framebuffer_scale = [hidpi_factor as f32, hidpi_factor as f32];
        let logical_size = window.inner_size().to_logical(hidpi_factor);
        let logical_size = self.scale_size_from_winit(window, logical_size);
        io.display_size = [logical_size.width as f32, logical_size.height as f32];
    }

    pub fn hidpi_factor(&self) -> f64 { self.hidpi_factor }

    pub fn scale_size_from_winit(&self, window: &Window, logical_size: LogicalSize<f64>) -> LogicalSize<f64> {
        match self.hidpi_mode {
            ActiveHiDpiMode::Default => logical_size,
            _ => logical_size.to_physical::<f64>(window.scale_factor()).to_logical(self.hidpi_factor),
        }
    }

    pub fn scale_pos_from_winit(&self, window: &Window, logical_pos: LogicalPosition<f64>) -> LogicalPosition<f64> {
        match self.hidpi_mode {
            ActiveHiDpiMode::Default => logical_pos,
            _ => logical_pos.to_physical::<f64>(window.scale_factor()).to_logical(self.hidpi_factor),
        }
    }

    pub fn scale_pos_for_winit(&self, window: &Window, logical_pos: LogicalPosition<f64>) -> LogicalPosition<f64> {
        match self.hidpi_mode {
            ActiveHiDpiMode::Default => logical_pos,
            _ => logical_pos.to_physical::<f64>(self.hidpi_factor).to_logical(window.scale_factor()),
        }
    }

    pub fn handle_event<T>(&mut self, io: &mut Io, window: &Window, event: &Event<T>) {
        match *event {
            Event::WindowEvent { window_id, ref event } if window_id == window.id() => {
                self.handle_window_event(io, window, event);
            }
            _ => (),
        }
    }

    fn handle_window_event(&mut self, io: &mut Io, window: &Window, event: &WindowEvent) {
        match *event {
            WindowEvent::Resized(physical_size) => {
                let logical_size = physical_size.to_logical(window.scale_factor());
                let logical_size = self.scale_size_from_winit(window, logical_size);
                io.display_size = [logical_size.width as f32, logical_size.height as f32];
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let hidpi_factor = match self.hidpi_mode {
                    ActiveHiDpiMode::Default => scale_factor,
                    ActiveHiDpiMode::Rounded => scale_factor.round(),
                    _ => return,
                };
                if io.mouse_pos[0].is_finite() && io.mouse_pos[1].is_finite() {
                    io.mouse_pos = [
                        io.mouse_pos[0] * (hidpi_factor / self.hidpi_factor) as f32,
                        io.mouse_pos[1] * (hidpi_factor / self.hidpi_factor) as f32,
                    ];
                }
                self.hidpi_factor = hidpi_factor;
                io.display_framebuffer_scale = [hidpi_factor as f32, hidpi_factor as f32];
                let logical_size = window.inner_size().to_logical(scale_factor);
                let logical_size = self.scale_size_from_winit(window, logical_size);
                io.display_size = [logical_size.width as f32, logical_size.height as f32];
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                let state = modifiers.state();
                io.add_key_event(Key::ModShift, state.shift_key());
                io.add_key_event(Key::ModCtrl, state.control_key());
                io.add_key_event(Key::ModAlt, state.alt_key());
                io.add_key_event(Key::ModSuper, state.super_key());
            }
            WindowEvent::KeyboardInput { ref event, .. } => {
                let is_pressed = event.state.is_pressed();
                if is_pressed {
                    if let Some(txt) = &event.text {
                        for ch in txt.chars() {
                            if ch != '\u{7f}' { io.add_input_character(ch) }
                        }
                    }
                }
                if let Some(key) = to_imgui_key(&event.logical_key, event.location) {
                    io.add_key_event(key, is_pressed);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let position = position.to_logical(window.scale_factor());
                let position = self.scale_pos_from_winit(window, position);
                io.add_mouse_pos_event([position.x as f32, position.y as f32]);
            }
            WindowEvent::MouseWheel { delta, phase: TouchPhase::Moved, .. } => {
                let (h, v) = match delta {
                    MouseScrollDelta::LineDelta(h, v) => (h, v),
                    MouseScrollDelta::PixelDelta(pos) => {
                        let pos = pos.to_logical::<f64>(self.hidpi_factor);
                        let h = match pos.x.partial_cmp(&0.0) {
                            Some(Ordering::Greater) => 1.0, Some(Ordering::Less) => -1.0, _ => 0.0,
                        };
                        let v = match pos.y.partial_cmp(&0.0) {
                            Some(Ordering::Greater) => 1.0, Some(Ordering::Less) => -1.0, _ => 0.0,
                        };
                        (h, v)
                    }
                };
                io.add_mouse_wheel_event([h, v]);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(mb) = to_imgui_mouse_button(button) {
                    io.add_mouse_button_event(mb, state == ElementState::Pressed);
                }
            }
            WindowEvent::Touch(touch) => {
                // touch.location is in physical pixels — convert to logical so it
                // matches imgui's coordinate space (display_size is in logical px).
                let logical = touch.location.to_logical::<f64>(window.scale_factor());
                let logical = self.scale_pos_from_winit(window, LogicalPosition::new(logical.x, logical.y));
                let ly = logical.y as f32;
                io.add_mouse_pos_event([logical.x as f32, ly]);
                match touch.phase {
                    TouchPhase::Started => {
                        self.touch_last_y = Some(ly);
                        io.add_mouse_button_event(imgui::MouseButton::Left, true);
                    }
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        self.touch_last_y = None;
                        io.add_mouse_button_event(imgui::MouseButton::Left, false);
                    }
                    TouchPhase::Moved => {
                        // Synthesize vertical scroll from finger drag so imgui
                        // child-window scrolling works without a scrollbar.
                        if let Some(last_y) = self.touch_last_y {
                            let delta = ly - last_y;
                            // Scale: imgui scroll unit ≈ one text line (~16px).
                            // Divide by 16 so a full-screen drag scrolls ~50 lines.
                            io.add_mouse_wheel_event([0.0, delta / 16.0]);
                        }
                        self.touch_last_y = Some(ly);
                    }
                }
            }
            WindowEvent::Focused(newly_focused) => {
                if !newly_focused { io.app_focus_lost = true; }
            }
            _ => (),
        }
    }

    pub fn prepare_frame(&self, io: &mut Io, window: &Window) -> Result<(), ExternalError> {
        if io.want_set_mouse_pos {
            let logical_pos = self.scale_pos_for_winit(
                window,
                LogicalPosition::new(f64::from(io.mouse_pos[0]), f64::from(io.mouse_pos[1])),
            );
            window.set_cursor_position(logical_pos)
        } else {
            Ok(())
        }
    }

    pub fn prepare_render(&mut self, ui: &Ui, window: &Window) {
        let io = ui.io();
        if !io.config_flags.contains(ConfigFlags::NO_MOUSE_CURSOR_CHANGE) {
            let cursor = CursorSettings { cursor: ui.mouse_cursor(), draw_cursor: io.mouse_draw_cursor };
            if self.cursor_cache != Some(cursor) {
                cursor.apply(window);
                self.cursor_cache = Some(cursor);
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum HiDpiMode {
    Default,
    Rounded,
    Locked(f64),
}

impl HiDpiMode {
    fn apply(&self, hidpi_factor: f64) -> (ActiveHiDpiMode, f64) {
        match *self {
            HiDpiMode::Default       => (ActiveHiDpiMode::Default, hidpi_factor),
            HiDpiMode::Rounded       => (ActiveHiDpiMode::Rounded, hidpi_factor.round()),
            HiDpiMode::Locked(value) => (ActiveHiDpiMode::Locked, value),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct CursorSettings { cursor: Option<imgui::MouseCursor>, draw_cursor: bool }

impl CursorSettings {
    fn apply(&self, window: &Window) {
        match self.cursor {
            Some(mc) if !self.draw_cursor => {
                window.set_cursor_visible(true);
                window.set_cursor(to_winit_cursor(mc));
            }
            _ => window.set_cursor_visible(false),
        }
    }
}

fn to_winit_cursor(cursor: imgui::MouseCursor) -> MouseCursor {
    match cursor {
        imgui::MouseCursor::Arrow      => MouseCursor::Default,
        imgui::MouseCursor::TextInput  => MouseCursor::Text,
        imgui::MouseCursor::ResizeAll  => MouseCursor::Move,
        imgui::MouseCursor::ResizeNS   => MouseCursor::NsResize,
        imgui::MouseCursor::ResizeEW   => MouseCursor::EwResize,
        imgui::MouseCursor::ResizeNESW => MouseCursor::NeswResize,
        imgui::MouseCursor::ResizeNWSE => MouseCursor::NwseResize,
        imgui::MouseCursor::Hand       => MouseCursor::Grab,
        imgui::MouseCursor::NotAllowed => MouseCursor::NotAllowed,
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum ActiveHiDpiMode { Default, Rounded, Locked }

fn to_imgui_mouse_button(button: MouseButton) -> Option<imgui::MouseButton> {
    match button {
        MouseButton::Left  | MouseButton::Other(0) => Some(imgui::MouseButton::Left),
        MouseButton::Right | MouseButton::Other(1) => Some(imgui::MouseButton::Right),
        MouseButton::Middle| MouseButton::Other(2) => Some(imgui::MouseButton::Middle),
        MouseButton::Other(3) => Some(imgui::MouseButton::Extra1),
        MouseButton::Other(4) => Some(imgui::MouseButton::Extra2),
        _ => None,
    }
}

fn to_imgui_key(key: &winit::keyboard::Key, location: KeyLocation) -> Option<Key> {
    match (key.as_ref(), location) {
        (WinitKey::Named(NamedKey::Tab),        _) => Some(Key::Tab),
        (WinitKey::Named(NamedKey::ArrowLeft),  _) => Some(Key::LeftArrow),
        (WinitKey::Named(NamedKey::ArrowRight), _) => Some(Key::RightArrow),
        (WinitKey::Named(NamedKey::ArrowUp),    _) => Some(Key::UpArrow),
        (WinitKey::Named(NamedKey::ArrowDown),  _) => Some(Key::DownArrow),
        (WinitKey::Named(NamedKey::PageUp),     _) => Some(Key::PageUp),
        (WinitKey::Named(NamedKey::PageDown),   _) => Some(Key::PageDown),
        (WinitKey::Named(NamedKey::Home),       _) => Some(Key::Home),
        (WinitKey::Named(NamedKey::End),        _) => Some(Key::End),
        (WinitKey::Named(NamedKey::Insert),     _) => Some(Key::Insert),
        (WinitKey::Named(NamedKey::Delete),     _) => Some(Key::Delete),
        (WinitKey::Named(NamedKey::Backspace),  _) => Some(Key::Backspace),
        (WinitKey::Named(NamedKey::Space),      _) => Some(Key::Space),
        (WinitKey::Named(NamedKey::Enter), KeyLocation::Standard) => Some(Key::Enter),
        (WinitKey::Named(NamedKey::Enter), KeyLocation::Numpad)   => Some(Key::KeypadEnter),
        (WinitKey::Named(NamedKey::Escape),     _) => Some(Key::Escape),
        (WinitKey::Named(NamedKey::Control), KeyLocation::Left)  => Some(Key::LeftCtrl),
        (WinitKey::Named(NamedKey::Control), KeyLocation::Right) => Some(Key::RightCtrl),
        (WinitKey::Named(NamedKey::Shift),   KeyLocation::Left)  => Some(Key::LeftShift),
        (WinitKey::Named(NamedKey::Shift),   KeyLocation::Right) => Some(Key::RightShift),
        (WinitKey::Named(NamedKey::Alt),     KeyLocation::Left)  => Some(Key::LeftAlt),
        (WinitKey::Named(NamedKey::Alt),     KeyLocation::Right) => Some(Key::RightAlt),
        (WinitKey::Named(NamedKey::Super),   KeyLocation::Left)  => Some(Key::LeftSuper),
        (WinitKey::Named(NamedKey::Super),   KeyLocation::Right) => Some(Key::RightSuper),
        (WinitKey::Named(NamedKey::F1),  _) => Some(Key::F1),
        (WinitKey::Named(NamedKey::F2),  _) => Some(Key::F2),
        (WinitKey::Named(NamedKey::F3),  _) => Some(Key::F3),
        (WinitKey::Named(NamedKey::F4),  _) => Some(Key::F4),
        (WinitKey::Named(NamedKey::F5),  _) => Some(Key::F5),
        (WinitKey::Named(NamedKey::F6),  _) => Some(Key::F6),
        (WinitKey::Named(NamedKey::F7),  _) => Some(Key::F7),
        (WinitKey::Named(NamedKey::F8),  _) => Some(Key::F8),
        (WinitKey::Named(NamedKey::F9),  _) => Some(Key::F9),
        (WinitKey::Named(NamedKey::F10), _) => Some(Key::F10),
        (WinitKey::Named(NamedKey::F11), _) => Some(Key::F11),
        (WinitKey::Named(NamedKey::F12), _) => Some(Key::F12),
        (WinitKey::Character("a"), _) => Some(Key::A),
        (WinitKey::Character("b"), _) => Some(Key::B),
        (WinitKey::Character("c"), _) => Some(Key::C),
        (WinitKey::Character("d"), _) => Some(Key::D),
        (WinitKey::Character("e"), _) => Some(Key::E),
        (WinitKey::Character("f"), _) => Some(Key::F),
        (WinitKey::Character("g"), _) => Some(Key::G),
        (WinitKey::Character("h"), _) => Some(Key::H),
        (WinitKey::Character("i"), _) => Some(Key::I),
        (WinitKey::Character("j"), _) => Some(Key::J),
        (WinitKey::Character("k"), _) => Some(Key::K),
        (WinitKey::Character("l"), _) => Some(Key::L),
        (WinitKey::Character("m"), _) => Some(Key::M),
        (WinitKey::Character("n"), _) => Some(Key::N),
        (WinitKey::Character("o"), _) => Some(Key::O),
        (WinitKey::Character("p"), _) => Some(Key::P),
        (WinitKey::Character("q"), _) => Some(Key::Q),
        (WinitKey::Character("r"), _) => Some(Key::R),
        (WinitKey::Character("s"), _) => Some(Key::S),
        (WinitKey::Character("t"), _) => Some(Key::T),
        (WinitKey::Character("u"), _) => Some(Key::U),
        (WinitKey::Character("v"), _) => Some(Key::V),
        (WinitKey::Character("w"), _) => Some(Key::W),
        (WinitKey::Character("x"), _) => Some(Key::X),
        (WinitKey::Character("y"), _) => Some(Key::Y),
        (WinitKey::Character("z"), _) => Some(Key::Z),
        _ => None,
    }
}
