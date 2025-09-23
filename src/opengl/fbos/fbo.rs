use crate::opengl::constants::gl_buffer::GlBuffer;
use crate::opengl::fbos::attachment::attachment::Attachment;
use crate::opengl::fbos::attachment::attachment_type::AttachmentType;
use crate::opengl::fbos::fbo_target::FboTarget;
use crate::opengl::fbos::ifbo::IFbo;
use crate::opengl::textures::parameters::mag_filter_parameter::MagFilterParameter;
use gl;

pub struct Fbo {
    id: u32,
    width: i32,
    height: i32,
    attachments: Vec<Box<dyn Attachment>>,
    depth_attachment: Option<Box<dyn Attachment>>,
    read_attachment: Option<Box<dyn Attachment>>,
    draw_attachments: Option<Vec<Box<dyn Attachment>>>,
}

impl Fbo {
    pub const DEPTH_ATTACHMENT: i32 = -1;

    pub fn new(id: u32, width: i32, height: i32) -> Self {
        Self {
            id,
            width,
            height,
            attachments: Vec::new(),
            depth_attachment: None,
            read_attachment: None,
            draw_attachments: None,
        }
    }

    pub fn create(width: i32, height: i32) -> Self {
        let mut id: u32 = 0;
        unsafe { gl::GenFramebuffers(1, &mut id) }; // Fix: Generate proper ID
        Fbo::new(id, width, height)
    }

    pub fn get_depth_attachment(&self) -> Option<&Box<dyn Attachment>> {
        self.depth_attachment.as_ref()
    }

    pub fn get_attachments(&self) -> &Vec<Box<dyn Attachment>> {
        &self.attachments
    }

    pub fn add_attachment(&mut self, attachment: Box<dyn Attachment>) {
        self.bind(FboTarget::Framebuffer);
        match attachment.get_attachment_type() {
            AttachmentType::Colour => {
                self.add_colour_attachment(attachment);
            }
            AttachmentType::Depth => {
                self.set_depth_attachment(attachment);
            }
            _ => {}
        }
    }

    fn add_colour_attachment(&mut self, mut attachment: Box<dyn Attachment>) {
        attachment.init(self);
        self.attachments.push(attachment);
    }

    fn set_depth_attachment(&mut self, mut attachment: Box<dyn Attachment>) {
        if let Some(mut existing) = self.depth_attachment.take() {
            existing.delete();
        }
        attachment.init(self);
        self.depth_attachment = Some(attachment);
    }

    pub fn is_sized(&self, width: i32, height: i32) -> bool {
        self.width == width && self.height == height
    }

    pub fn blit_fbo(&mut self, fbo: &mut Fbo, filter: MagFilterParameter, buffers: &[GlBuffer]) {
        self.bind(FboTarget::ReadFramebuffer);
        fbo.bind(FboTarget::DrawFramebuffer);

        unsafe {
            gl::BlitFramebuffer(
                0,
                0,
                self.width,
                self.height,
                0,
                0,
                fbo.width,
                fbo.height,
                GlBuffer::get_value(buffers),
                filter.get(),
            );
        }

        fbo.unbind(FboTarget::DrawFramebuffer);
        self.unbind(FboTarget::ReadFramebuffer);
    }

    // Add static blit_framebuffer method for scene_fbo.rs compatibility
    pub fn blit_framebuffer(
        src_x0: i32,
        src_y0: i32,
        src_x1: i32,
        src_y1: i32,
        dst_x0: i32,
        dst_y0: i32,
        dst_x1: i32,
        dst_y1: i32,
        filter: MagFilterParameter,
        buffers: &[GlBuffer],
    ) {
        unsafe {
            gl::BlitFramebuffer(
                src_x0,
                src_y0,
                src_x1,
                src_y1,
                dst_x0,
                dst_y0,
                dst_x1,
                dst_y1,
                GlBuffer::get_value(buffers),
                filter.get(),
            );
        }
    }

    pub fn set_read_attachment(&mut self, attachment_index: i32) {
        if attachment_index == Fbo::DEPTH_ATTACHMENT {
            self.read_attachment = self.depth_attachment.as_ref().map(|a| a.clone_attachment());
        } else {
            self.read_attachment =
                Some(self.attachments[attachment_index as usize].clone_attachment());
        }
    }

