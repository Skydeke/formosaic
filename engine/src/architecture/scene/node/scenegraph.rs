use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use crate::architecture::scene::node::node::{Node, NodeBehavior};

type NodeList = Vec<Rc<RefCell<dyn NodeBehavior>>>;
type CacheEntry = (u64, NodeList);

pub struct Scenegraph {
    pub root: Rc<RefCell<Node>>,
    generation: Cell<u64>,
    node_cache: RefCell<HashMap<TypeId, CacheEntry>>,
}

impl Clone for Scenegraph {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
            generation: Cell::new(self.generation.get()),
            node_cache: RefCell::new(HashMap::new()),
        }
    }
}

impl Scenegraph {
    pub fn new() -> Self {
        Self {
            root: Rc::new(RefCell::new(Node::new())),
            generation: Cell::new(0),
            node_cache: RefCell::new(HashMap::new()),
        }
    }

    /// Bump the generation and clear the cache.
    fn invalidate_cache(&self) {
        self.generation.set(self.generation.get() + 1);
        self.node_cache.borrow_mut().clear();
    }

    pub fn add_node(&self, node: Rc<RefCell<dyn NodeBehavior>>) {
        let root_as_behavior: Rc<RefCell<dyn NodeBehavior>> = self.root.clone();
        root_as_behavior
            .borrow_mut()
            .add_child_impl(root_as_behavior.clone(), node);
        self.invalidate_cache();
    }

    /// Remove all child nodes from the scene graph root.
    pub fn clear(&self) {
        use crate::architecture::scene::node::node::Node;
        let new_root = Node::new();
        *self.root.borrow_mut() = new_root;
        self.invalidate_cache();
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
        let tid = TypeId::of::<T>();
        let gen = self.generation.get();

        if let Some(cached) = self.node_cache.borrow().get(&tid) {
            if cached.0 == gen {
                return cached.1.clone();
            }
        }

        let root_as_behavior: Rc<RefCell<dyn NodeBehavior>> = self.root.clone();
        let nodes = Node::collect_of_type::<T>(&root_as_behavior);
        self.node_cache
            .borrow_mut()
            .insert(tid, (gen, nodes.clone()));
        nodes
    }
}
