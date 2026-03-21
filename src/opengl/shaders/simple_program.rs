//! A simple compiled and linked shader program without the type-parameterized
//! uniform system of ShaderProgram<T>. Used for custom renderers (e.g. imgui,
//! menu) that manage their own uniforms imperatively.

use crate::opengl::shaders::shader::Shader;
use std::ffi::CString;

pub struct SimpleProgram {
    id: u32,
}

impl SimpleProgram {
    pub fn from_sources(vert: &str, frag: &str) -> Result<Self, String> {
        let v = Shader::vertex(vert)?;
        let f = Shader::fragment(frag)?;
        let id = unsafe {
            let id = gl::CreateProgram();
            v.attach(id); f.attach(id);
            gl::LinkProgram(id);
            let mut ok = 0i32;
            gl::GetProgramiv(id, gl::LINK_STATUS, &mut ok);
            v.detach(id); v.delete();
            f.detach(id); f.delete();
            if ok != gl::TRUE as i32 {
                return Err("Program link failed".to_string());
            }
            id
        };
        Ok(Self { id })
    }

    pub fn bind(&self) {
        unsafe { gl::UseProgram(self.id); }
    }

    pub fn unbind(&self) {
        unsafe { gl::UseProgram(0); }
    }

    pub fn uniform_location(&self, name: &str) -> i32 {
        let c = CString::new(name).unwrap();
        unsafe { gl::GetUniformLocation(self.id, c.as_ptr()) }
    }

    pub fn set_uniform_mat4(&self, loc: i32, mat: &[f32; 16]) {
        if loc >= 0 {
            unsafe { gl::UniformMatrix4fv(loc, 1, gl::FALSE, mat.as_ptr()); }
        }
    }

    pub fn set_uniform_int(&self, loc: i32, v: i32) {
        if loc >= 0 {
            unsafe { gl::Uniform1i(loc, v); }
        }
    }

    pub fn set_uniform_float(&self, loc: i32, v: f32) {
        if loc >= 0 {
            unsafe { gl::Uniform1f(loc, v); }
        }
    }

    pub fn set_uniform_vec2(&self, loc: i32, x: f32, y: f32) {
        if loc >= 0 {
            unsafe { gl::Uniform2f(loc, x, y); }
        }
    }

    pub fn set_uniform_vec3(&self, loc: i32, x: f32, y: f32, z: f32) {
        if loc >= 0 {
            unsafe { gl::Uniform3f(loc, x, y, z); }
        }
    }

    pub fn id(&self) -> u32 { self.id }
}

impl Drop for SimpleProgram {
    fn drop(&mut self) {
        unsafe { gl::DeleteProgram(self.id); }
    }
}
