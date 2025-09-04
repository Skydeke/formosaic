use crate::mesh::Mesh;
use crate::opengl::constants::data_type::DataType;
use crate::opengl::constants::render_mode::RenderMode;
use crate::opengl::constants::vbo_usage::VboUsage;
use crate::opengl::objects::attribute::Attribute;
use crate::opengl::objects::data_buffer::DataBuffer;
use crate::opengl::objects::index_buffer::IndexBuffer;
use crate::opengl::objects::vao::Vao;
use crate::opengl::shaders::{uniform::UniformAdapter, RenderState, ShaderProgram, UniformMatrix4};
use crate::simple_model::SimpleModel;

use cgmath::Matrix4;
use std::rc::Rc;

pub struct Renderer {
    shader_program: ShaderProgram,
    model: SimpleModel,
}

impl Renderer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        log::info!("Initializing Renderer");

        let vertex_src = include_str!("../assets/shaders/basic.vert.glsl");
        let fragment_src = include_str!("../assets/shaders/basic.frag.glsl");

        let mut shader_program = ShaderProgram::from_sources(vertex_src, fragment_src)?;

        shader_program.add_per_instance_uniform(Box::new(UniformAdapter {
            uniform: UniformMatrix4::new("uMVP"),
            extractor: Box::new(|state: &RenderState| state.mvp_matrix),
        }));

        let positions: [f32; 9] = [-0.5, -0.5, 0.0, 0.5, -0.5, 0.0, 0.0, 0.5, 0.0];
        let colors: [f32; 9] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        let indicies: [i32; 3] = [0, 1, 2];

        let mut pos_buffer = DataBuffer::new(VboUsage::StaticDraw);
        pos_buffer.store_float(0, &positions);

        let mut color_buffer = DataBuffer::new(VboUsage::StaticDraw);
        color_buffer.store_float(0, &colors);

        let mut indicies_buffer = IndexBuffer::new(VboUsage::StaticDraw);
        indicies_buffer.store_int(0, &indicies);

        let mut vao = Vao::create();
        // Position attribute -> VBO 0
        let pos_attr = Attribute::of(0, 3, DataType::Float, false);
        vao.load_data_buffer(Rc::new(pos_buffer), &[pos_attr]);
        // Color attribute -> VBO 1
        let color_attr = Attribute::of(1, 3, DataType::Float, false);
        vao.load_data_buffer(Rc::new(color_buffer), &[color_attr]);
        vao.load_index_buffer(Rc::new(indicies_buffer), true);

        let mesh = Mesh::from_vao(vao);
        let model = SimpleModel::with_bounds(vec![mesh], RenderMode::Triangles);

        log::log!(log::Level::Info, "Renderer initialized successfully");
        Ok(Self {
            shader_program,
            model,
        })
    }

    pub fn render_triangle(&mut self, mvp: &Matrix4<f32>) {
        let mut render_state = RenderState::new();
        render_state.mvp_matrix = *mvp;

        self.shader_program.bind();
        self.shader_program
            .update_per_instance_uniforms(&render_state);

        let mesh_count = self.model.meshes().len();
        for i in 0..mesh_count {
            self.model.bind_and_configure(i);
            self.model.render(&render_state, i);
            self.model.unbind(i);
        }

        self.shader_program.unbind();
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        log::info!("Renderer dropped");
    }
}
