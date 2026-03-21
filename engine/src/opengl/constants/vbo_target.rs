#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VboTarget {
    ArrayBuffer,
    ElementArrayBuffer,
    UniformBuffer,
}

impl VboTarget {
    pub fn value(&self) -> u32 {
        use gl::*;
        match self {
            VboTarget::ArrayBuffer => ARRAY_BUFFER,
            VboTarget::ElementArrayBuffer => ELEMENT_ARRAY_BUFFER,
            VboTarget::UniformBuffer => UNIFORM_BUFFER,
        }
    }

    /// Converts a raw OpenGL value to the corresponding VboTarget, if valid
    pub fn from_value(id: u32) -> Option<VboTarget> {
        use gl::*;
        match id {
            ARRAY_BUFFER => Some(VboTarget::ArrayBuffer),
            ELEMENT_ARRAY_BUFFER => Some(VboTarget::ElementArrayBuffer),
            UNIFORM_BUFFER => Some(VboTarget::UniformBuffer),
            _ => None,
        }
    }
}
