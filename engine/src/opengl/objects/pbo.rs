use gl::types::*;

/// Pixel-Buffer-Object wrapper for async texture uploads.
///
/// A PBO lets pixel data be uploaded to GPU memory without stalling
/// the GL pipeline.  `glBufferData` copies the data asynchronously,
/// and a subsequent `glTexImage2D` with a null pointer reads from the
/// PBO instead of from CPU-visible memory.
pub struct Pbo {
    id: GLuint,
    size: usize,
}

thread_local! {
    static BOUND_PBO: std::cell::Cell<GLuint> = const { std::cell::Cell::new(0) };
}

impl Pbo {
    pub fn create() -> Self {
        let mut id = 0;
        unsafe { gl::GenBuffers(1, &mut id) };
        Self { id, size: 0 }
    }

    pub fn get_id(&self) -> GLuint {
        self.id
    }

    pub fn bind(&self) {
        if BOUND_PBO.with(|b| b.get() != self.id) {
            unsafe { gl::BindBuffer(gl::PIXEL_UNPACK_BUFFER, self.id) };
            BOUND_PBO.with(|b| b.set(self.id));
        }
    }

    #[allow(dead_code)]
    pub fn unbind(&self) {
        if BOUND_PBO.with(|b| b.get() != 0) {
            unsafe { gl::BindBuffer(gl::PIXEL_UNPACK_BUFFER, 0) };
            BOUND_PBO.with(|b| b.set(0));
        }
    }

    /// Upload raw bytes into the PBO.
    ///
    /// Re-allocates if the buffer is not large enough.
    pub fn store_data(&mut self, data: &[u8]) {
        let byte_len = data.len();
        if byte_len > self.size {
            self.bind();
            unsafe {
                gl::BufferData(
                    gl::PIXEL_UNPACK_BUFFER,
                    byte_len as isize,
                    data.as_ptr() as *const _,
                    gl::STREAM_DRAW,
                );
            }
            self.size = byte_len;
        } else {
            self.bind();
            unsafe {
                gl::BufferSubData(
                    gl::PIXEL_UNPACK_BUFFER,
                    0,
                    byte_len as isize,
                    data.as_ptr() as *const _,
                );
            }
        }
    }

    pub fn delete(&mut self) {
        if self.id != 0 {
            unsafe { gl::DeleteBuffers(1, &self.id) };
            self.id = 0;
            self.size = 0;
        }
    }
}

impl Drop for Pbo {
    fn drop(&mut self) {
        if self.id != 0 {
            log::warn!("Pbo {} dropped without explicit delete", self.id);
        }
    }
}

/// Whether the GL driver supports PBO-based texture uploads.
///
/// Desktop GL 2.1+ and GLES 3.0+ include PBO in core.
pub fn supports_pbo() -> bool {
    unsafe {
        let s = gl::GetString(gl::VERSION);
        if s.is_null() {
            return false;
        }
        let s = match std::ffi::CStr::from_ptr(s as *const std::ffi::c_char).to_str() {
            Ok(v) => v,
            Err(_) => return false,
        };

        let is_gles = s.contains("ES");
        let major = s
            .chars()
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse::<u32>()
            .unwrap_or(0);

        if is_gles && major >= 3 {
            return true;
        }
        if !is_gles && major >= 2 {
            return true;
        }
        false
    }
}
