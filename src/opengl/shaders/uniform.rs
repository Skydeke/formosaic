use cgmath::Matrix;
use cgmath::Matrix4;
use std::ffi::CString;
use std::rc::Rc;

use crate::engine::rendering::abstracted::processable::Processable;
use crate::opengl::shaders::RenderState;
use crate::opengl::textures::texture::Texture;

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

pub struct UniformBoolean {
    name: String,
    location: i32,
}

impl UniformBoolean {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            location: -1,
        }
    }
}

impl Uniform<bool> for UniformBoolean {
    fn initialize(&mut self, program_id: u32) {
        let cname = CString::new(self.name.clone()).unwrap();
        self.location = unsafe { gl::GetUniformLocation(program_id, cname.as_ptr()) };
        if self.location == -1 {
            log::warn!("Uniform '{}' not found in shader program", self.name);
        }
    }

    fn load(&self, value: &bool) {
        if self.location != -1 {
            unsafe {
                gl::Uniform1i(self.location, if *value { 1 } else { 0 });
            }
        }
    }
}

pub struct UniformTexture {
    name: String,
    location: i32,
    unit: u32,
}

impl UniformTexture {
    pub fn new(name: &str, unit: u32) -> Self {
        Self {
            name: name.to_string(),
            location: -1,
            unit: 0,
        }
    }
}

impl Uniform<Option<Rc<dyn Texture>>> for UniformTexture {
    fn initialize(&mut self, program_id: u32) {
        let cname = CString::new(self.name.clone()).unwrap();
        self.location = unsafe { gl::GetUniformLocation(program_id, cname.as_ptr()) };
        if self.location == -1 {
            log::warn!("Uniform '{}' not found in shader program", self.name);
        }
        unsafe { gl::Uniform1i(self.location, self.unit.try_into().unwrap()) };
    }

    fn load(&self, value: &Option<Rc<dyn Texture>>) {
        if self.location != -1 {
            unsafe {
                if let Some(tex) = value {
                    // Bind only if texture exists
                    tex.as_ref().bind_to_unit(self.unit);
                } else {
                    // Optional: unbind texture from this unit
                    gl::BindTexture(gl::TEXTURE_2D, 0);
                }
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
    T: Processable,
{
    pub uniform: U,
    pub extractor: Box<dyn for<'a> Fn(&'a RenderState<'a, T>) -> F>,
}

// Implement for any possible lifetime
impl<U, T, F> Uniform<RenderState<'_, T>> for UniformAdapter<U, T, F>
where
    U: Uniform<F>,
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
