use crate::engine::architecture::models::model::Model;
use crate::engine::architecture::models::simple_model::SimpleModel;
use crate::engine::architecture::scene::entity::scene_object::SceneObject;
use crate::engine::architecture::scene::node::node::{NodeBehavior, NodeChildren};
use crate::engine::architecture::scene::node::transform::Transform;
use crate::engine::rendering::abstracted::processable::Processable;
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
    pub fn new(model: Rc<RefCell<SimpleModel>>) -> Self {
        Self {
            uuid: rand::rng().random(),
            hidden: false,
            debug_name: Some("SimpleTriangle".to_string()),
            children: Vec::new(),
            transform: Transform::new(),
            model,
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
