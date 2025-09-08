use crate::engine::rendering::instances::camera::camera::Camera;

/// Trait representing a camera controller
pub trait CameraController {
    fn control(&mut self, camera: &mut Camera);
}

/// No-op camera controller
pub struct NoneController;

impl NoneController {
    pub fn new() -> Self {
        Self
    }
}

impl CameraController for NoneController {
    fn control(&mut self, _camera: &mut Camera) {
        // Do nothing
    }
}
