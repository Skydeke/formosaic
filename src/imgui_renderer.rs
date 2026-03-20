//! Minimal Dear ImGui renderer for OpenGL ES 3.0.
//!
//! Compiles the imgui draw data into a single VAO+VBO+EBO and submits
//! it with scissor-clipped draw calls — exactly how the reference
//! imgui_impl_opengl3.cpp works, adapted for GLES 3.0 and the raw `gl` crate.

use imgui::{DrawData, DrawIdx, DrawVert, TextureId};
use imgui::internal::RawWrapper;

macro_rules! offset_of {
    ($ty:ty, $field:ident) => {{
        let base = std::mem::MaybeUninit::<$ty>::uninit();
        let base_ptr = base.as_ptr() as *const u8;
        let field_ptr = unsafe { std::ptr::addr_of!((*base.as_ptr()).$field) as *const u8 };
        unsafe { field_ptr.offset_from(base_ptr) as usize }
    }};
}
use std::ffi::CString;
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
void main() {
    fragColor = vColor * texture(uTexture, vUV);
}
"#;

pub struct ImguiGlRenderer {
    prog:      u32,
    loc_proj:  i32,
    loc_tex:   i32,
    vao:       u32,
    vbo:       u32,
    ebo:       u32,
    font_tex:  u32,
}

impl ImguiGlRenderer {
    pub fn new(imgui: &mut imgui::Context) -> Result<Self, String> {
        // ── Compile shader ────────────────────────────────────────────────
        let prog = unsafe {
            let v = compile_shader(gl::VERTEX_SHADER,   VERT_SRC)?;
            let f = compile_shader(gl::FRAGMENT_SHADER, FRAG_SRC)?;
            let p = gl::CreateProgram();
            gl::AttachShader(p, v); gl::AttachShader(p, f);
            gl::LinkProgram(p);
            let mut ok = 0i32;
            gl::GetProgramiv(p, gl::LINK_STATUS, &mut ok);
            gl::DeleteShader(v); gl::DeleteShader(f);
            if ok != gl::TRUE as i32 { return Err("imgui shader link failed".into()); }
            p
        };
        let loc_proj = uniform_loc(prog, "uProjMtx");
        let loc_tex  = uniform_loc(prog, "uTexture");

        // ── VAO + streaming VBO/EBO ───────────────────────────────────────
        let (vao, vbo, ebo) = unsafe {
            let mut ids = [0u32; 3];
            gl::GenVertexArrays(1, &mut ids[0]);
            gl::GenBuffers(1, &mut ids[1]);
            gl::GenBuffers(1, &mut ids[2]);

            gl::BindVertexArray(ids[0]);
            gl::BindBuffer(gl::ARRAY_BUFFER, ids[1]);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ids[2]);

            let stride = mem::size_of::<DrawVert>() as i32;
            // pos: vec2 @ offset 0
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, stride,
                offset_of!(DrawVert, pos) as *const _);
            // uv: vec2 @ offset 8
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, stride,
                offset_of!(DrawVert, uv) as *const _);
            // col: vec4 u8 @ offset 16, normalised
            gl::EnableVertexAttribArray(2);
            gl::VertexAttribPointer(2, 4, gl::UNSIGNED_BYTE, gl::TRUE, stride,
                offset_of!(DrawVert, col) as *const _);

            gl::BindVertexArray(0);
            (ids[0], ids[1], ids[2])
        };

        // ── Font texture ──────────────────────────────────────────────────
        let font_tex = unsafe {
            let atlas   = imgui.fonts();
            let tex     = atlas.build_rgba32_texture();
            let mut id  = 0u32;
            gl::GenTextures(1, &mut id);
            gl::BindTexture(gl::TEXTURE_2D, id);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            gl::TexImage2D(
                gl::TEXTURE_2D, 0, gl::RGBA as i32,
                tex.width as i32, tex.height as i32, 0,
                gl::RGBA, gl::UNSIGNED_BYTE,
                tex.data.as_ptr() as *const _,
            );
            gl::BindTexture(gl::TEXTURE_2D, 0);
            id
        };
        imgui.fonts().tex_id = TextureId::new(font_tex as usize);

        Ok(Self { prog, loc_proj, loc_tex, vao, vbo, ebo, font_tex })
    }

    pub fn render(&self, draw_data: &DrawData) {
        let fb_w = draw_data.display_size[0] * draw_data.framebuffer_scale[0];
        let fb_h = draw_data.display_size[1] * draw_data.framebuffer_scale[1];
        if fb_w <= 0.0 || fb_h <= 0.0 { return; }

        let [dl, dt] = draw_data.display_pos;
        let [dr, db] = [dl + draw_data.display_size[0], dt + draw_data.display_size[1]];

        // Orthographic projection
        #[rustfmt::skip]
        let proj: [f32; 16] = [
             2.0/(dr-dl),  0.0,           0.0, 0.0,
             0.0,          2.0/(dt-db),   0.0, 0.0,
             0.0,          0.0,          -1.0, 0.0,
            (dr+dl)/(dl-dr),(dt+db)/(db-dt), 0.0, 1.0,
        ];

        unsafe {
            // Save state
            let mut prev_scissor = [0i32; 4];
            let prev_scissor_test = gl::IsEnabled(gl::SCISSOR_TEST);
            gl::GetIntegerv(gl::SCISSOR_BOX, prev_scissor.as_mut_ptr());

            gl::Enable(gl::BLEND);
            gl::BlendEquation(gl::FUNC_ADD);
            gl::BlendFuncSeparate(
                gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA,
                gl::ONE,       gl::ONE_MINUS_SRC_ALPHA,
            );
            gl::Disable(gl::CULL_FACE);
            gl::Disable(gl::DEPTH_TEST);
            gl::Disable(gl::STENCIL_TEST);
            gl::Enable(gl::SCISSOR_TEST);

            gl::UseProgram(self.prog);
            if self.loc_proj >= 0 {
                gl::UniformMatrix4fv(self.loc_proj, 1, gl::FALSE, proj.as_ptr());
            }
            if self.loc_tex >= 0 {
                gl::Uniform1i(self.loc_tex, 0);
            }
            gl::BindVertexArray(self.vao);

            let clip_off   = draw_data.display_pos;
            let clip_scale = draw_data.framebuffer_scale;

            for draw_list in draw_data.draw_lists() {
                let vtx = draw_list.vtx_buffer();
                let idx = draw_list.idx_buffer();

                gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
                gl::BufferData(
                    gl::ARRAY_BUFFER,
                    (vtx.len() * mem::size_of::<DrawVert>()) as isize,
                    vtx.as_ptr() as *const _,
                    gl::STREAM_DRAW,
                );
                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.ebo);
                gl::BufferData(
                    gl::ELEMENT_ARRAY_BUFFER,
                    (idx.len() * mem::size_of::<DrawIdx>()) as isize,
                    idx.as_ptr() as *const _,
                    gl::STREAM_DRAW,
                );

                for cmd in draw_list.commands() {
                    match cmd {
                        imgui::DrawCmd::Elements { count, cmd_params } => {
                            let clip_min_x = (cmd_params.clip_rect[0] - clip_off[0]) * clip_scale[0];
                            let clip_min_y = (cmd_params.clip_rect[1] - clip_off[1]) * clip_scale[1];
                            let clip_max_x = (cmd_params.clip_rect[2] - clip_off[0]) * clip_scale[0];
                            let clip_max_y = (cmd_params.clip_rect[3] - clip_off[1]) * clip_scale[1];
                            if clip_max_x <= clip_min_x || clip_max_y <= clip_min_y { continue; }

                            gl::Scissor(
                                clip_min_x as i32,
                                (fb_h - clip_max_y) as i32,
                                (clip_max_x - clip_min_x) as i32,
                                (clip_max_y - clip_min_y) as i32,
                            );

                            let tex_id = cmd_params.texture_id.id() as u32;
                            gl::ActiveTexture(gl::TEXTURE0);
                            gl::BindTexture(gl::TEXTURE_2D, tex_id);

                            let idx_type = if mem::size_of::<DrawIdx>() == 2 {
                                gl::UNSIGNED_SHORT
                            } else {
                                gl::UNSIGNED_INT
                            };
                            gl::DrawElements(
                                gl::TRIANGLES,
                                count as i32,
                                idx_type,
                                (cmd_params.idx_offset * mem::size_of::<DrawIdx>()) as *const _,
                            );
                        }
                        imgui::DrawCmd::ResetRenderState => {}
                        imgui::DrawCmd::RawCallback { callback, raw_cmd } => {
                            callback(draw_list.raw(), raw_cmd);
                        }
                    }
                }
            }

            // Restore state
            gl::BindVertexArray(0);
            gl::UseProgram(0);
            gl::Disable(gl::BLEND);
            gl::Enable(gl::CULL_FACE);
            gl::Enable(gl::DEPTH_TEST);
            if prev_scissor_test == gl::FALSE {
                gl::Disable(gl::SCISSOR_TEST);
            }
            gl::Scissor(
                prev_scissor[0], prev_scissor[1],
                prev_scissor[2], prev_scissor[3],
            );
        }
    }
}

