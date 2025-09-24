use rand::Rng;
use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

use crate::engine::architecture::scene::node::transform::Transform;

pub trait NodeBehavior: NodeChildren {
    fn get_uuid(&self) -> u32;
    fn is_hidden(&self) -> bool;
    fn set_hidden(&mut self, hidden: bool);
    fn get_name(&self) -> String;
    fn transform(&self) -> &Transform;
    fn transform_mut(&mut self) -> &mut Transform;
    fn update(&mut self) {}
    fn process(&mut self) {}
    fn cleanup(&mut self) {}
    fn as_any(&self) -> &dyn Any;
}

pub struct Node {
    pub uuid: u32,
    hidden: bool,
    debug_name: Option<String>,
    parent: Option<Weak<RefCell<dyn NodeBehavior>>>,
    children: Vec<Rc<RefCell<dyn NodeBehavior>>>,
    transform: Transform,
}

impl Node {
    pub fn new() -> Self {
        Self {
            uuid: rand::rng().random(),
            hidden: false,
            debug_name: None,
            parent: None,
            children: Vec::new(),
            transform: Transform::new(),
        }
    }

    pub fn collect_all(node: &Rc<RefCell<dyn NodeBehavior>>) -> Vec<Rc<RefCell<dyn NodeBehavior>>> {
        let mut ret = vec![Rc::clone(node)];
        ret.extend(node.borrow().get_children_impl());
        for child in node.borrow().get_children_impl() {
            ret.extend(Self::collect_all(&child));
        }
        ret
    }

    pub fn collect_of_type<T: NodeBehavior + 'static>(
        node: &Rc<RefCell<dyn NodeBehavior>>,
    ) -> Vec<Rc<RefCell<dyn NodeBehavior>>> {
        let mut ret = Vec::new();

        // Check if current node is of the type we're looking for
        if node.borrow().as_any().is::<T>() {
            ret.push(Rc::clone(node));
        }

        // Recursively check children
        for child in node.borrow().get_children_impl() {
            ret.extend(Self::collect_of_type::<T>(&child));
        }

        ret
    }

    pub fn update_all(node: &Rc<RefCell<dyn NodeBehavior>>) {
        node.borrow_mut().update();

        for child in node.borrow().get_children_impl() {
            Self::update_all(&child);
        }
    }

    pub fn process_all(node: &Rc<RefCell<dyn NodeBehavior>>) {
        node.borrow_mut().process();

        for child in node.borrow().get_children_impl() {
            Self::process_all(&child);
        }
    }
}

impl NodeBehavior for Node {
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
            format!("Node#{}", self.uuid)
        }
    }

    fn transform(&self) -> &Transform {
        &self.transform
    }

    fn transform_mut(&mut self) -> &mut Transform {
        &mut self.transform
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// Extension trait for managing children - needs to be implemented by NodeBehavior implementors
pub trait NodeChildren {
    fn add_child_impl(
        &mut self,
        parent: Rc<RefCell<dyn NodeBehavior>>,
        child: Rc<RefCell<dyn NodeBehavior>>,
    );
    fn get_children_impl(&self) -> Vec<Rc<RefCell<dyn NodeBehavior>>>;
}

impl NodeChildren for Node {
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

    fn get_children_impl(&self) -> Vec<Rc<RefCell<dyn NodeBehavior>>> {
        self.children.clone()
    }
}
