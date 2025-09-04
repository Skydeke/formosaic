use gl::types::*;
use std::cell::{Cell, RefCell};
use std::ptr;
use std::rc::Rc;

use crate::opengl::constants::data_type::DataType;
use crate::opengl::constants::vbo_access::VboAccess;
use crate::opengl::constants::vbo_target::VboTarget;
use crate::opengl::constants::vbo_usage::VboUsage;

pub struct Vbo {
    id: GLuint,
    target: VboTarget,
    usage: VboUsage,
    size: usize,
    deleted: bool,
    dt: Option<DataType>,
}

thread_local! {
    static BOUND_VBO: Cell<GLuint> = Cell::new(0);
}

impl Vbo {
    pub const NULL_ARRAY: Vbo = Vbo::new_raw(0, VboTarget::ArrayBuffer, VboUsage::StaticDraw);
    pub const NULL_INDEX: Vbo =
        Vbo::new_raw(0, VboTarget::ElementArrayBuffer, VboUsage::StaticDraw);

    pub const fn new_raw(id: GLuint, target: VboTarget, usage: VboUsage) -> Self {
        Self {
            id,
            target,
            usage,
            size: 0,
            deleted: false,
            dt: None,
        }
    }

    pub fn create(target: VboTarget, usage: VboUsage) -> Self {
        let mut id = 0;
        unsafe { gl::GenBuffers(1, &mut id) };
        Self::new_raw(id, target, usage)
    }

    fn print_reallocation(&self, data_type: &str, new_size: usize) {
        log::warn!(
            "Vbo {} reallocates {} buffer again, new limit: {}, last limit: {}",
            self.id,
            data_type,
            new_size,
            self.size
        );
    }

    pub fn allocate_float(&mut self, size: usize) {
        self.allocate_data(size * DataType::Float.bytes());
        self.dt = Some(DataType::Float);
    }

    pub fn allocate_int(&mut self, size: usize) {
        self.allocate_data(size * DataType::Int.bytes());
        self.dt = Some(DataType::Int);
    }

    pub fn allocate_data(&mut self, size_in_bytes: usize) {
        self.bind();
        unsafe {
            gl::BufferData(
                self.target.value(),
                size_in_bytes as isize,
                ptr::null(),
                self.usage.value(),
            );
        }
        self.size = size_in_bytes;
    }

    pub fn store_float(&mut self, pointer: usize, data: &[f32]) {
        self.store_data_generic(pointer, data, DataType::Float);
    }

    pub fn store_int(&mut self, pointer: usize, data: &[i32]) {
        self.store_data_generic(pointer, data, DataType::Int);
    }

    pub fn store_data(&mut self, pointer: usize, data: &[u8]) {
        self.store_data_generic(pointer, data, DataType::UByte);
    }

    fn store_data_generic<T>(&mut self, pointer: usize, data: &[T], dt: DataType) {
        let data_size = data.len() * dt.bytes();
        self.dt = Some(dt);
        if pointer + data_size > self.size {
            self.print_reallocation(&format!("{:?}", dt), data_size);
            self.allocate_data(data_size);
        }
        self.bind();
        unsafe {
            gl::BufferSubData(
                self.target.value(),
                pointer as isize,
                data_size as isize,
                data.as_ptr() as *const _,
            );
        }
    }

    pub fn get_size(&self) -> usize {
        if let Some(dt) = self.dt {
            self.size / dt.bytes()
        } else {
            self.size
        }
    }

    pub fn bind_to_vao(&self, _vao: &crate::opengl::objects::vao::Vao) {
        self.bind();
    }

    pub fn map(&self, access: VboAccess) -> *mut u8 {
        self.bind();
        unsafe { gl::MapBuffer(self.target.value(), access.value()) as *mut u8 }
    }

    pub fn unmap(&self) {
        self.bind();
        unsafe { gl::UnmapBuffer(self.target.value()) };
    }

    pub fn size(&self) -> usize {
        if let Some(dt) = self.dt {
            self.size / dt.bytes()
        } else {
            self.size
        }
    }

    pub fn bind(&self) {
        if BOUND_VBO.with(|b| b.get() != self.id) {
            unsafe { gl::BindBuffer(self.target.value(), self.id) };
            BOUND_VBO.with(|b| b.set(self.id));
        }
    }

    pub fn unbind(&self) {
        if BOUND_VBO.with(|b| b.get() != 0) {
            unsafe { gl::BindBuffer(self.target.value(), 0) };
            BOUND_VBO.with(|b| b.set(0));
        }
    }

    pub fn delete(&mut self) {
        if !self.deleted {
            unsafe { gl::DeleteBuffers(1, &self.id) };
            self.deleted = true;
        }
    }
}
