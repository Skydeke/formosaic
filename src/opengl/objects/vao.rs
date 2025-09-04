use gl::types::*;
use std::cell::Cell;
use std::rc::Rc;

use crate::opengl::objects::attribute::Attribute;
use crate::opengl::objects::ivbo::IVbo;

pub struct Vao {
    pub id: GLuint,
    data_buffers: Vec<Rc<dyn IVbo>>,
    attributes: Vec<Attribute>,
    index_buffer: Option<Rc<dyn IVbo>>,
    deleted: bool,
}

thread_local! {
    static BOUND_VAO: Cell<GLuint> = Cell::new(0);
}

impl Vao {
    pub fn create() -> Self {
        let mut id: GLuint = 0;
        unsafe { gl::GenVertexArrays(1, &mut id) };
        let vao = Vao {
            id,
            data_buffers: Vec::new(),
            attributes: Vec::new(),
            index_buffer: None,
            deleted: false,
        };
        vao.bind();
        vao
    }

    pub fn bind_if_none() {
        BOUND_VAO.with(|b| {
            if b.get() == 0 {
                let vao = Vao::create();
                vao.bind();
            }
        });
    }

    pub fn is_bound(&self) -> bool {
        BOUND_VAO.with(|b| b.get() == self.id)
    }

    pub fn bind(&self) {
        if !self.is_bound() {
            unsafe { gl::BindVertexArray(self.id) };
            BOUND_VAO.with(|b| b.set(self.id));
        }
    }

    pub fn unbind(&self) {
        if self.is_bound() {
            unsafe { gl::BindVertexArray(0) };
            BOUND_VAO.with(|b| b.set(0));
        }
    }

    pub fn enable_attributes(&mut self) {
        for attr in &mut self.attributes {
            attr.enable();
        }
    }

    pub fn disable_attributes(&mut self) {
        for attr in &mut self.attributes {
            attr.disable();
        }
    }

    pub fn has_indices(&self) -> bool {
        self.index_buffer.is_some()
    }

    pub fn get_index_count(&self) -> usize {
        self.index_buffer
            .as_ref()
            .map(|ib| ib.get_size())
            .unwrap_or(0)
    }

    pub fn load_data_buffer(&mut self, vbo: Rc<dyn IVbo>, attributes: &[Attribute]) {
        vbo.bind_to_vao(self);
        self.link_attributes(attributes);
        self.data_buffers.push(vbo);
    }

    pub fn load_index_buffer(&mut self, index_buffer: Rc<dyn IVbo>, delete_old: bool) {
        if let Some(old) = &self.index_buffer {
            if delete_old {
                old.delete();
            }
        }
        self.index_buffer = Some(index_buffer.clone());
        index_buffer.bind_to_vao(self);
    }

    pub fn delete(&mut self, delete_vbos: bool) {
        if delete_vbos {
            if let Some(index) = &self.index_buffer {
                index.delete();
            }
            for vbo in &self.data_buffers {
                vbo.delete();
            }
        }

        if !self.deleted {
            unsafe { gl::DeleteVertexArrays(1, &self.id) };
            self.deleted = true;
        }
    }

    fn link_attributes(&mut self, attributes: &[Attribute]) {
        let mut offset = 0;
        let stride = self.get_bytes_per_vertex(attributes);
        for attr in attributes {
            attr.link(stride, offset);
            offset += attr.bytes_per_vertex() as i32;
            self.attributes.push(attr.clone());
        }
    }

    fn get_bytes_per_vertex(&self, attributes: &[Attribute]) -> i32 {
        attributes.iter().map(|a| a.bytes_per_vertex() as i32).sum()
    }
}
