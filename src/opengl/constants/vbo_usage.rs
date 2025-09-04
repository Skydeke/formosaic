#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VboUsage {
    StreamDraw,
    StreamRead,
    StreamCopy,
    StaticDraw,
    StaticRead,
    StaticCopy,
    DynamicDraw,
    DynamicRead,
    DynamicCopy,
}

impl VboUsage {
    pub fn value(&self) -> u32 {
        use gl::*;
        match self {
            VboUsage::StreamDraw => STREAM_DRAW,
            VboUsage::StreamRead => STREAM_READ,
            VboUsage::StreamCopy => STREAM_COPY,
            VboUsage::StaticDraw => STATIC_DRAW,
            VboUsage::StaticRead => STATIC_READ,
            VboUsage::StaticCopy => STATIC_COPY,
            VboUsage::DynamicDraw => DYNAMIC_DRAW,
            VboUsage::DynamicRead => DYNAMIC_READ,
            VboUsage::DynamicCopy => DYNAMIC_COPY,
        }
    }
}
