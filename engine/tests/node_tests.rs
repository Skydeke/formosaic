use formosaic_engine::architecture::scene::node::node::{Node, NodeBehavior};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn node_collect_all_includes_descendants() {
    let root: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));
    let child: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));
    let grandchild: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));

    child
        .borrow_mut()
        .add_child_impl(Rc::clone(&child), Rc::clone(&grandchild));
    root.borrow_mut()
        .add_child_impl(Rc::clone(&root), Rc::clone(&child));

    let all = Node::collect_all(&root);
    assert_eq!(all.len(), 3);
}

#[test]
fn node_collect_of_type_filters() {
    let root: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));
    let child: Rc<RefCell<dyn NodeBehavior>> = Rc::new(RefCell::new(Node::new()));
    root.borrow_mut()
        .add_child_impl(Rc::clone(&root), Rc::clone(&child));

    let nodes = Node::collect_of_type::<Node>(&root);
    assert_eq!(nodes.len(), 2);
}
