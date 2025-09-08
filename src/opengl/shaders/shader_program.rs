use crate::engine::rendering::abstracted::processable::Processable;
use crate::opengl::shaders::{shader::Shader, uniform::Uniform, RenderState};
use std::cell::RefCell;
use std::ffi::CString;
use std::ptr;
use std::rc::Rc;

pub struct ShaderProgram<T: Processable> {
    id: u32,
    per_render_uniforms: Vec<Rc<RefCell<dyn for<'a> Uniform<RenderState<'a, T>>>>>,
    per_instance_uniforms: Vec<Rc<RefCell<dyn for<'a> Uniform<RenderState<'a, T>>>>>,
    bound_program: Option<u32>,
}

impl<T: Processable> ShaderProgram<T> {
    pub fn new(shaders: &[Shader]) -> Result<Self, Box<dyn std::error::Error>> {
        unsafe {
            let id = gl::CreateProgram();
            if id == 0 {
                return Err("Failed to create shader program".into());
            }

            for shader in shaders {
                shader.attach(id);
            }

            gl::LinkProgram(id);

            let mut success = gl::FALSE as i32;
            gl::GetProgramiv(id, gl::LINK_STATUS, &mut success);
            if success != gl::TRUE as i32 {
                let error_log = get_program_info_log(id);
                gl::DeleteProgram(id);
                return Err(format!("Program linking failed: {}", error_log).into());
            }

            gl::ValidateProgram(id);
            gl::GetProgramiv(id, gl::VALIDATE_STATUS, &mut success);
            if success != gl::TRUE as i32 {
                let error_log = get_program_info_log(id);
                log::warn!("Program validation warning: {}", error_log);
            }

            for shader in shaders {
                shader.detach(id);
                shader.delete();
            }

            Ok(Self {
                id,
                per_render_uniforms: Vec::new(),
                per_instance_uniforms: Vec::new(),
                bound_program: None,
            })
        }
    }

    pub fn from_sources(
        vertex_src: &str,
        fragment_src: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let vertex_shader = Shader::vertex(vertex_src)?;
        let fragment_shader = Shader::fragment(fragment_src)?;
        Self::new(&[vertex_shader, fragment_shader])
    }

    pub fn bind(&mut self) {
        if self.bound_program != Some(self.id) {
            unsafe { gl::UseProgram(self.id) };
            self.bound_program = Some(self.id);
        }
    }

    pub fn unbind(&mut self) {
        if self.bound_program != Some(0) {
            unsafe { gl::UseProgram(0) };
            self.bound_program = Some(0);
        }
    }

    pub fn add_per_render_uniform(
        &mut self,
        uniform: Rc<RefCell<dyn for<'a> Uniform<RenderState<'a, T>>>>,
    ) {
        self.bind();
        uniform.borrow_mut().initialize(self.id);
        self.per_render_uniforms.push(uniform);
    }

    pub fn add_per_instance_uniform(
        &mut self,
        uniform: Rc<RefCell<dyn for<'a> Uniform<RenderState<'a, T>>>>,
    ) {
        self.bind();
        uniform.borrow_mut().initialize(self.id);
        self.per_instance_uniforms.push(uniform);
    }

    pub fn update_per_render_uniforms<'a>(&self, state: &RenderState<'a, T>) {
        for uniform in &self.per_render_uniforms {
            uniform.borrow().load(state);
        }
    }

    pub fn update_per_instance_uniforms<'a>(&self, state: &RenderState<'a, T>) {
        for uniform in &self.per_instance_uniforms {
            uniform.borrow().load(state);
        }
    }

    pub fn get_uniform_location(&self, name: &str) -> i32 {
        let cname = CString::new(name).unwrap();
        unsafe { gl::GetUniformLocation(self.id, cname.as_ptr()) }
    }

    pub fn bind_attribute(&self, location: u32, name: &str) {
        let cname = CString::new(name).unwrap();
        unsafe { gl::BindAttribLocation(self.id, location, cname.as_ptr()) };
    }

    pub fn id(&self) -> u32 {
        self.id
    }
}

impl<T: Processable> Drop for ShaderProgram<T> {
    fn drop(&mut self) {
        unsafe { gl::DeleteProgram(self.id) };
    }
}

fn get_program_info_log(program_id: u32) -> String {
    unsafe {
        let mut len = 0;
        gl::GetProgramiv(program_id, gl::INFO_LOG_LENGTH, &mut len);
        if len > 0 {
            let mut buffer: Vec<u8> = vec![0; len as usize];
            gl::GetProgramInfoLog(
                program_id,
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
