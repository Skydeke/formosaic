use std::ffi::CString;
use std::ptr;

#[derive(Debug, Clone, Copy)]
pub enum ShaderType {
    Vertex = gl::VERTEX_SHADER as isize,
    Fragment = gl::FRAGMENT_SHADER as isize,
    Geometry = gl::GEOMETRY_SHADER as isize,
}

pub struct Shader {
    id: u32,
    shader_type: ShaderType,
}

impl Shader {
    pub fn new(source: &str, shader_type: ShaderType) -> Result<Self, String> {
        unsafe {
            let id = gl::CreateShader(shader_type as u32);
            if id == 0 {
                return Err("Failed to create shader".to_string());
            }

            let c_str = CString::new(source).unwrap();
            gl::ShaderSource(id, 1, &c_str.as_ptr(), ptr::null());
            gl::CompileShader(id);

            let mut success = gl::FALSE as i32;
            gl::GetShaderiv(id, gl::COMPILE_STATUS, &mut success);

            if success != gl::TRUE as i32 {
                let error_log = get_shader_info_log(id);
                gl::DeleteShader(id);
                return Err(format!("Shader compilation failed: {}", error_log));
            }

            Ok(Self { id, shader_type })
        }
    }

    pub fn vertex(source: &str) -> Result<Self, String> {
        Self::new(source, ShaderType::Vertex)
    }

    pub fn fragment(source: &str) -> Result<Self, String> {
        Self::new(source, ShaderType::Fragment)
    }

    pub fn geometry(source: &str) -> Result<Self, String> {
        Self::new(source, ShaderType::Geometry)
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn attach(&self, program_id: u32) {
        unsafe { gl::AttachShader(program_id, self.id) };
    }

    pub fn detach(&self, program_id: u32) {
        unsafe { gl::DetachShader(program_id, self.id) };
    }

    pub fn delete(&self) {
        unsafe { gl::DeleteShader(self.id) };
    }
}

fn get_shader_info_log(shader_id: u32) -> String {
    unsafe {
        let mut len = 0;
        gl::GetShaderiv(shader_id, gl::INFO_LOG_LENGTH, &mut len);
        if len > 0 {
            let mut buffer: Vec<u8> = vec![0; len as usize];
            gl::GetShaderInfoLog(
                shader_id,
                len,
                ptr::null_mut(),
                buffer.as_mut_ptr() as *mut _,
            );
            if let Some(&0) = buffer.last() {
                buffer.pop();
            }
            String::from_utf8_lossy(&buffer).into_owned()
        } else {
            String::new()
        }
    }
}
