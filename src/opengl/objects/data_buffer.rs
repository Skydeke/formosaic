use std::cell::RefCell;
use std::rc::Rc;

use crate::opengl::{
    constants::{vbo_access::VboAccess, vbo_target::VboTarget, vbo_usage::VboUsage},
    objects::{ivbo::IVbo, vao::Vao, vbo::Vbo},
};

pub struct DataBuffer {
    vbo: Rc<RefCell<Vbo>>, // Changed to RefCell
}

impl DataBuffer {
    pub fn null() -> Self {
        Self {
            vbo: Rc::new(RefCell::new(Vbo::NULL_ARRAY)),
        }
    }

    pub fn new(usage: VboUsage) -> Self {
        Self {
            vbo: Rc::new(RefCell::new(Vbo::create(VboTarget::ArrayBuffer, usage))),
        }
    }

    // Add the missing load_static method
    pub fn load_static(data: &[f32]) -> Self {
        let mut buffer = Self::new(VboUsage::StaticDraw);
        buffer.allocate_float(data.len());
        buffer.store_float(0, data);
        buffer
    }

    pub fn allocate_float(&mut self, size: usize) {
        self.vbo.borrow_mut().allocate_float(size);
    }

    pub fn allocate_int(&mut self, size: usize) {
        self.vbo.borrow_mut().allocate_int(size);
    }

    pub fn allocate_data(&mut self, size: usize) {
        self.vbo.borrow_mut().allocate_data(size);
    }

    pub fn store_float(&mut self, pointer: usize, data: &[f32]) {
        self.vbo.borrow_mut().store_float(pointer, data);
    }

    pub fn store_int(&mut self, pointer: usize, data: &[i32]) {
        self.vbo.borrow_mut().store_int(pointer, data);
    }

    pub fn store_byte(&mut self, pointer: usize, data: &[u8]) {
        self.vbo.borrow_mut().store_data(pointer, data);
    }

    pub fn map(&self, access: VboAccess) -> *mut u8 {
        self.vbo.borrow().map(access)
    }

    pub fn unmap(&self) {
        self.vbo.borrow().unmap();
    }
}

impl IVbo for DataBuffer {
    fn get_size(&self) -> usize {
        self.vbo.borrow().get_size()
    }

    fn bind(&self) {
        self.vbo.borrow().bind();
    }

    fn bind_to_vao(&self, vao: &Vao) {
        self.vbo.borrow().bind_to_vao(vao);
    }

    fn unbind(&self) {
        self.vbo.borrow().unbind();
    }

    fn delete(&self) {
        self.vbo.borrow_mut().delete();
    }
}
