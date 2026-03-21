use gl::types::*;

use crate::opengl::constants::data_type::DataType;

#[derive(Debug, Clone)]
pub struct Attribute {
    attribute_index: GLuint,
    component_count: GLint,
    components_type: GLenum,
    bytes_per_vertex: usize,
    normalized: bool,
    instances: GLuint,
    enabled: bool,
}

impl Attribute {
    /// Creates a standard attribute
    pub fn of(
        attribute_index: GLuint,
        component_count: GLint,
        data_type: DataType,
        normalized: bool,
    ) -> Self {
        Self::new(attribute_index, component_count, data_type, normalized, 0)
    }

    /// Creates an instanced attribute with divisor 1
    pub fn of_instance(
        attribute_index: GLuint,
        component_count: GLint,
        data_type: DataType,
        normalized: bool,
    ) -> Self {
        Self::new(attribute_index, component_count, data_type, normalized, 1)
    }

    /// Creates an instanced attribute with a custom divisor
    pub fn of_instances(
        attribute_index: GLuint,
        component_count: GLint,
        data_type: DataType,
        normalized: bool,
        instances: GLuint,
    ) -> Self {
        Self::new(
            attribute_index,
            component_count,
            data_type,
            normalized,
            instances,
        )
    }

    /// Convenience attribute for vertex positions
    pub fn of_positions() -> Self {
        Self::of(0, 3, DataType::Float, false)
    }

    /// Convenience attribute for texture coordinates
    pub fn of_tex_coords() -> Self {
        Self::of(1, 2, DataType::Float, false)
    }

    /// Convenience attribute for normals
    pub fn of_normals() -> Self {
        Self::of(2, 3, DataType::Float, false)
    }

    fn new(
        attribute_index: GLuint,
        component_count: GLint,
        data_type: DataType,
        normalized: bool,
        instances: GLuint,
    ) -> Self {
        Self {
            attribute_index,
            component_count,
            components_type: data_type.value(),
            bytes_per_vertex: (component_count as usize) * data_type.bytes(),
            normalized,
            instances,
            enabled: false,
        }
    }

    /// Enable this attribute
    pub fn enable(&mut self) {
        if !self.enabled {
            unsafe { gl::EnableVertexAttribArray(self.attribute_index) };
            self.enabled = true;
        }
    }

    /// Disable this attribute
    pub fn disable(&mut self) {
        if self.enabled {
            unsafe { gl::DisableVertexAttribArray(self.attribute_index) };
            self.enabled = false;
        }
    }

    /// Link the attribute to a VBO with a stride and offset
    pub fn link(&self, stride: GLint, offset: GLint) {
        unsafe {
            gl::VertexAttribPointer(
                self.attribute_index,
                self.component_count,
                self.components_type,
                self.normalized as GLboolean,
                stride,
                offset as *const _,
            );
            if self.instances > 0 {
                gl::VertexAttribDivisor(self.attribute_index, self.instances);
            }
        }
    }

    /// Static helpers for enabling/disabling by index
    pub fn enable_index(index: GLuint) {
        unsafe { gl::EnableVertexAttribArray(index) };
    }

    pub fn disable_index(index: GLuint) {
        unsafe { gl::DisableVertexAttribArray(index) };
    }

    /// Getters
    pub fn attribute_index(&self) -> GLuint {
        self.attribute_index
    }
    pub fn component_count(&self) -> GLint {
        self.component_count
    }
    pub fn components_type(&self) -> GLenum {
        self.components_type
    }
    pub fn normalized(&self) -> bool {
        self.normalized
    }
    pub fn bytes_per_vertex(&self) -> usize {
        self.bytes_per_vertex
    }
    pub fn instances(&self) -> GLuint {
        self.instances
    }
}
