use crate::engine::{
    architecture::scene::node::transform::Transform,
    rendering::instances::camera::camera_controller::CameraController,
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
    fn control(&mut self, transform: &mut Transform) {
        for controller in &mut self.controllers {
            controller.control(transform);
        }
    }

    fn handle_event(&mut self, event: &crate::input::Event, width: f32, height: f32) {
        for controller in &mut self.controllers {
            controller.handle_event(event, width, height);
        }
    }
}
