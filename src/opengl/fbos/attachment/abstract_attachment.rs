use crate::opengl::fbos::attachment::attachment_type::AttachmentType;

pub struct AbstractAttachment {
    attachment_type: AttachmentType,
    attachment_index: i32,
}

impl AbstractAttachment {
    pub fn new(attachment_type: AttachmentType, attachment_index: i32) -> Self {
        Self {
            attachment_type,
            attachment_index,
        }
    }

    pub fn get_attachment_point(&self) -> i32 {
        (self.attachment_type.get() as i32) + self.attachment_index
    }

    pub fn get_attachment_type(&self) -> AttachmentType {
        self.attachment_type
    }
}