    pub fn set_draw_attachments(&mut self, attachments: &[i32]) {
        let mut list = Vec::new();
        for &index in attachments {
            if index == Fbo::DEPTH_ATTACHMENT {
                if let Some(depth) = &self.depth_attachment {
                    list.push(depth.clone_attachment());
                }
            } else {
                list.push(self.attachments[index as usize].clone_attachment());
            }
        }
        self.draw_attachments = Some(list);
    }

    fn get_read_attachment(&mut self) -> &Box<dyn Attachment> {
        if self.attachments.is_empty() {
            panic!("Cannot read from empty Fbo");
        }
        if self.read_attachment.is_none() {
            self.read_attachment = Some(self.attachments[0].clone_attachment());
        }
        self.read_attachment.as_ref().unwrap()
    }

    fn read_attachment(&mut self) {
        let attachment = self.get_read_attachment();
        unsafe { gl::ReadBuffer(attachment.get_attachment_point() as u32) };
    }

    fn get_draw_attachments(&self) -> Vec<Box<dyn Attachment>> {
        if let Some(list) = &self.draw_attachments {
            list.iter().map(|a| a.clone_attachment()).collect()
        } else {
            self.attachments
                .iter()
                .map(|a| a.clone_attachment())
                .collect()
        }
    }

    fn draw_attachments(&self) {
        let draw_attachments = self.get_draw_attachments();
        if draw_attachments.len() >= 1 {
            let buffer: Vec<u32> = draw_attachments
                .iter()
                .map(|a| a.get_attachment_point() as u32)
                .collect();
            unsafe { gl::DrawBuffers(buffer.len() as i32, buffer.as_ptr()) };
        }
    }

    pub fn get_width(&self) -> i32 {
        self.width
    }

    pub fn get_height(&self) -> i32 {
        self.height
    }

    pub fn resize(&mut self, width: i32, height: i32) {
        self.width = width;
        self.height = height;

        // Bind framebuffer before messing with attachments
        self.bind(FboTarget::Framebuffer);

        for attachment in &mut self.attachments {
            attachment.resize(width, height);
        }
        if let Some(depth) = &mut self.depth_attachment {
            depth.resize(width, height);
        }
        self.unbind(FboTarget::Framebuffer);
    }

    pub fn bind(&mut self, target: FboTarget) {
        unsafe { gl::BindFramebuffer(target.get(), self.id) };
        match target {
            FboTarget::DrawFramebuffer => {
                self.draw_attachments();
                self.set_viewport();
            }
            FboTarget::Framebuffer => {
                self.set_viewport();
            }
            FboTarget::ReadFramebuffer => {
                self.read_attachment();
            }
        }
    }

    fn set_viewport(&self) {
        unsafe { gl::Viewport(0, 0, self.width, self.height) };
    }

    pub fn unbind(&self, target: FboTarget) {
        unsafe { gl::BindFramebuffer(target.get(), 0) };
    }

    pub fn delete(&mut self) {
        unsafe { gl::DeleteFramebuffers(1, &self.id) };
        for attachment in &mut self.attachments {
            attachment.delete();
        }
        if let Some(depth) = &mut self.depth_attachment {
            depth.delete();
        }
    }
}

impl Drop for Fbo {
    fn drop(&mut self) {
        self.delete();
    }
}

impl IFbo for Fbo {
    fn blit_fbo(&mut self, fbo: &mut dyn IFbo, filter: MagFilterParameter, buffers: &[GlBuffer]) {
        // Convert dyn IFbo to &Fbo for compatibility
        if let Some(concrete_fbo) = (fbo as &mut dyn std::any::Any).downcast_mut::<Fbo>() {
            Fbo::blit_fbo(self, concrete_fbo, filter, buffers);
        }
    }

    fn get_width(&self) -> i32 {
        self.width
    }

    fn get_height(&self) -> i32 {
        self.height
    }

    fn resize(&mut self, width: i32, height: i32) {
        Fbo::resize(self, width, height);
    }

    fn bind(&mut self, target: FboTarget) {
        Fbo::bind(self, target);
    }

    fn unbind(&self, target: FboTarget) {
        Fbo::unbind(self, target);
    }

    fn delete(&mut self) {
        Fbo::delete(self);
    }
}
