use cgmath::Vector3;
use std::{cell::RefCell, rc::Rc};

use crate::{
    engine::{
        architecture::{
            models::{model_loader::ModelLoader, simple_model::SimpleModel},
            scene::{
                entity::simple_entity::SimpleEntity,
                node::node::{NodeBehavior, NodeChildren},
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
    t: Option<Rc<RefCell<SimpleEntity>>>,
}

impl Formosaic {
    pub fn new() -> Self {
        Self { time: 0.0, t: None }
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

        // Create triangle entity and add to scene
        if let Some(scene) = context.scene() {
            let e1 = SimpleEntity::new(cactus_model.clone());
            let triangle = Rc::new(RefCell::new(e1));
            triangle
                .borrow_mut()
                .transform_mut()
                .add_rotation_euler_world(0.0, 180.0, 0.0);
            triangle
                .borrow_mut()
                .transform_mut()
                .set_scale(Vector3::new(0.005, 0.005, 0.005));
            triangle
                .borrow_mut()
                .transform_mut()
                .set_position(Vector3::new(1.0, 0.0, 0.0));
            scene.add_node(triangle.clone());
            self.t = Some(triangle.clone());

            let e2 = SimpleEntity::new(cactus_model.clone());
            let triangle2 = Rc::new(RefCell::new(e2));
            triangle2
                .borrow_mut()
                .transform_mut()
                .add_rotation_euler_world(0.0, -90.0, 0.0);
            triangle2
                .borrow_mut()
                .transform_mut()
                .set_scale(Vector3::new(1.0, 1.0, 1.0));
            triangle2
                .borrow_mut()
                .transform_mut()
                .set_position(Vector3::new(1.0, -1.0, 0.0));
            triangle
                .borrow_mut()
                .add_child_impl(triangle.clone(), triangle2.clone());

            let e3 = SimpleEntity::new(cactus_model.clone());
            let triangle3 = Rc::new(RefCell::new(e3));
            triangle3
                .borrow_mut()
                .transform_mut()
                .add_rotation_euler_world(0.0, 90.0, 0.0);
            triangle3
                .borrow_mut()
                .transform_mut()
                .set_scale(Vector3::new(1.0, 1.0, 1.0));
            triangle3
                .borrow_mut()
                .transform_mut()
                .set_position(Vector3::new(1.0, -1.0, 0.0));
            triangle2
                .borrow_mut()
                .add_child_impl(triangle2.clone(), triangle3.clone());

            if let Some(camera) = context.camera() {
                let centeroid = triangle.borrow().centroid();
                let orbit_controller = Box::new(OrbitController::new(centeroid, 3.0));
                camera.borrow_mut().set_controller(Some(orbit_controller));
            }
        }
    }

    fn on_update(&mut self, delta_time: f32, _context: &mut SceneContext) {
        self.time += delta_time;

        self.t
            .as_mut()
            .expect("Nothing.")
            .borrow_mut()
            .transform_mut()
            .add_rotation_euler_world(0.0, 90.0 * delta_time, 0.0);
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
