use gl;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AttachmentType {
    Colour,
    Depth,
    Stencil,
    DepthStencil,
}

impl AttachmentType {
    pub fn get(&self) -> u32 {
        match self {
            AttachmentType::Colour => gl::COLOR_ATTACHMENT0,
            AttachmentType::Depth => gl::DEPTH_ATTACHMENT,
            AttachmentType::Stencil => gl::STENCIL_ATTACHMENT,
            AttachmentType::DepthStencil => gl::DEPTH_STENCIL_ATTACHMENT,
        }
    }
}
