use crate::shaders::{uniform::UniformAdapter, RenderState, ShaderProgram, UniformMatrix4};
use cgmath::Matrix4;

pub struct Renderer {
    shader_program: ShaderProgram,
    vao: u32,
    vbo: u32,
}

// Embed shaders at compile-time
const VERTEX_SHADER_SOURCE: &str = include_str!("../assets/shaders/basic.vert.glsl");
const FRAGMENT_SHADER_SOURCE: &str = include_str!("../assets/shaders/basic.frag.glsl");

impl Renderer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        log::info!("Initializing renderer");

        // Create ShaderProgram using the new system
        let mut shader_program =
            ShaderProgram::from_sources(VERTEX_SHADER_SOURCE, FRAGMENT_SHADER_SOURCE)?;

        // Add MVP uniform as per-instance uniform (changes for each object rendered)
        shader_program.add_per_instance_uniform(Box::new(UniformAdapter {
            uniform: UniformMatrix4::new("uMVP"),
            extractor: Box::new(|state: &RenderState| state.mvp_matrix),
        }));

        // Vertex data
        let vertices: [f32; 18] = [
            -0.5, -0.5, 0.0, 1.0, 0.0, 0.0, // Bottom left - Red
            0.5, -0.5, 0.0, 0.0, 1.0, 0.0, // Bottom right - Green
            0.0, 0.5, 0.0, 0.0, 0.0, 1.0, // Top center - Blue
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
                (vertices.len() * std::mem::size_of::<f32>()) as isize,
                vertices.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            // Position attribute
            gl::VertexAttribPointer(
                0,
                3,
                gl::FLOAT,
                gl::FALSE,
                6 * std::mem::size_of::<f32>() as i32,
                0 as *const _,
            );
            gl::EnableVertexAttribArray(0);

            // Color attribute
            gl::VertexAttribPointer(
                1,
                3,
                gl::FLOAT,
                gl::FALSE,
                6 * std::mem::size_of::<f32>() as i32,
                (3 * std::mem::size_of::<f32>()) as *const _,
            );
            gl::EnableVertexAttribArray(1);

            gl::BindVertexArray(0);
        }

        log::info!("Renderer initialized successfully");
        Ok(Self {
            shader_program,
            vao,
            vbo,
        })
    }

    pub fn render_triangle(&mut self, mvp: &Matrix4<f32>) {
        // Create render state
        let mut render_state = RenderState::new();
        render_state.mvp_matrix = *mvp;

        self.shader_program.bind();

        // Update per-instance uniforms (MVP matrix in this case)
        self.shader_program
            .update_per_instance_uniforms(&render_state);

        unsafe {
            gl::BindVertexArray(self.vao);
            gl::DrawArrays(gl::TRIANGLES, 0, 3);
            gl::BindVertexArray(0);
        }

        self.shader_program.unbind();
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
