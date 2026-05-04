use formosaic_engine::architecture::scene::node::node::{Node, NodeBehavior, NodeChildren};
use formosaic_engine::architecture::scene::node::scenegraph::Scenegraph;
use std::cell::Cell;
use std::cell::RefCell;
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

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
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
fn scenegraph_collects_nodes_of_type() {
    let graph = Scenegraph::new();

    let node_a: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));
    let node_b: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));
    graph.add_node(Rc::clone(&node_a));
    graph.add_node(Rc::clone(&node_b));

    let nodes = graph.collect_nodes_of_type::<Node>();
    assert_eq!(nodes.len(), 3, "root + 2 children should be collected");
}

#[test]
fn scenegraph_clear_removes_children() {
    let graph = Scenegraph::new();

    let node: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));
    graph.add_node(node);
    assert_eq!(graph.collect_nodes_of_type::<Node>().len(), 2);

    graph.clear();
    assert_eq!(graph.collect_nodes_of_type::<Node>().len(), 1);
}

#[test]
fn scenegraph_update_and_process_recurse() {
    let mut graph = Scenegraph::new();
    let update_count = Rc::new(Cell::new(0));
    let process_count = Rc::new(Cell::new(0));

    let parent: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(TestNode::new(
        Rc::clone(&update_count),
        Rc::clone(&process_count),
    )));
    let child: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(TestNode::new(
        Rc::clone(&update_count),
        Rc::clone(&process_count),
    )));

    parent
        .borrow_mut()
        .add_child_impl(Rc::clone(&parent), Rc::clone(&child));
    graph.add_node(parent);

    graph.update();
    graph.process();

    assert_eq!(update_count.get(), 2);
    assert_eq!(process_count.get(), 2);
}
