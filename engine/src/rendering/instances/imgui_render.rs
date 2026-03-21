//! Dear ImGui GL renderer — implements `IRenderer`.
//!
//! Uses ShaderProgram<NoopProcessable> + UniformAdapter — same pattern as
//! EntityRenderer. uTexture (sampler) is registered as a constant UniformInt.
//! uProjMtx is written into FrameState each frame and read by the extractor.

use crate::architecture::scene::scene_context::SceneContext;
use crate::rendering::abstracted::{
    irenderer::{IRenderer, RenderPass},
    processable::NoopProcessable,
};
use crate::opengl::{
    constants::{data_type::DataType, vbo_target::VboTarget, vbo_usage::VboUsage},
    fbos::simple_texture::SimpleTexture,
    objects::{attribute::Attribute, vao::Vao, vbo::Vbo},
    shaders::{
        uniform::{UniformAdapter, UniformInt, UniformMatrix4},
        RenderState, ShaderProgram,
    },
    textures::{
        texture::Texture,
        texture_configs::TextureConfigs,
        parameters::{
            mag_filter_parameter::MagFilterParameter,
            min_filter_parameter::MinFilterParameter,
        },
    },
};
use cgmath::Matrix4;
use imgui::{DrawData, DrawIdx, DrawVert, TextureId};
use std::{cell::RefCell, mem, rc::Rc};

struct FrameState {
    proj: Matrix4<f32>,
}

pub struct ImguiGlRenderer {
    shader:   ShaderProgram<NoopProcessable>,
    frame:    Rc<RefCell<FrameState>>,
    vao:      Vao,
    vbo:      Vbo,
    ebo:      Vbo,
    #[allow(dead_code)]
    font_tex: SimpleTexture,
}

const DEFAULT_VERT: &str = include_str!("../../../assets/shaders/imgui.vert.glsl");
const DEFAULT_FRAG: &str = include_str!("../../../assets/shaders/imgui.frag.glsl");

