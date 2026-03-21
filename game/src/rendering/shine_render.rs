//! Solve shine renderer — Overlay pass.
//!
//! Uses ShaderProgram<NoopProcessable> + UniformAdapter closures over a shared
//! FrameState — the same pattern as EntityRenderer and LightingPass.
//! No raw gl::Uniform* calls, no uniform_location.

use formosaic_engine::{
    architecture::scene::scene_context::SceneContext,
    rendering::abstracted::{
        irenderer::{IRenderer, RenderPass},
        processable::NoopProcessable,
    },
    opengl::{
        constants::data_type::DataType,
        fbos::simple_texture::SimpleTexture,
        objects::{attribute::Attribute, data_buffer::DataBuffer, ivbo::IVbo, vao::Vao},
        shaders::{
            uniform::{UniformAdapter, UniformFloat, UniformTexture},
            RenderState, ShaderProgram,
        },
        textures::texture::Texture,
    },
};
use std::{cell::RefCell, rc::Rc};

const DEFAULT_VERT: &str = include_str!("../../assets/shaders/shine.vert.glsl");
const DEFAULT_FRAG: &str = include_str!("../../assets/shaders/shine.frag.glsl");

struct FrameState {
    solved_timer:    f32,  // negative = not solved
    position_tex_id: u32,
    albedo_tex_id:   u32,
}

pub struct ShineRenderer {
    vao:    Vao,
    shader: ShaderProgram<NoopProcessable>,
    frame:  Rc<RefCell<FrameState>>,
}

impl ShineRenderer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_shaders(DEFAULT_VERT, DEFAULT_FRAG)
    }

    pub fn with_shaders(vert: &str, frag: &str) -> Result<Self, Box<dyn std::error::Error>> {
        #[rustfmt::skip]
        let verts: [f32; 12] = [
            -1.0, -1.0,  1.0, -1.0,  1.0,  1.0,
            -1.0, -1.0,  1.0,  1.0, -1.0,  1.0,
        ];
        let buf = DataBuffer::load_static(&verts);
        let mut vao = Vao::create();
        vao.load_data_buffer(
            Rc::new(buf) as Rc<dyn IVbo>,
            &[Attribute::of(0, 2, DataType::Float, false)],
        );

        let frame = Rc::new(RefCell::new(FrameState {
            solved_timer:    -1.0,
            position_tex_id: 0,
            albedo_tex_id:   0,
        }));

        let mut shader = ShaderProgram::<NoopProcessable>::from_sources(vert, frag)?;

        // uTime — drives the sweep animation.
        {
            let f = Rc::clone(&frame);
            shader.add_per_render_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform: UniformFloat::new("uTime"),
                extractor: Box::new(move |_: &RenderState<NoopProcessable>| {
                    f.borrow().solved_timer
                }),
            })));
        }
        // uPosition — world-space position G-buffer, sampler on unit 0.
        {
            let f = Rc::clone(&frame);
            shader.add_per_render_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform: UniformTexture::new("uPosition", 0),
                extractor: Box::new(move |_: &RenderState<NoopProcessable>| {
                    let id = f.borrow().position_tex_id;
                    if id == 0 { None }
                    else { Some(Rc::new(SimpleTexture::new(id)) as Rc<dyn Texture>) }
                }),
            })));
        }
        // uAlbedo — albedo G-buffer (alpha channel = model mask), sampler on unit 1.
        {
            let f = Rc::clone(&frame);
            shader.add_per_render_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform: UniformTexture::new("uAlbedo", 1),
                extractor: Box::new(move |_: &RenderState<NoopProcessable>| {
                    let id = f.borrow().albedo_tex_id;
                    if id == 0 { None }
                    else { Some(Rc::new(SimpleTexture::new(id)) as Rc<dyn Texture>) }
                }),
            })));
        }

        log::info!("ShineRenderer initialised");
        Ok(Self { vao, shader, frame })
    }
}

impl IRenderer for ShineRenderer {
    fn pass(&self) -> RenderPass { RenderPass::Overlay }

    fn render(&mut self, context: &SceneContext) {
        let timer  = match context.solved_timer { Some(t) => t, None => return };
        let output = match context.output_data() { Some(d) => d, None => return };

        // Write current frame's texture IDs and timer into shared state so the
        // UniformAdapter extractors pick them up in update_per_render_uniforms.
        {
            let mut f = self.frame.borrow_mut();
            f.solved_timer    = timer;
            f.position_tex_id = output.position.get_id();
            f.albedo_tex_id   = output.colour.get_id();
        }

        unsafe {
            gl::Disable(gl::DEPTH_TEST);
            gl::Disable(gl::CULL_FACE);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }

        self.shader.bind();
        let state = RenderState::new_screenspace(self);
        // UniformTexture::load() now always calls bind_to_unit() regardless of
        // whether the driver reports location -1 — texture binding is global GL
        // state separated from the sampler-unit Uniform1i declaration.
        self.shader.update_per_render_uniforms(&state);

        self.vao.bind();
        self.vao.enable_attributes();
        unsafe { gl::DrawArrays(gl::TRIANGLES, 0, 6); }
        self.vao.unbind();
        self.shader.unbind();

        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::Enable(gl::CULL_FACE);
            gl::Disable(gl::BLEND);
            // Clean up texture units.
            gl::ActiveTexture(gl::TEXTURE1);
            gl::BindTexture(gl::TEXTURE_2D, 0);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }
    }

    fn any_processed(&self) -> bool { self.frame.borrow().solved_timer >= 0.0 }
    fn finish(&mut self) {}
}
