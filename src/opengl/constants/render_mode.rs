#[derive(Debug, Clone, Copy)]
pub enum RenderMode {
    Points,
    Lines,
    LineLoop,
    LineStrip,
    Triangles,
    TriangleStrip,
    TriangleFan,
    Quads,
}

impl RenderMode {
    pub fn value(&self) -> u32 {
        use gl::*;
        match self {
            RenderMode::Points => POINTS,
            RenderMode::Lines => LINES,
            RenderMode::LineLoop => LINE_LOOP,
            RenderMode::LineStrip => LINE_STRIP,
            RenderMode::Triangles => TRIANGLES,
            RenderMode::TriangleStrip => TRIANGLE_STRIP,
            RenderMode::TriangleFan => TRIANGLE_FAN,
            RenderMode::Quads => QUADS,
        }
    }

    // Add the missing get() method
    pub fn get(&self) -> u32 {
        self.value()
    }
}
