use std::cell::RefCell;
use std::rc::Rc;

use crate::engine::architecture::scene::node::node::{Node, NodeBehavior};

#[derive(Clone)]
pub struct Scenegraph {
    pub root: Rc<RefCell<Node>>,
}

impl Scenegraph {
    pub fn new() -> Self {
        Self {
            root: Rc::new(RefCell::new(Node::new())),
        }
    }

    pub fn add_node(&self, node: Rc<RefCell<dyn NodeBehavior>>) {
        // Cast root to NodeBehavior for consistent interface
        let root_as_behavior: Rc<RefCell<dyn NodeBehavior>> = self.root.clone();
        root_as_behavior
            .borrow_mut()
            .add_child_impl(root_as_behavior.clone(), node);
    }

    pub fn update(&mut self) {
        let root_as_behavior: Rc<RefCell<dyn NodeBehavior>> = self.root.clone();
        Node::update_all(&root_as_behavior);
    }

    pub fn process(&mut self) {
        let root_as_behavior: Rc<RefCell<dyn NodeBehavior>> = self.root.clone();
        Node::process_all(&root_as_behavior);
    }

    pub fn collect_nodes_of_type<T: NodeBehavior + 'static>(
        &self,
    ) -> Vec<Rc<RefCell<dyn NodeBehavior>>> {
        let root_as_behavior: Rc<RefCell<dyn NodeBehavior>> = self.root.clone();
        Node::collect_of_type::<T>(&root_as_behavior)
    }
}
