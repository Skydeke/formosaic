use formosaic_engine::architecture::models::simple_model::PuzzleParams;
use formosaic_engine::architecture::scene::node::node::{Node, NodeBehavior, NodeChildren};
use formosaic_engine::architecture::scene::node::scenegraph::Scenegraph;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct TestNode {
    node: Node,
    update_count: Rc<Cell<u32>>,
    process_count: Rc<Cell<u32>>,
}

impl TestNode {
    fn new(update_count: Rc<Cell<u32>>, process_count: Rc<Cell<u32>>) -> Self {
        Self {
            node: Node::new(),
            update_count,
            process_count,
        }
    }
}

impl NodeBehavior for TestNode {
    fn get_uuid(&self) -> u32 {
        self.node.get_uuid()
    }

    fn is_hidden(&self) -> bool {
        self.node.is_hidden()
    }

    fn set_hidden(&mut self, hidden: bool) {
        self.node.set_hidden(hidden);
    }

    fn get_name(&self) -> String {
        self.node.get_name()
    }

    fn transform(&self) -> &formosaic_engine::architecture::scene::node::transform::Transform {
        self.node.transform()
    }

    fn transform_mut(
        &mut self,
    ) -> &mut formosaic_engine::architecture::scene::node::transform::Transform {
        self.node.transform_mut()
    }

    fn update(&mut self) {
        self.update_count.set(self.update_count.get() + 1);
    }

    fn process(&mut self) {
        self.process_count.set(self.process_count.get() + 1);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl NodeChildren for TestNode {
    fn add_child_impl(
        &mut self,
        parent: Rc<RefCell<dyn NodeBehavior>>,
        child: Rc<RefCell<dyn NodeBehavior>>,
    ) {
        self.node.add_child_impl(parent, child);
    }

    fn children(&self) -> &[Rc<RefCell<dyn NodeBehavior>>] {
        self.node.children()
    }
}

#[test]
fn deep_hierarchy_collect_all() {
    let root: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));
    let l1: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));
    let l2: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));
    let l3: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));

    l2.borrow_mut().add_child_impl(Rc::clone(&l2), Rc::clone(&l3));
    l1.borrow_mut().add_child_impl(Rc::clone(&l1), Rc::clone(&l2));
    root.borrow_mut().add_child_impl(Rc::clone(&root), Rc::clone(&l1));

    let all = Node::collect_all(&root);
    assert_eq!(all.len(), 4);
}

#[test]
fn multiple_children_at_same_level() {
    let root: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));
    for _ in 0..5 {
        let child: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));
        root.borrow_mut().add_child_impl(Rc::clone(&root), child);
    }
    assert_eq!(root.borrow().children().len(), 5);
}

#[test]
fn collect_all_on_leaf_node() {
    let leaf: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));
    let all = Node::collect_all(&leaf);
    assert_eq!(all.len(), 1);
}

#[test]
fn update_all_depth_first() {
    let update_count = Rc::new(Cell::new(0));
    let process_count = Rc::new(Cell::new(0));

    let mut graph = Scenegraph::new();

    let parent: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(TestNode::new(
        Rc::clone(&update_count),
        Rc::clone(&process_count),
    )));
    let child: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(TestNode::new(
        Rc::clone(&update_count),
        Rc::clone(&process_count),
    )));
    let grandchild: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(TestNode::new(
        Rc::clone(&update_count),
        Rc::clone(&process_count),
    )));

    child
        .borrow_mut()
        .add_child_impl(Rc::clone(&child), Rc::clone(&grandchild));
    parent
        .borrow_mut()
        .add_child_impl(Rc::clone(&parent), Rc::clone(&child));
    graph.add_node(parent);

    graph.update();
    assert_eq!(update_count.get(), 3);

    graph.process();
    assert_eq!(process_count.get(), 3);
}

#[test]
fn update_all_multiple_calls_accumulate() {
    let update_count = Rc::new(Cell::new(0));
    let process_count = Rc::new(Cell::new(0));

    let mut graph = Scenegraph::new();
    let node: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(TestNode::new(
        Rc::clone(&update_count),
        Rc::clone(&process_count),
    )));
    graph.add_node(node);

    graph.update();
    graph.update();
    graph.update();
    assert_eq!(update_count.get(), 3);

    graph.process();
    graph.process();
    assert_eq!(process_count.get(), 2);
}

#[test]
fn hidden_flag_toggles() {
    let node: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));
    assert!(!node.borrow().is_hidden());

    node.borrow_mut().set_hidden(true);
    assert!(node.borrow().is_hidden());

    node.borrow_mut().set_hidden(false);
    assert!(!node.borrow().is_hidden());
}

#[test]
fn node_uuid_is_unique() {
    let a: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));
    let b: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));

    assert_ne!(a.borrow().get_uuid(), b.borrow().get_uuid());
}

#[test]
fn node_as_any_downcasts() {
    let update_count = Rc::new(Cell::new(0));
    let process_count = Rc::new(Cell::new(0));
    let node: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(TestNode::new(
        Rc::clone(&update_count),
        Rc::clone(&process_count),
    )));

    let binding = node.borrow();
    let any_ref = binding.as_any();
    assert!(any_ref.is::<TestNode>());
    assert!(!any_ref.is::<Node>());
}

#[test]
fn node_as_any_mut_downcasts() {
    let update_count = Rc::new(Cell::new(0));
    let process_count = Rc::new(Cell::new(0));
    let node: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(TestNode::new(
        Rc::clone(&update_count),
        Rc::clone(&process_count),
    )));

    let mut binding = node.borrow_mut();
    let any_mut = binding.as_any_mut();
    assert!(any_mut.is::<TestNode>());
}

#[test]
fn scenegraph_new_has_valid_root() {
    let graph = Scenegraph::new();
    let root_name = graph.root.borrow().get_name();
    assert!(root_name.starts_with("Node#"));
}

#[test]
fn scenegraph_update_with_empty_graph() {
    let mut graph = Scenegraph::new();
    graph.update();
    graph.process();
}

#[test]
fn scenegraph_clone() {
    let graph = Scenegraph::new();
    let _clone = graph.clone();
}

#[test]
fn node_default_impl() {
    let node = Node::default();
    assert!(node.get_name().starts_with("Node#"));
    assert!(!node.is_hidden());
}

#[test]
fn puzzle_params_default_for() {
    let params = PuzzleParams::default_for(1.0);
    assert!((params.entity_scale - 0.005).abs() < 1e-6);
    assert!((params.orbit_distance - 3.0).abs() < 1e-6);
    assert!((params.min_disp - 3.0).abs() < 1e-6);
    assert!((params.max_disp - 15.0).abs() < 1e-6);
    assert!((params.model_space_radius - 1.0).abs() < 1e-6);
}

#[test]
fn puzzle_params_clone_copy() {
    let params = PuzzleParams::default_for(2.0);
    let params2 = params;
    assert_eq!(params.entity_scale, params2.entity_scale);
}
