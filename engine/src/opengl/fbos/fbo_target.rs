#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FboTarget {
    Framebuffer,
    DrawFramebuffer,
    ReadFramebuffer,
}

impl FboTarget {
    pub fn get(&self) -> u32 {
        match self {
            FboTarget::Framebuffer => gl::FRAMEBUFFER,
            FboTarget::DrawFramebuffer => gl::DRAW_FRAMEBUFFER,
            FboTarget::ReadFramebuffer => gl::READ_FRAMEBUFFER,
        }
    }
}

