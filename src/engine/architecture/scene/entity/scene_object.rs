use std::cell::RefCell;
use std::rc::Rc;

use crate::engine::architecture::models::model::Model;
use crate::engine::architecture::scene::node::node::{NodeBehavior, NodeChildren};
use crate::engine::rendering::abstracted::processable::Processable;

pub trait SceneObject: NodeBehavior + NodeChildren + Processable {
    fn model(&self) -> Rc<RefCell<impl Model>>;

    fn process(&mut self);

    fn update(&mut self) {}

    fn delete(&mut self) {
        self.model().borrow_mut().delete();
    }
}
