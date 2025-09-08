use cgmath::Matrix;
use cgmath::Matrix4;
use std::ffi::CString;

use crate::engine::rendering::abstracted::processable::Processable;
use crate::opengl::shaders::RenderState;

pub trait Uniform<T> {
    fn initialize(&mut self, program_id: u32);
    fn load(&self, state: &T);
}

pub struct UniformMatrix4 {
    name: String,
    location: i32,
}

impl UniformMatrix4 {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            location: -1,
        }
    }
}

impl Uniform<Matrix4<f32>> for UniformMatrix4 {
    fn initialize(&mut self, program_id: u32) {
        let cname = CString::new(self.name.clone()).unwrap();
        self.location = unsafe { gl::GetUniformLocation(program_id, cname.as_ptr()) };
        if self.location == -1 {
            log::warn!("Uniform '{}' not found in shader program", self.name);
        }
    }

    fn load(&self, matrix: &Matrix4<f32>) {
        if self.location != -1 {
            unsafe {
                gl::UniformMatrix4fv(self.location, 1, gl::FALSE, matrix.as_ptr());
            }
        }
    }
}

pub struct UniformFloat {
    name: String,
    location: i32,
}

impl UniformFloat {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            location: -1,
        }
    }
}

impl Uniform<f32> for UniformFloat {
    fn initialize(&mut self, program_id: u32) {
        let cname = CString::new(self.name.clone()).unwrap();
        self.location = unsafe { gl::GetUniformLocation(program_id, cname.as_ptr()) };
        if self.location == -1 {
            log::warn!("Uniform '{}' not found in shader program", self.name);
        }
    }

    fn load(&self, value: &f32) {
        if self.location != -1 {
            unsafe {
                gl::Uniform1f(self.location, *value);
            }
        }
    }
}

pub struct UniformInt {
    name: String,
    location: i32,
}

impl UniformInt {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            location: -1,
        }
    }
}

impl Uniform<i32> for UniformInt {
    fn initialize(&mut self, program_id: u32) {
        let cname = CString::new(self.name.clone()).unwrap();
        self.location = unsafe { gl::GetUniformLocation(program_id, cname.as_ptr()) };
        if self.location == -1 {
            log::warn!("Uniform '{}' not found in shader program", self.name);
        }
    }

    fn load(&self, value: &i32) {
        if self.location != -1 {
            unsafe {
                gl::Uniform1i(self.location, *value);
            }
        }
    }
}

pub struct UniformVec3 {
    name: String,
    location: i32,
}

impl UniformVec3 {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            location: -1,
        }
    }
}

impl Uniform<cgmath::Vector3<f32>> for UniformVec3 {
    fn initialize(&mut self, program_id: u32) {
        let cname = CString::new(self.name.clone()).unwrap();
        self.location = unsafe { gl::GetUniformLocation(program_id, cname.as_ptr()) };
        if self.location == -1 {
            log::warn!("Uniform '{}' not found in shader program", self.name);
        }
    }

    fn load(&self, vector: &cgmath::Vector3<f32>) {
        if self.location != -1 {
            unsafe {
                gl::Uniform3f(self.location, vector.x, vector.y, vector.z);
            }
        }
    }
}

// Define a wrapper that can work with any lifetime
pub struct UniformAdapter<U, T, F>
where
    U: Uniform<F>,
    F: Copy,
    T: Processable,
{
    pub uniform: U,
    pub extractor: Box<dyn for<'a> Fn(&'a RenderState<'a, T>) -> F>,
}

// Implement for any possible lifetime
impl<U, T, F> Uniform<RenderState<'_, T>> for UniformAdapter<U, T, F>
where
    U: Uniform<F>,
    F: Copy,
    T: Processable,
{
    fn initialize(&mut self, program_id: u32) {
        self.uniform.initialize(program_id);
    }

    fn load(&self, state: &RenderState<'_, T>) {
        let value = (self.extractor)(state);
        self.uniform.load(&value);
    }
}