impl ImguiGlRenderer {
    pub fn new(imgui: &mut imgui::Context) -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_shaders(imgui, DEFAULT_VERT, DEFAULT_FRAG)
    }

    pub fn with_shaders(
        imgui: &mut imgui::Context,
        vert_src: &str,
        frag_src: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let frame = Rc::new(RefCell::new(FrameState {
            proj: Matrix4::from_scale(1.0),
        }));

        let mut shader = ShaderProgram::<NoopProcessable>::from_sources(vert_src, frag_src)?;

        // uProjMtx — rebuilt each frame from DrawData in draw().
        {
            let f = Rc::clone(&frame);
            shader.add_per_render_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform: UniformMatrix4::new("uProjMtx"),
                extractor: Box::new(move |_: &RenderState<NoopProcessable>| f.borrow().proj),
            })));
        }
        // uTexture — always unit 0, never changes.
        shader.add_per_render_uniform(Rc::new(RefCell::new(UniformAdapter {
            uniform: UniformInt::new("uTexture"),
            extractor: Box::new(|_: &RenderState<NoopProcessable>| 0i32),
        })));

        // DrawVert layout: pos(f32×2) + uv(f32×2) + col(u8×4) = 20 bytes
        let stride = mem::size_of::<DrawVert>() as i32;

        // VAO must be bound before EBO — on GLES the EBO binding is VAO state.
        let vao = Vao::create();
        let vbo = Vbo::create(VboTarget::ArrayBuffer,        VboUsage::StreamDraw);
        let ebo = Vbo::create(VboTarget::ElementArrayBuffer, VboUsage::StreamDraw);
        vbo.bind();
        ebo.bind(); // captured into VAO here

        let a_pos = Attribute::of(0, 2, DataType::Float, false);
        let a_uv  = Attribute::of(1, 2, DataType::Float, false);
        let a_col = Attribute::of(2, 4, DataType::UByte, true);
        a_pos.link(stride, 0);
        a_uv.link(stride, 8);
        a_col.link(stride, 16);
        Attribute::enable_index(0);
        Attribute::enable_index(1);
        Attribute::enable_index(2);

        vao.unbind();
        vbo.unbind();
        // Do NOT unbind EBO while VAO is unbound — corrupts VAO state on some drivers.

        // Font texture
        let mut font_tex = SimpleTexture::create();
        font_tex.bind();
        let configs = TextureConfigs {
            mag_filter: Some(MagFilterParameter::Linear),
            min_filter: Some(MinFilterParameter::Linear),
            mipmap: false,
            ..TextureConfigs::new(
                crate::opengl::constants::format_type::FormatType::Rgba,
                crate::opengl::constants::format_type::FormatType::Rgba,
                DataType::UByte,
            )
        };
        font_tex.apply_configs(&configs);
        let atlas = imgui.fonts();
        let tex   = atlas.build_rgba32_texture();
        font_tex.upload_rgba8(tex.width as i32, tex.height as i32, tex.data);
        font_tex.unbind();
        imgui.fonts().tex_id = TextureId::new(font_tex.get_id() as usize);

        Ok(Self { shader, frame, vao, vbo, ebo, font_tex })
    }

    fn draw(&mut self, draw_data: &DrawData) {
        let fb_w = draw_data.display_size[0] * draw_data.framebuffer_scale[0];
        let fb_h = draw_data.display_size[1] * draw_data.framebuffer_scale[1];
        if fb_w <= 0.0 || fb_h <= 0.0 { return; }

        let [dl, dt] = draw_data.display_pos;
        let [dr, db] = [dl + draw_data.display_size[0], dt + draw_data.display_size[1]];

        // Write the projection matrix into FrameState so the UniformAdapter extractor picks it up.
        self.frame.borrow_mut().proj = Matrix4::new(
             2.0/(dr-dl),        0.0,          0.0, 0.0,
             0.0,          2.0/(dt-db),         0.0, 0.0,
             0.0,                0.0,          -1.0, 0.0,
            (dr+dl)/(dl-dr), (dt+db)/(db-dt),   0.0, 1.0,
        );

        let mut prev_scissor = [0i32; 4];
        let prev_scissor_test = unsafe { gl::IsEnabled(gl::SCISSOR_TEST) };
        unsafe { gl::GetIntegerv(gl::SCISSOR_BOX, prev_scissor.as_mut_ptr()); }

        unsafe {
            gl::Enable(gl::BLEND);
            gl::BlendEquation(gl::FUNC_ADD);
            gl::BlendFuncSeparate(
                gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA,
                gl::ONE,       gl::ONE_MINUS_SRC_ALPHA,
            );
            gl::Disable(gl::CULL_FACE);
            gl::Disable(gl::DEPTH_TEST);
            gl::Enable(gl::SCISSOR_TEST);
        }

        self.shader.bind();
        let state = RenderState::new_screenspace(self);
        self.shader.update_per_render_uniforms(&state);
        self.vao.bind();

        let clip_off   = draw_data.display_pos;
        let clip_scale = draw_data.framebuffer_scale;

        for draw_list in draw_data.draw_lists() {
            self.vbo.store_raw(draw_list.vtx_buffer());
            self.ebo.store_raw(draw_list.idx_buffer());

            for cmd in draw_list.commands() {
                match cmd {
                    imgui::DrawCmd::Elements { count, cmd_params } => {
                        let cx = (cmd_params.clip_rect[0] - clip_off[0]) * clip_scale[0];
                        let cy = (cmd_params.clip_rect[1] - clip_off[1]) * clip_scale[1];
                        let cw = (cmd_params.clip_rect[2] - clip_off[0]) * clip_scale[0];
                        let ch = (cmd_params.clip_rect[3] - clip_off[1]) * clip_scale[1];
                        if cw <= cx || ch <= cy { continue; }

                        unsafe {
                            gl::Scissor(cx as i32, (fb_h - ch) as i32,
                                        (cw - cx) as i32, (ch - cy) as i32);
                            gl::ActiveTexture(gl::TEXTURE0);
                            gl::BindTexture(gl::TEXTURE_2D,
                                            cmd_params.texture_id.id() as u32);
                        }

                        let idx_type = if mem::size_of::<DrawIdx>() == 2 {
                            gl::UNSIGNED_SHORT
                        } else { gl::UNSIGNED_INT };

                        unsafe {
                            gl::DrawElements(
                                gl::TRIANGLES, count as i32, idx_type,
                                (cmd_params.idx_offset * mem::size_of::<DrawIdx>()) as *const _,
                            );
                        }
                    }
                    imgui::DrawCmd::ResetRenderState => {}
                    imgui::DrawCmd::RawCallback { callback, raw_cmd } => {
                        use imgui::internal::RawWrapper;
                        unsafe { callback(draw_list.raw(), raw_cmd); }
                    }
                }
            }
        }

        self.vao.unbind();
        self.shader.unbind();
        unsafe {
            gl::Disable(gl::BLEND);
            gl::Enable(gl::CULL_FACE);
            gl::Enable(gl::DEPTH_TEST);
            if prev_scissor_test == gl::FALSE { gl::Disable(gl::SCISSOR_TEST); }
            gl::Scissor(prev_scissor[0], prev_scissor[1], prev_scissor[2], prev_scissor[3]);
        }
    }
}

impl IRenderer for ImguiGlRenderer {
    fn pass(&self) -> RenderPass { RenderPass::Overlay }

    fn render(&mut self, context: &SceneContext) {
        if let Some(ptr) = context.imgui_draw_data {
            // SAFETY: pointer is set from a live &DrawData borrow that remains
            // valid for the entire frame — pipeline.draw() completes before
            // game_engine calls imgui_ctx.render() again.
            let draw_data = unsafe { &*ptr };
            self.draw(draw_data);
        }
    }

    fn any_processed(&self) -> bool { true }
    fn finish(&mut self) {}
}
