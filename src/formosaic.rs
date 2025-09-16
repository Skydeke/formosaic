use cgmath::{Deg, One, Quaternion, Rotation3, Vector3};
use std::{cell::RefCell, rc::Rc};

use crate::{
    engine::architecture::{
        models::{mesh::Mesh, model_loader::ModelLoader, simple_model::SimpleModel},
        scene::{
            entity::simple_entity::SimpleEntity, node::node::NodeBehavior,
            scene_context::SceneContext,
        },
    },
    input::Event,
    opengl::{
        constants::{data_type::DataType, render_mode::RenderMode, vbo_usage::VboUsage},
        objects::{
            attribute::Attribute, data_buffer::DataBuffer, index_buffer::IndexBuffer, vao::Vao,
        },
    },
    EngineKey as Key,
};

pub trait Application {
    fn on_init(&mut self, context: &mut SceneContext);
    fn on_update(&mut self, delta_time: f32, context: &mut SceneContext);
    fn on_event(&mut self, event: &Event, context: &mut SceneContext);
}

pub struct Formosaic {
    mouse_dragging: bool,
    drag_start_x: f32,
    drag_start_y: f32,
    time: f32,
    simple_triangle: Option<Rc<RefCell<dyn NodeBehavior>>>,
    start_rotation: Option<Quaternion<f32>>,
}

impl Formosaic {
    pub fn new() -> Self {
        Self {
            mouse_dragging: false,
            drag_start_x: 0.0,
            drag_start_y: 0.0,
            time: 0.0,
            simple_triangle: None,
            start_rotation: None,
        }
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

        let positions: [f32; 9] = [
            -0.5, -0.5, 0.0, // Left
            0.5, -0.5, 0.0, // Top
            0.0, 0.5, 0.0, // Right
        ];

        let colors: [f32; 9] = [
            1.0, 0.0, 0.0, // Red
            0.0, 1.0, 0.0, // Green
            0.0, 0.0, 1.0, // Blue
        ];
        let indices: [i32; 3] = [0, 1, 2];

        let mut pos_buffer = DataBuffer::new(VboUsage::StaticDraw);
        pos_buffer.store_float(0, &positions);
        let mut color_buffer = DataBuffer::new(VboUsage::StaticDraw);
        color_buffer.store_float(0, &colors);
        let mut indices_buffer = IndexBuffer::new(VboUsage::StaticDraw);
        indices_buffer.store_int(0, &indices);

        let mut vao = Vao::create();
        // Position attribute -> VBO 0
        let pos_attr = Attribute::of(0, 3, DataType::Float, false);
        vao.load_data_buffer(Rc::new(pos_buffer), &[pos_attr]);
        // Color attribute -> VBO 1
        let color_attr = Attribute::of(1, 3, DataType::Float, false);
        vao.load_data_buffer(Rc::new(color_buffer), &[color_attr]);
        vao.load_index_buffer(Rc::new(indices_buffer), true);

        let mesh = Mesh::from_vao(vao);
        let model = Rc::new(RefCell::new(SimpleModel::with_bounds(
            vec![mesh],
            RenderMode::Triangles,
        )));

        let path = "models/Cactus/cactus.fbx";
        // Load the model using your ModelLoader
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
                .set_scale(Vector3::new(0.005, 0.005, 0.005));
            scene.add_node(triangle.clone());
            self.simple_triangle = Some(triangle);

            let e2 = SimpleEntity::new(model.clone());
            let triangle2 = Rc::new(RefCell::new(e2));
            triangle2.borrow_mut().transform_mut().position = Vector3::new(1.0, 0.0, 0.0);
            scene.add_node(triangle2.clone());
        }
    }

    fn on_update(&mut self, delta_time: f32, _context: &mut SceneContext) {
        self.time += delta_time;

        // Automatic rotation when not dragging
        if !self.mouse_dragging {
            if let Some(triangle) = &self.simple_triangle {
                triangle
                    .borrow_mut()
                    .transform_mut()
                    .add_rotation_euler_local(0.0, 20.0 * delta_time, 0.0);
            }
        }
    }

    fn on_event(&mut self, event: &Event, _context: &mut SceneContext) {
        match event {
            Event::MouseDown { x, y, .. } | Event::TouchDown { x, y, .. } => {
                self.mouse_dragging = true;
                self.drag_start_x = *x;
                self.drag_start_y = *y;

                // store starting rotation for relative drag
                if let Some(triangle) = &self.simple_triangle {
                    self.start_rotation = Some(triangle.borrow().transform().rotation);
                }
            }

            Event::MouseUp { .. } | Event::TouchUp { .. } => {
                self.mouse_dragging = false;
            }

            Event::MouseMove {
                x,
                y,
                width,
                height,
            }
            | Event::TouchMove {
                x,
                y,
                width,
                height,
                ..
            } => {
                if self.mouse_dragging {
                    let dx = (x - self.drag_start_x) / width;
                    let dy = (y - self.drag_start_y) / height;

                    if let Some(triangle) = &self.simple_triangle {
                        let mut tri = triangle.borrow_mut();
                        let t = tri.transform_mut();
                        if let Some(start_rot) = self.start_rotation {
                            // Apply rotation relative to starting rotation
                            let rot_x = Quaternion::from_angle_x(Deg(dy * 180.0));
                            let rot_y = Quaternion::from_angle_y(Deg(dx * 360.0));
                            t.rotation = rot_y * rot_x * start_rot;
                        }
                    }
                }
            }

            Event::KeyDown { key } => {
                if let Key::R = key {
                    if let Some(triangle) = &self.simple_triangle {
                        triangle.borrow_mut().transform_mut().rotation = Quaternion::one();
                    }
                }
            }
            _ => {}
        }
    }
}
