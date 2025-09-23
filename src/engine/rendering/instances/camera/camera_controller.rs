use crate::engine::architecture::scene::node::transform::Transform;

/// Trait representing a camera controller
pub trait CameraController {
    fn control(&mut self, transform: &mut Transform);
    fn handle_event(&mut self, event: &crate::input::Event, width: f32, height: f32);
}

/// No-op camera controller
pub struct NoneController;

impl NoneController {
    pub fn new() -> Self {
        Self
    }
}

impl CameraController for NoneController {
    fn control(&mut self, _transform: &mut Transform) {
        // Do nothing
    }

    fn handle_event(&mut self, _event: &crate::input::Event, _width: f32, _height: f32) {}
}
