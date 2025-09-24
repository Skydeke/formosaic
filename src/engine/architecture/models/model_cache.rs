use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::engine::architecture::models::simple_model::SimpleModel;

/// Model cache (cleared when GL context is destroyed)
pub struct ModelCache;

thread_local! {
    static STORAGE: RefCell<HashMap<String, Rc<RefCell<SimpleModel>>>> =
        RefCell::new(HashMap::new());
}

impl ModelCache {
    pub fn get(path: &str) -> Option<Rc<RefCell<SimpleModel>>> {
        STORAGE.with(|storage| storage.borrow().get(path).cloned())
    }

    pub fn insert(path: String, model: Rc<RefCell<SimpleModel>>) {
        STORAGE.with(|storage| {
            storage.borrow_mut().insert(path, model);
        });
    }

    pub fn clear() {
        STORAGE.with(|storage| {
            storage.borrow_mut().clear();
        });
    }
}

