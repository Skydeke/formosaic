use crate::architecture::models::model::Model;
use crate::architecture::models::simple_model::SimpleModel;
use crate::architecture::scene::entity::scene_object::SceneObject;
use crate::architecture::scene::node::node::{NodeBehavior, NodeChildren};
use crate::architecture::scene::node::transform::Transform;
use crate::rendering::abstracted::processable::Processable;
use cgmath::{Vector3, Vector4};
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

    pub fn centroid(&self) -> Vector3<f32> {
        let model = self.model.borrow();
        let center = model
            .visual_center()
            .or_else(|| model.centroid())
            .unwrap_or(Vector3::new(0.0, 0.0, 0.0));
        let center4 = Vector4::new(center.x, center.y, center.z, 1.0);
        (self.transform.get_matrix() * center4).truncate()
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

    fn update(&mut self) {
        // Update all children recursively
        for child in &self.children {
            child.borrow_mut().update();
        }
    }

    fn process(&mut self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// Implement NodeChildren
impl NodeChildren for SimpleEntity {
    fn add_child_impl(
        &mut self,
        parent: Rc<RefCell<dyn NodeBehavior>>,
        child: Rc<RefCell<dyn NodeBehavior>>,
    ) {
        // Set this node as the parent of the child
        child
            .borrow_mut()
            .transform_mut()
            .set_parent(Some(Rc::downgrade(&parent)));

        // Add to children list
        self.children.push(child);
    }

    fn children(&self) -> &[Rc<RefCell<dyn NodeBehavior>>] {
        &self.children
    }
}

impl Processable for SimpleEntity {
    fn process(&mut self) {}
}

// Implement SceneObject trait
impl SceneObject for SimpleEntity {
    fn model(&self) -> Rc<RefCell<impl Model>> {
        self.model.clone()
    }

    fn process(&mut self) {}
}
