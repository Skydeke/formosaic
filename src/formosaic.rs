use cgmath::{One, Quaternion, Vector3};
use std::{cell::RefCell, rc::Rc};

use crate::{
    engine::{
        architecture::{
            models::{model_loader::ModelLoader, simple_model::SimpleModel},
            scene::{
                entity::simple_entity::SimpleEntity, node::node::NodeBehavior,
                scene_context::SceneContext,
            },
        },
        rendering::instances::camera::orbit_controller::OrbitController,
    },
    input::Event,
};

pub trait Application {
    fn on_init(&mut self, context: &mut SceneContext);
    fn on_update(&mut self, delta_time: f32, context: &mut SceneContext);
    fn on_event(&mut self, event: &Event, context: &mut SceneContext);
}

pub struct Formosaic {
    time: f32,
}

impl Formosaic {
    pub fn new() -> Self {
        Self { time: 0.0 }
    }
}

impl Default for Formosaic {
    fn default() -> Self {
        Self::new()
    }
}

impl Application for Formosaic {
    fn on_init(&mut self, context: &mut SceneContext) {
        log::info!("Initializing scene...");
        let path = "models/Cactus/cactus.fbx";
        let cactus_model: Rc<RefCell<SimpleModel>> = ModelLoader::load(path);

        // Set camera position
        if let Some(camera) = context.camera() {
            camera.borrow_mut().get_transform_mut().position = Vector3::new(0.0, 0.0, 3.0);
        }

        // Create triangle entity and add to scene
        if let Some(scene) = context.scene() {
            let e1 = SimpleEntity::new(cactus_model.clone());
            let triangle = Rc::new(RefCell::new(e1));
            triangle
                .borrow_mut()
                .transform_mut()
                .set_rotation(Quaternion::one());
            triangle
                .borrow_mut()
                .transform_mut()
                .set_scale(Vector3::new(0.005, 0.005, 0.005));
            triangle
                .borrow_mut()
                .transform_mut()
                .set_position(Vector3::new(0.0, 0.0, 0.0));
            scene.add_node(triangle.clone());

            if let Some(camera) = context.camera() {
                let centeroid = triangle.borrow_mut().centroid();
                let c = triangle.borrow_mut().transform_mut().position + centeroid;
                let orbit_controller = Box::new(OrbitController::new(c, 3.0));
                camera.borrow_mut().set_controller(Some(orbit_controller));
            }
        }
    }

    fn on_update(&mut self, delta_time: f32, _context: &mut SceneContext) {
        self.time += delta_time;
    }

    fn on_event(&mut self, event: &Event, context: &mut SceneContext) {
        if let Some(camera) = context.camera() {
            let width = camera.borrow().resolution.x as f32;
            let height = camera.borrow().resolution.y as f32;

            camera
                .borrow_mut()
                .controller
                .handle_event(event, width, height);
        }
    }
}
