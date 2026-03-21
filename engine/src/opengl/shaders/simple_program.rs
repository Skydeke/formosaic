//! A compiled and linked shader program without the type-parameterised uniform
//! system of `ShaderProgram<T>`.
//!
//! Used only when the uniform system cannot apply — e.g. highly imperative
//! per-draw-call rendering (imgui's per-draw-list scissor/texture loop).
//! All game and engine renderers must use `ShaderProgram<T>` + `UniformAdapter`.

use crate::opengl::shaders::shader::Shader;

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

    pub fn bind(&self)   { unsafe { gl::UseProgram(self.id); } }
    pub fn unbind(&self) { unsafe { gl::UseProgram(0); } }
    pub fn id(&self) -> u32 { self.id }
}

impl Drop for SimpleProgram {
    fn drop(&mut self) {
        unsafe { gl::DeleteProgram(self.id); }
    }
}
