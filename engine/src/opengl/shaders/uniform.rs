use cgmath::Matrix;
use cgmath::Matrix4;
use std::ffi::CString;
use std::rc::Rc;

use crate::opengl::shaders::RenderState;
use crate::opengl::textures::texture::Texture;
use crate::rendering::abstracted::processable::Processable;

pub trait Uniform<T> {
    fn initialize(&mut self, program_id: u32);
    fn load(&self, state: &T);
}

macro_rules! make_uniform {
    ($name:ident, $ty:ty, $load_body:expr) => {
        pub struct $name {
            name: String,
            location: i32,
        }

        impl $name {
            pub fn new(name: &str) -> Self {
                Self {
                    name: name.to_string(),
                    location: -1,
                }
            }
        }

        impl Uniform<$ty> for $name {
            fn initialize(&mut self, program_id: u32) {
                let cname = CString::new(self.name.clone()).unwrap();
                self.location = unsafe { gl::GetUniformLocation(program_id, cname.as_ptr()) };
                if self.location == -1 {
                    log::warn!("Uniform '{}' not found in shader program", self.name);
                }
            }

            fn load(&self, value: &$ty) {
                if self.location != -1 {
                    let f: fn(&$name, &$ty) = $load_body;
                    f(self, value)
                }
            }
        }
    };
}

make_uniform!(UniformMatrix4, Matrix4<f32>, |s, v| unsafe {
    gl::UniformMatrix4fv(s.location, 1, gl::FALSE, v.as_ptr())
});
make_uniform!(UniformFloat, f32, |s, v| unsafe {
    gl::Uniform1f(s.location, *v)
});
make_uniform!(UniformInt, i32, |s, v| unsafe {
    gl::Uniform1i(s.location, *v)
});
make_uniform!(UniformBoolean, bool, |s, v| unsafe {
    gl::Uniform1i(s.location, if *v { 1 } else { 0 })
});
make_uniform!(UniformVec2, cgmath::Vector2<f32>, |s, v| unsafe {
    gl::Uniform2f(s.location, v.x, v.y)
});
make_uniform!(UniformVec3, cgmath::Vector3<f32>, |s, v| unsafe {
    gl::Uniform3f(s.location, v.x, v.y, v.z)
});

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
            unit,
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
                    tex.as_ref().bind_to_unit(self.unit);
                } else {
                    gl::BindTexture(gl::TEXTURE_2D, 0);
                }
            }
        }
    }
}

pub struct UniformMatrix4Array {
    elements: Vec<UniformMatrix4>,
}

impl UniformMatrix4Array {
    pub fn new(name: &str, max_count: usize) -> Self {
        let elements = (0..max_count)
            .map(|i| UniformMatrix4::new(&format!("{}[{}]", name, i)))
            .collect();
        Self { elements }
    }
}

impl Uniform<Vec<Matrix4<f32>>> for UniformMatrix4Array {
    fn initialize(&mut self, program_id: u32) {
        for elem in &mut self.elements {
            elem.initialize(program_id);
        }
    }

    fn load(&self, matrices: &Vec<Matrix4<f32>>) {
        let count = matrices.len().min(self.elements.len());
        for i in 0..count {
            self.elements[i].load(&matrices[i]);
        }
    }
}

pub struct UniformAdapter<U, T, F>
where
    U: Uniform<F>,
    T: Processable,
{
    pub uniform: U,
    pub extractor: Box<dyn for<'a> Fn(&'a RenderState<'a, T>) -> F>,
}

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
