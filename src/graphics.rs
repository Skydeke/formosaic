use cgmath::{Matrix, Matrix4};
use std::ffi::CString;
use std::mem;
use std::ptr;

pub struct Shader {
    pub program: u32,
}

fn get_shader_info_log(shader: u32) -> String {
    unsafe {
        let mut len = 0;
        gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
        if len > 0 {
            let mut buffer: Vec<u8> = vec![0; len as usize];
            gl::GetShaderInfoLog(shader, len, ptr::null_mut(), buffer.as_mut_ptr() as *mut _);
            if let Some(&0) = buffer.last() {
                buffer.pop();
            }
            String::from_utf8_lossy(&buffer).into_owned()
        } else {
            String::new()
        }
    }
}

fn get_program_info_log(program: u32) -> String {
    unsafe {
        let mut len = 0;
        gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
        if len > 0 {
            let mut buffer: Vec<u8> = vec![0; len as usize];
            gl::GetProgramInfoLog(program, len, ptr::null_mut(), buffer.as_mut_ptr() as *mut _);
            if let Some(&0) = buffer.last() {
                buffer.pop();
            }
            String::from_utf8_lossy(&buffer).into_owned()
        } else {
            String::new()
        }
    }
}

impl Shader {
    pub fn new(vertex_src: &str, fragment_src: &str) -> Result<Self, String> {
        unsafe {
            let vertex_shader = gl::CreateShader(gl::VERTEX_SHADER);
            let vertex_src_c = CString::new(vertex_src).unwrap();
            gl::ShaderSource(vertex_shader, 1, &vertex_src_c.as_ptr(), ptr::null());
            gl::CompileShader(vertex_shader);

            let mut success = gl::FALSE as i32;
            gl::GetShaderiv(vertex_shader, gl::COMPILE_STATUS, &mut success);
            if success != gl::TRUE as i32 {
                return Err(format!(
                    "Vertex shader compilation failed: {}",
                    get_shader_info_log(vertex_shader)
                ));
            }

            let fragment_shader = gl::CreateShader(gl::FRAGMENT_SHADER);
            let fragment_src_c = CString::new(fragment_src).unwrap();
            gl::ShaderSource(fragment_shader, 1, &fragment_src_c.as_ptr(), ptr::null());
            gl::CompileShader(fragment_shader);

            gl::GetShaderiv(fragment_shader, gl::COMPILE_STATUS, &mut success);
            if success != gl::TRUE as i32 {
                return Err(format!(
                    "Fragment shader compilation failed: {}",
                    get_shader_info_log(fragment_shader)
                ));
            }

            let program = gl::CreateProgram();
            gl::AttachShader(program, vertex_shader);
            gl::AttachShader(program, fragment_shader);
            gl::LinkProgram(program);

            gl::GetProgramiv(program, gl::LINK_STATUS, &mut success);
            if success != gl::TRUE as i32 {
                return Err(format!(
                    "Shader program linking failed: {}",
                    get_program_info_log(program)
                ));
            }

            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(fragment_shader);
            Ok(Shader { program })
        }
    }

    pub fn use_program(&self) {
        unsafe {
            gl::UseProgram(self.program);
        }
    }

    pub fn set_matrix4(&self, name: &str, matrix: &Matrix4<f32>) {
        unsafe {
            let name_cstring = CString::new(name).unwrap();
            let location = gl::GetUniformLocation(self.program, name_cstring.as_ptr());
            if location >= 0 {
                gl::UniformMatrix4fv(location, 1, gl::FALSE, matrix.as_ptr());
            } else {
                log::warn!("Uniform '{}' not found in shader", name);
            }
        }
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.program);
        }
    }
}

pub struct Renderer {
    shader: Shader,
    vao: u32,
    vbo: u32,
}

impl Renderer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        log::info!("Initializing renderer");

        let vertex_shader_source = r#"
            #version 100
            precision mediump float;
            attribute vec3 aPos;
            attribute vec3 aColor;
            uniform mat4 uMVP;
            varying vec3 FragColor;
            void main() {
                FragColor = aColor;
                gl_Position = uMVP * vec4(aPos, 1.0);
            }
        "#;

        let fragment_shader_source = r#"
            #version 100
            precision mediump float;
            varying vec3 FragColor;
            void main() {
                gl_FragColor = vec4(FragColor, 1.0);
            }
        "#;

        let shader = Shader::new(vertex_shader_source, fragment_shader_source)?;

        let vertices: [f32; 18] = [
            -0.5, -0.5, 0.0, 1.0, 0.0, 0.0, // left-bottom: red
            0.5, -0.5, 0.0, 0.0, 1.0, 0.0, // right-bottom: green
            0.0, 0.5, 0.0, 0.0, 0.0, 1.0, // top: blue
        ];

        let mut vao = 0;
        let mut vbo = 0;

        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);

            gl::BindVertexArray(vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * mem::size_of::<f32>()) as isize,
                vertices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            // query actual attribute locations from the shader
            let pos_loc =
                gl::GetAttribLocation(shader.program, CString::new("aPos").unwrap().as_ptr());
            let color_loc =
                gl::GetAttribLocation(shader.program, CString::new("aColor").unwrap().as_ptr());

            if pos_loc >= 0 {
                gl::VertexAttribPointer(
                    pos_loc as u32,
                    3,
                    gl::FLOAT,
                    gl::FALSE,
                    (6 * mem::size_of::<f32>()) as i32,
                    0 as *const _,
                );
                gl::EnableVertexAttribArray(pos_loc as u32);
            }

            if color_loc >= 0 {
                gl::VertexAttribPointer(
                    color_loc as u32,
                    3,
                    gl::FLOAT,
                    gl::FALSE,
                    (6 * mem::size_of::<f32>()) as i32,
                    (3 * mem::size_of::<f32>()) as *const _,
                );
                gl::EnableVertexAttribArray(color_loc as u32);
            }

            gl::BindVertexArray(0);
        }

        log::info!("Renderer initialized successfully");
        Ok(Renderer { shader, vao, vbo })
    }

    pub fn render_triangle(&mut self, mvp: &Matrix4<f32>) {
        unsafe {
            self.shader.use_program();
            self.shader.set_matrix4("uMVP", mvp);
            gl::BindVertexArray(self.vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 3);
            gl::BindVertexArray(0);
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteVertexArrays(1, &self.vao);
            gl::DeleteBuffers(1, &self.vbo);
        }
    }
}
