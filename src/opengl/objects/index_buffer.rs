use gl::types::*;
use std::{cell::RefCell, rc::Rc};

use crate::opengl::{
    constants::{vbo_access::VboAccess, vbo_target::VboTarget, vbo_usage::VboUsage},
    objects::{ivbo::IVbo, vao::Vao, vbo::Vbo},
};

pub struct IndexBuffer {
    vbo: Rc<RefCell<Vbo>>, // Changed to RefCell
}

impl IndexBuffer {
    /// Null index buffer
    pub fn null() -> Self {
        Self {
            vbo: Rc::new(RefCell::new(Vbo::NULL_INDEX)),
        }
    }

    /// Create a new index buffer with the given usage
    pub fn new(usage: VboUsage) -> Self {
        Self {
            vbo: Rc::new(RefCell::new(Vbo::create(
                VboTarget::ElementArrayBuffer,
                usage,
            ))),
        }
    }

    pub fn load_static(data: &[i32]) -> Self {
        let mut buffer = Self::new(VboUsage::StaticDraw);
        buffer.allocate_int(data.len());
        buffer.store_int(0, data);
        buffer
    }

    pub fn allocate_int(&mut self, size: usize) {
        self.vbo.borrow_mut().allocate_int(size);
    }

    pub fn store_int(&mut self, pointer: usize, data: &[i32]) {
        self.vbo.borrow_mut().store_int(pointer, data);
    }

    pub fn allocate_byte(&mut self, size: usize) {
        self.vbo.borrow_mut().allocate_data(size);
    }

    pub fn store_byte(&mut self, pointer: usize, data: &[u8]) {
        self.vbo.borrow_mut().store_data(pointer, data);
    }

    pub fn map(&self, access: VboAccess) -> *mut u8 {
        self.vbo.borrow_mut().map(access)
    }

    pub fn unmap(&self) {
        self.vbo.borrow_mut().unmap();
    }
}

impl IVbo for IndexBuffer {
    fn get_size(&self) -> usize {
        self.vbo.borrow_mut().get_size()
    }

    fn bind(&self) {
        self.vbo.borrow_mut().bind();
    }

    fn bind_to_vao(&self, vao: &Vao) {
        self.vbo.borrow_mut().bind_to_vao(vao);
    }

    fn unbind(&self) {
        self.vbo.borrow_mut().unbind();
    }

    fn delete(&self) {
        self.vbo.borrow_mut().delete();
    }
}
