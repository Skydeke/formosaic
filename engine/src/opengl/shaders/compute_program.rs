use cgmath::Vector2;

use crate::opengl::shaders::uniform::Uniform;
use crate::opengl::shaders::RenderState;
use crate::rendering::abstracted::processable::Processable;
use std::ffi::CString;

pub struct ComputeProgram<T: Processable> {
    id: u32,
    uniforms: Vec<Box<dyn for<'a> Uniform<RenderState<'a, T>>>>,
}

impl<T: Processable> ComputeProgram<T> {
    pub fn from_source(source: &str) -> Result<Self, String> {
        unsafe {
            let shader = gl::CreateShader(gl::COMPUTE_SHADER);
            if shader == 0 {
                return Err("Failed to create compute shader".into());
            }

            let c_source = CString::new(source).unwrap();
            gl::ShaderSource(shader, 1, &c_source.as_ptr(), std::ptr::null());
            gl::CompileShader(shader);

            let mut success = 0;
            gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
            if success != gl::TRUE as i32 {
                let mut len = 0;
                gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
                let mut buf = vec![0u8; len as usize];
                gl::GetShaderInfoLog(
                    shader,
                    len,
                    std::ptr::null_mut(),
                    buf.as_mut_ptr() as *mut _,
                );
                return Err(format!(
                    "Shader compile error: {}",
                    String::from_utf8_lossy(&buf)
                ));
            }

            let program = gl::CreateProgram();
            gl::AttachShader(program, shader);
            gl::LinkProgram(program);

            let mut link_success = 0;
            gl::GetProgramiv(program, gl::LINK_STATUS, &mut link_success);
            if link_success != gl::TRUE as i32 {
                let mut len = 0;
                gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
                let mut buf = vec![0u8; len as usize];
                gl::GetProgramInfoLog(
                    program,
                    len,
                    std::ptr::null_mut(),
                    buf.as_mut_ptr() as *mut _,
                );
                return Err(format!(
                    "Program link error: {}",
                    String::from_utf8_lossy(&buf)
                ));
            }

            gl::DeleteShader(shader);

            Ok(Self {
                id: program,
                uniforms: Vec::new(),
            })
        }
    }

    pub fn bind(&self) {
        unsafe { gl::UseProgram(self.id) };
    }

    pub fn unbind(&self) {
        unsafe { gl::UseProgram(0) };
    }

    pub fn dispatch(&self, group_x: u32, group_y: u32, resolution: Vector2<u32>) {
        let dispatch_x = resolution.x.div_ceil(group_x);
        let dispatch_y = resolution.y.div_ceil(group_y);

        unsafe {
            gl::DispatchCompute(dispatch_x, dispatch_y, 1);
        }
    }

    pub fn memory_barrier(&self, flags: u32) {
        unsafe { gl::MemoryBarrier(flags) };
    }

    pub fn add_uniform(&mut self, mut uniform: Box<dyn for<'a> Uniform<RenderState<'a, T>>>) {
        self.bind();
        uniform.initialize(self.id);
        self.uniforms.push(uniform);
    }

    pub fn update_uniforms<'a>(&self, state: &RenderState<'a, T>) {
        for uniform in &self.uniforms {
            uniform.load(state);
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }
}

impl<T: Processable> Drop for ComputeProgram<T> {
    fn drop(&mut self) {
        unsafe { gl::DeleteProgram(self.id) };
    }
}
