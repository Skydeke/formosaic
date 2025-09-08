#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlBuffer {
    Colour,
    Depth,
    Stencil,
}

impl GlBuffer {
    pub fn get_value(buffers: &[GlBuffer]) -> u32 {
        let mut result = 0;
        for buffer in buffers {
            result |= match buffer {
                GlBuffer::Colour => gl::COLOR_BUFFER_BIT,
                GlBuffer::Depth => gl::DEPTH_BUFFER_BIT,
                GlBuffer::Stencil => gl::STENCIL_BUFFER_BIT,
            };
        }
        result
    }
}
