use crate::engine::rendering::instances::camera::{
    camera::Camera, camera_controller::CameraController,
};

pub struct CameraControllers {
    controllers: Vec<Box<dyn CameraController>>,
}

impl CameraControllers {
    pub fn new(controllers: Vec<Box<dyn CameraController>>) -> Self {
        Self { controllers }
    }
}

impl CameraController for CameraControllers {
    fn control(&mut self, camera: &mut Camera) {
        for controller in &mut self.controllers {
            controller.control(camera);
        }
    }

    fn handle_event(&mut self, event: &crate::input::Event, width: f32, height: f32) {
        for controller in &mut self.controllers {
            controller.handle_event(event, width, height);
        }
    }
}
