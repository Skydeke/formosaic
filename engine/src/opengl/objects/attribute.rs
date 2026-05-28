use gl::types::*;
use std::cell::Cell;

use crate::opengl::constants::data_type::DataType;

#[derive(Debug, Clone)]
pub struct Attribute {
    attribute_index: GLuint,
    component_count: GLint,
    components_type: GLenum,
    bytes_per_vertex: usize,
    normalized: bool,
    instances: GLuint,
    enabled: Cell<bool>,
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
            enabled: Cell::new(false),
        }
    }

    /// Enable this attribute
    pub fn enable(&self) {
        if !self.enabled.get() {
            unsafe { gl::EnableVertexAttribArray(self.attribute_index) };
            self.enabled.set(true);
        }
    }

    /// Disable this attribute
    pub fn disable(&self) {
        if self.enabled.get() {
            unsafe { gl::DisableVertexAttribArray(self.attribute_index) };
            self.enabled.set(false);
        }
    }

    /// Link the attribute to a VBO with a stride and offset.
    ///
    /// **Non-normalized** integer GL types (INT, UNSIGNED_INT, …) must be
    /// linked with `VertexAttribIPointer` so the shader receives raw integer
    /// values in `ivec` inputs.  Using `VertexAttribPointer` for those types
    /// causes the driver to silently reinterpret the bits as floats, producing
    /// garbage bone indices — the root cause of broken skinning.
    ///
    /// **Normalized** integer types (e.g. UNSIGNED_BYTE with normalized=true,
    /// as used by ImGui for packed RGBA colors) must still go through
    /// `VertexAttribPointer`: the whole point is that 0-255 gets mapped to
    /// [0, 1] as a float.  `VertexAttribIPointer` does not normalize, so
    /// using it here would send raw 0-255 integers to a `vec4` input and
    /// make the UI invisible.
    pub fn link(&self, stride: GLint, offset: GLint) {
        unsafe {
            if Self::is_integer_gl_type(self.components_type) && !self.normalized {
                gl::VertexAttribIPointer(
                    self.attribute_index,
                    self.component_count,
                    self.components_type,
                    stride,
                    offset as *const _,
                );
            } else {
                gl::VertexAttribPointer(
                    self.attribute_index,
                    self.component_count,
                    self.components_type,
                    self.normalized as GLboolean,
                    stride,
                    offset as *const _,
                );
            }
            if self.instances > 0 {
                gl::VertexAttribDivisor(self.attribute_index, self.instances);
            }
        }
    }

    /// Returns `true` for GL types that are integer-backed.
    /// Combine with `!self.normalized` to decide whether `VertexAttribIPointer`
    /// is required.
    fn is_integer_gl_type(gl_type: GLenum) -> bool {
        matches!(
            gl_type,
            gl::BYTE
                | gl::UNSIGNED_BYTE
                | gl::SHORT
                | gl::UNSIGNED_SHORT
                | gl::INT
                | gl::UNSIGNED_INT
        )
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
