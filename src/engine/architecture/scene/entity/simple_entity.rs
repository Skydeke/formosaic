use crate::engine::architecture::models::mesh::Mesh;
use crate::engine::architecture::models::model::Model;
use crate::engine::architecture::models::simple_model::SimpleModel;
use crate::engine::architecture::scene::entity::scene_object::SceneObject;
use crate::engine::architecture::scene::node::node::{NodeBehavior, NodeChildren};
use crate::engine::architecture::scene::node::transform::Transform;
use crate::engine::rendering::abstracted::processable::Processable;
use crate::opengl::constants::data_type::DataType;
use crate::opengl::constants::render_mode::RenderMode;
use crate::opengl::constants::vbo_usage::VboUsage;
use crate::opengl::objects::attribute::Attribute;
use crate::opengl::objects::data_buffer::DataBuffer;
use crate::opengl::objects::index_buffer::IndexBuffer;
use crate::opengl::objects::vao::Vao;
use rand::Rng;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

pub struct SimpleEntity {
    // Node properties
    uuid: u32,
    hidden: bool,
    debug_name: Option<String>,
    children: Vec<Rc<RefCell<dyn NodeBehavior>>>,
    transform: Transform,

    // SceneObject-specific properties
    model: Rc<RefCell<SimpleModel>>,
}

impl SimpleEntity {
    pub fn new() -> Self {
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
        let model = SimpleModel::with_bounds(vec![mesh], RenderMode::Triangles);

        Self {
            uuid: rand::rng().random(),
            hidden: false,
            debug_name: Some("SimpleTriangle".to_string()),
            children: Vec::new(),
            transform: Transform::new(),
            model: Rc::new(RefCell::new(model)),
        }
    }
}

// Implement NodeBehavior
impl NodeBehavior for SimpleEntity {
    fn get_uuid(&self) -> u32 {
        self.uuid
    }

    fn is_hidden(&self) -> bool {
        self.hidden
    }

    fn set_hidden(&mut self, hidden: bool) {
        self.hidden = hidden;
    }

    fn get_name(&self) -> String {
        if let Some(name) = &self.debug_name {
            name.clone()
        } else {
            format!("SimpleEntity#{}", self.uuid)
        }
    }

    fn transform(&self) -> &Transform {
        &self.transform
    }

    fn transform_mut(&mut self) -> &mut Transform {
        &mut self.transform
    }

    fn update(&mut self) {}

    fn process(&mut self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// Implement NodeChildren
impl NodeChildren for SimpleEntity {
    fn add_child_impl(&mut self, child: Rc<RefCell<dyn NodeBehavior>>) {
        self.children.push(child);
    }

    fn get_children_impl(&self) -> Vec<Rc<RefCell<dyn NodeBehavior>>> {
        self.children.clone()
    }
}

// Implement Processable trait
impl Processable for SimpleEntity {
    fn process(&mut self) {}

    fn get_model(&self) -> &impl Model {
        unsafe { &*(self.model.as_ref().as_ptr()) }
        // TODO:  ⚠️ This is unsafe and not recommended!
    }
}

// Implement SceneObject trait
impl SceneObject for SimpleEntity {
    fn model(&self) -> Rc<RefCell<impl Model>> {
        self.model.clone()
    }

    fn process(&mut self) {}
}
