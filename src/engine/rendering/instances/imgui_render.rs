//! Dear ImGui renderer using the project's OpenGL abstractions.
//!
//! Uses SimpleProgram, SimpleTexture, Vao, Vbo and Attribute from crate::opengl.

use crate::opengl::{
    constants::{data_type::DataType, vbo_target::VboTarget, vbo_usage::VboUsage},
    fbos::simple_texture::SimpleTexture,
    objects::{attribute::Attribute, vao::Vao, vbo::Vbo},
    shaders::SimpleProgram,
    textures::{
        texture::Texture,
        texture_configs::TextureConfigs,
        parameters::{
            mag_filter_parameter::MagFilterParameter,
            min_filter_parameter::MinFilterParameter,
        },
    },
};
use imgui::{DrawData, DrawIdx, DrawVert, TextureId};
use std::mem;

const VERT_SRC: &str = r#"#version 300 es
precision mediump float;
layout(location=0) in vec2 aPos;
layout(location=1) in vec2 aUV;
layout(location=2) in vec4 aColor;
uniform mat4 uProjMtx;
out vec2 vUV;
out vec4 vColor;
void main() {
    vUV    = aUV;
    vColor = aColor;
    gl_Position = uProjMtx * vec4(aPos, 0.0, 1.0);
}
"#;

const FRAG_SRC: &str = r#"#version 300 es
precision mediump float;
in vec2  vUV;
in vec4  vColor;
uniform sampler2D uTexture;
out vec4 fragColor;
void main() { fragColor = vColor * texture(uTexture, vUV); }
"#;

pub struct ImguiGlRenderer {
    program:   SimpleProgram,
    loc_proj:  i32,
    loc_tex:   i32,
    vao:       Vao,
    vbo:       Vbo,          // vertex buffer (streaming)
    ebo:       Vbo,          // element/index buffer (streaming)
    font_tex:  SimpleTexture,
}

impl ImguiGlRenderer {
    pub fn new(imgui: &mut imgui::Context) -> Result<Self, String> {
        let program  = SimpleProgram::from_sources(VERT_SRC, FRAG_SRC)?;
        let loc_proj = program.uniform_location("uProjMtx");
        let loc_tex  = program.uniform_location("uTexture");

        // DrawVert layout: pos(f32×2) + uv(f32×2) + col(u8×4) = 20 bytes
        let stride = mem::size_of::<DrawVert>() as i32;

        // ── VAO must be bound FIRST, then VBOs created and bound inside it ──
        // On GLES the EBO binding is part of VAO state — binding it before
        // the VAO is active means it never gets captured, causing draw call
        // failures (GL_INVALID_OPERATION) at runtime, especially on Adreno.
        let vao = Vao::create(); // creates and auto-binds the VAO

        let vbo = Vbo::create(VboTarget::ArrayBuffer,        VboUsage::StreamDraw);
        let ebo = Vbo::create(VboTarget::ElementArrayBuffer, VboUsage::StreamDraw);

        // Bind both buffers while the VAO is active so their state is captured.
        vbo.bind();
        ebo.bind(); // EBO captured into VAO here — critical for GLES

        // Set up vertex attribute pointers (VAO still bound).
        let a_pos = Attribute::of(0, 2, DataType::Float, false);
        let a_uv  = Attribute::of(1, 2, DataType::Float, false);
        let a_col = Attribute::of(2, 4, DataType::UByte, true);  // normalised u8 colour
        a_pos.link(stride, 0);
        a_uv.link(stride, 8);
        a_col.link(stride, 16);
        Attribute::enable_index(0);
        Attribute::enable_index(1);
        Attribute::enable_index(2);

        // Unbind in the right order: VAO first, then VBOs (so unbinding VBO
        // doesn't accidentally clear the VAO's buffer bindings on some drivers).
        vao.unbind();
        vbo.unbind();
        // NOTE: do NOT unbind the EBO while the VAO is unbound — on some GLES
        // drivers that can corrupt the VAO's element buffer reference. We leave
        // the EBO bound to GL_ELEMENT_ARRAY_BUFFER globally (harmless) and let
        // the next vao.bind() restore the correct association.

        // ── Font texture via SimpleTexture ────────────────────────────────
        let mut font_tex = SimpleTexture::create();
        font_tex.bind();

        // Apply linear filtering via TextureConfigs
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

        // Upload atlas pixels
        let atlas = imgui.fonts();
        let tex   = atlas.build_rgba32_texture();
        font_tex.upload_rgba8(tex.width as i32, tex.height as i32, tex.data);
        font_tex.unbind();

        imgui.fonts().tex_id = TextureId::new(font_tex.get_id() as usize);

        Ok(Self { program, loc_proj, loc_tex, vao, vbo, ebo, font_tex })
    }

    pub fn render(&mut self, draw_data: &DrawData) {
        let fb_w = draw_data.display_size[0] * draw_data.framebuffer_scale[0];
        let fb_h = draw_data.display_size[1] * draw_data.framebuffer_scale[1];
        if fb_w <= 0.0 || fb_h <= 0.0 { return; }

        let [dl, dt] = draw_data.display_pos;
        let [dr, db] = [dl + draw_data.display_size[0], dt + draw_data.display_size[1]];

        #[rustfmt::skip]
        let proj: [f32; 16] = [
             2.0/(dr-dl),       0.0,          0.0, 0.0,
             0.0,         2.0/(dt-db),         0.0, 0.0,
             0.0,               0.0,          -1.0, 0.0,
            (dr+dl)/(dl-dr),(dt+db)/(db-dt),   0.0, 1.0,
        ];

        // Save scissor state
        let mut prev_scissor = [0i32; 4];
        let prev_scissor_test = unsafe { gl::IsEnabled(gl::SCISSOR_TEST) };
        unsafe { gl::GetIntegerv(gl::SCISSOR_BOX, prev_scissor.as_mut_ptr()); }

        // Set render state
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

        self.program.bind();
        self.program.set_uniform_mat4(self.loc_proj, &proj);
        self.program.set_uniform_int(self.loc_tex, 0);
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

                        unsafe { gl::Scissor(cx as i32, (fb_h-ch) as i32, (cw-cx) as i32, (ch-cy) as i32); }

                        // Bind the texture for this draw call (font or user texture)
                        let tex_id = cmd_params.texture_id.id() as u32;
                        unsafe {
                            gl::ActiveTexture(gl::TEXTURE0);
                            gl::BindTexture(gl::TEXTURE_2D, tex_id);
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

        // Restore state
        self.vao.unbind();
        self.program.unbind();
        unsafe {
            gl::Disable(gl::BLEND);
            gl::Enable(gl::CULL_FACE);
            gl::Enable(gl::DEPTH_TEST);
            if prev_scissor_test == gl::FALSE { gl::Disable(gl::SCISSOR_TEST); }
            gl::Scissor(prev_scissor[0], prev_scissor[1], prev_scissor[2], prev_scissor[3]);
        }
    }
}
