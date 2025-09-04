use crate::opengl::{
    constants::{data_type::DataType, render_mode::RenderMode},
    objects::{attribute::Attribute, vao::Vao},
};
use crate::renderable::Renderable;

pub struct Mesh {
    vert: Vec<f32>,
    attributes: Vec<Attribute>,
    vao: Vao,
}

impl Mesh {
    pub fn from_vao(vao: Vao) -> Self {
        Self {
            vert: vec![],
            attributes: vec![],
            vao,
        }
    }

    pub fn lowest(&self) -> f32 {
        let mut ret = f32::INFINITY;
        let stride = self
            .attributes
            .first()
            .map_or(3, |a| a.bytes_per_vertex() / 4);
        for i in (1..self.vert.len()).step_by(stride) {
            if self.vert[i] < ret {
                ret = self.vert[i];
            }
        }
        ret
    }

    pub fn delete(&mut self, delete_vbos: bool) {
        self.vao.delete(delete_vbos);
    }
}

impl Renderable for Mesh {
    fn bind(&mut self) {
        self.vao.bind();
        self.vao.enable_attributes();
    }

    fn render(&self, render_mode: RenderMode) {
        if self.vao.has_indices() {
            unsafe {
                gl::DrawElements(
                    render_mode.value(),
                    self.vao.get_index_count().try_into().unwrap(),
                    DataType::UInt.value(),
                    std::ptr::null(),
                )
            };
        } else {
            panic!("Not implemented yet.")
        }
    }
}

impl Mesh {
    pub fn unbind(&self) {
        self.vao.unbind();
    }
}