impl Drop for ImguiGlRenderer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.prog);
            gl::DeleteVertexArrays(1, &self.vao);
            gl::DeleteBuffers(1, &self.vbo);
            gl::DeleteBuffers(1, &self.ebo);
            gl::DeleteTextures(1, &self.font_tex);
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

unsafe fn compile_shader(kind: u32, src: &str) -> Result<u32, String> {
    let id  = gl::CreateShader(kind);
    let c   = CString::new(src).unwrap();
    gl::ShaderSource(id, 1, &c.as_ptr(), std::ptr::null());
    gl::CompileShader(id);
    let mut ok = 0i32;
    gl::GetShaderiv(id, gl::COMPILE_STATUS, &mut ok);
    if ok != gl::TRUE as i32 {
        let mut len = 0i32;
        gl::GetShaderiv(id, gl::INFO_LOG_LENGTH, &mut len);
        let mut buf = vec![0u8; len as usize];
        gl::GetShaderInfoLog(id, len, std::ptr::null_mut(), buf.as_mut_ptr() as *mut _);
        gl::DeleteShader(id);
        return Err(format!("imgui shader compile: {}", String::from_utf8_lossy(&buf)));
    }
    Ok(id)
}

fn uniform_loc(prog: u32, name: &str) -> i32 {
    let c = CString::new(name).unwrap();
    unsafe { gl::GetUniformLocation(prog, c.as_ptr()) }
}


