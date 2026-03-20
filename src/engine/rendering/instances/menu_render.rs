//! In-game menu overlay renderer — OpenGL ES 2.0 / 3.0 compatible.
//!
//! Uses a single persistent VAO+VBO with streaming updates.
//! All 2D primitives are batched into one vertex buffer per draw type
//! (filled TRIANGLES then LINE segments) and flushed with 2 draw calls.
//!
//! This avoids creating/destroying VAOs per primitive which causes
//! GL_INVALID_OPERATION on many GLES drivers.

use crate::engine::architecture::scene::scene_context::SceneContext;
use crate::engine::rendering::abstracted::irenderer::IRenderer;
use crate::level::storage::LevelMeta;
use crate::opengl::shaders::shader::Shader;

// ─── Colour palette ───────────────────────────────────────────────────────────
const BG:       [f32; 4] = [0.039, 0.043, 0.055, 0.97];
const SURFACE:  [f32; 4] = [0.063, 0.071, 0.102, 1.0];
const BORDER:   [f32; 4] = [0.118, 0.133, 0.208, 1.0];
const TEXT:     [f32; 4] = [0.816, 0.847, 0.941, 1.0];
const TEXT_DIM: [f32; 4] = [0.353, 0.376, 0.502, 1.0];
const ACCENT_R: [f32; 4] = [0.753, 0.188, 0.290, 1.0];
const ACCENT_B: [f32; 4] = [0.188, 0.306, 0.753, 1.0];
const SOLVED:   [f32; 4] = [0.180, 0.753, 0.502, 1.0];

fn diff_color(d: f32) -> [f32; 4] {
    match d {
        v if v < 0.25 => SOLVED,
        v if v < 0.50 => [0.753, 0.627, 0.188, 1.0],
        v if v < 0.75 => ACCENT_R,
        _             => [0.627, 0.188, 0.753, 1.0],
    }
}

// ─── Menu panel / frame data ──────────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuPanel { Levels, Online }

pub struct MenuFrameData<'a> {
    pub active:         bool,
    pub panel:          MenuPanel,
    pub levels:         &'a [LevelMeta],
    pub selected_level: usize,
    pub is_downloading: bool,
    pub time:           f32,
}

// ─── Wire-mesh particle ───────────────────────────────────────────────────────
struct Particle { x: f32, y: f32, vx: f32, vy: f32 }

// ─── Vertex batch ────────────────────────────────────────────────────────────
// Each vertex is (x, y, r, g, b, a) — 6 floats.
const FLOATS_PER_VERT: usize = 6;
const MAX_VERTS: usize = 65536;

struct Batch {
    tris:  Vec<f32>,   // TRIANGLES  — 3 verts each
    lines: Vec<f32>,   // LINES      — 2 verts each
}

impl Batch {
    fn new() -> Self { Self { tris: Vec::new(), lines: Vec::new() } }
    fn clear(&mut self) { self.tris.clear(); self.lines.clear(); }

    fn push_vert(buf: &mut Vec<f32>, x: f32, y: f32, c: [f32; 4]) {
        buf.extend_from_slice(&[x, y, c[0], c[1], c[2], c[3]]);
    }

    /// Axis-aligned filled quad.
    fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: [f32; 4]) {
        let (x1, y1, x2, y2) = (x, y, x + w, y + h);
        Self::push_vert(&mut self.tris, x1, y1, c);
        Self::push_vert(&mut self.tris, x2, y1, c);
        Self::push_vert(&mut self.tris, x2, y2, c);
        Self::push_vert(&mut self.tris, x1, y1, c);
        Self::push_vert(&mut self.tris, x2, y2, c);
        Self::push_vert(&mut self.tris, x1, y2, c);
    }

    /// 1-pixel border.
    fn border_rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: [f32; 4]) {
        let (x2, y2) = (x + w, y + h);
        self.line(x, y, x2, y, c);
        self.line(x2, y, x2, y2, c);
        self.line(x2, y2, x, y2, c);
        self.line(x, y2, x, y, c);
    }

    fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, c: [f32; 4]) {
        Self::push_vert(&mut self.lines, x1, y1, c);
        Self::push_vert(&mut self.lines, x2, y2, c);
    }

    fn dot(&mut self, cx: f32, cy: f32, r: f32, c: [f32; 4]) {
        self.fill_rect(cx - r, cy - r, r * 2.0, r * 2.0, c);
    }
}

// ─── Bitmap font (5×7 line-segment glyphs) ───────────────────────────────────
// Each entry: list of (x1,y1, x2,y2) in a 4×6 cell, y grows downward.
fn char_segs(ch: char) -> &'static [(f32,f32,f32,f32)] {
    match ch.to_ascii_uppercase() {
        'A' => &[(0.,6.,2.,0.),(2.,0.,4.,6.),(1.,3.,3.,3.)],
        'B' => &[(0.,0.,0.,6.),(0.,0.,3.,0.),(3.,0.,4.,1.),(4.,1.,4.,3.),(0.,3.,3.,3.),(3.,3.,4.,4.),(4.,4.,4.,6.),(4.,6.,0.,6.)],
        'C' => &[(4.,0.,1.,0.),(1.,0.,0.,1.),(0.,1.,0.,5.),(0.,5.,1.,6.),(1.,6.,4.,6.)],
        'D' => &[(0.,0.,0.,6.),(0.,0.,2.,0.),(2.,0.,4.,2.),(4.,2.,4.,4.),(4.,4.,2.,6.),(2.,6.,0.,6.)],
        'E' => &[(0.,0.,0.,6.),(0.,0.,4.,0.),(0.,3.,3.,3.),(0.,6.,4.,6.)],
        'F' => &[(0.,0.,0.,6.),(0.,0.,4.,0.),(0.,3.,3.,3.)],
        'G' => &[(4.,0.,1.,0.),(1.,0.,0.,1.),(0.,1.,0.,5.),(0.,5.,1.,6.),(1.,6.,4.,6.),(4.,6.,4.,3.),(4.,3.,2.,3.)],
        'H' => &[(0.,0.,0.,6.),(4.,0.,4.,6.),(0.,3.,4.,3.)],
        'I' => &[(1.,0.,3.,0.),(2.,0.,2.,6.),(1.,6.,3.,6.)],
        'J' => &[(2.,0.,4.,0.),(3.,0.,3.,5.),(3.,5.,2.,6.),(2.,6.,0.,6.)],
        'K' => &[(0.,0.,0.,6.),(4.,0.,0.,3.),(0.,3.,4.,6.)],
        'L' => &[(0.,0.,0.,6.),(0.,6.,4.,6.)],
        'M' => &[(0.,6.,0.,0.),(0.,0.,2.,3.),(2.,3.,4.,0.),(4.,0.,4.,6.)],
        'N' => &[(0.,6.,0.,0.),(0.,0.,4.,6.),(4.,6.,4.,0.)],
        'O' => &[(1.,0.,3.,0.),(3.,0.,4.,1.),(4.,1.,4.,5.),(4.,5.,3.,6.),(3.,6.,1.,6.),(1.,6.,0.,5.),(0.,5.,0.,1.),(0.,1.,1.,0.)],
        'P' => &[(0.,0.,0.,6.),(0.,0.,3.,0.),(3.,0.,4.,1.),(4.,1.,4.,3.),(4.,3.,3.,3.),(3.,3.,0.,3.)],
        'R' => &[(0.,0.,0.,6.),(0.,0.,3.,0.),(3.,0.,4.,1.),(4.,1.,4.,3.),(4.,3.,3.,3.),(3.,3.,0.,3.),(0.,3.,4.,6.)],
        'S' => &[(4.,0.,1.,0.),(1.,0.,0.,1.),(0.,1.,0.,3.),(0.,3.,4.,3.),(4.,3.,4.,5.),(4.,5.,3.,6.),(3.,6.,0.,6.)],
        'T' => &[(0.,0.,4.,0.),(2.,0.,2.,6.)],
        'U' => &[(0.,0.,0.,5.),(0.,5.,1.,6.),(1.,6.,3.,6.),(3.,6.,4.,5.),(4.,5.,4.,0.)],
        'V' => &[(0.,0.,2.,6.),(2.,6.,4.,0.)],
        'W' => &[(0.,0.,1.,6.),(1.,6.,2.,3.),(2.,3.,3.,6.),(3.,6.,4.,0.)],
        'X' => &[(0.,0.,4.,6.),(4.,0.,0.,6.)],
        'Y' => &[(0.,0.,2.,3.),(4.,0.,2.,3.),(2.,3.,2.,6.)],
        'Z' => &[(0.,0.,4.,0.),(4.,0.,0.,6.),(0.,6.,4.,6.)],
        '0' => &[(1.,0.,3.,0.),(3.,0.,4.,1.),(4.,1.,4.,5.),(4.,5.,3.,6.),(3.,6.,1.,6.),(1.,6.,0.,5.),(0.,5.,0.,1.),(0.,1.,1.,0.)],
        '1' => &[(1.,1.,2.,0.),(2.,0.,2.,6.)],
        '2' => &[(0.,1.,1.,0.),(1.,0.,3.,0.),(3.,0.,4.,2.),(4.,2.,0.,6.),(0.,6.,4.,6.)],
        '3' => &[(0.,0.,4.,0.),(4.,0.,4.,6.),(4.,3.,1.,3.),(4.,6.,0.,6.)],
        '4' => &[(0.,0.,0.,3.),(0.,3.,4.,3.),(3.,0.,3.,6.)],
        '5' => &[(4.,0.,0.,0.),(0.,0.,0.,3.),(0.,3.,3.,3.),(3.,3.,4.,4.),(4.,4.,4.,6.),(4.,6.,0.,6.)],
        '6' => &[(4.,0.,1.,0.),(1.,0.,0.,1.),(0.,1.,0.,6.),(0.,6.,4.,6.),(4.,6.,4.,3.),(4.,3.,0.,3.)],
        '7' => &[(0.,0.,4.,0.),(4.,0.,2.,6.)],
        '8' => &[(1.,0.,3.,0.),(0.,1.,0.,3.),(0.,1.,1.,0.),(3.,0.,4.,1.),(4.,1.,4.,3.),(0.,3.,4.,3.),(0.,3.,0.,5.),(0.,5.,1.,6.),(1.,6.,3.,6.),(3.,6.,4.,5.),(4.,5.,4.,3.)],
        '9' => &[(0.,6.,3.,6.),(3.,6.,4.,5.),(4.,5.,4.,0.),(4.,0.,0.,0.),(0.,0.,0.,3.),(0.,3.,4.,3.)],
        '.' => &[(2.,5.,2.,6.)],
        ':' => &[(2.,1.,2.,2.),(2.,4.,2.,5.)],
        '-' => &[(1.,3.,3.,3.)],
        '/' => &[(0.,6.,4.,0.)],
        '[' => &[(3.,0.,1.,0.),(1.,0.,1.,6.),(1.,6.,3.,6.)],
        ']' => &[(1.,0.,3.,0.),(3.,0.,3.,6.),(3.,6.,1.,6.)],
        _   => &[],
    }
}

fn text_width(s: &str, scale: f32) -> f32 {
    s.chars().map(|c| if c == ' ' { 3.0 } else { 5.0 }).sum::<f32>() * scale
}

// ─── Renderer ────────────────────────────────────────────────────────────────
pub struct MenuRenderer {
    shader_id: u32,
    loc_res:   i32,
    // Single persistent VAO + streaming VBO (raw IDs, not wrapped)
    vao_id:    u32,
    vbo_id:    u32,
    // Batched geometry built each frame
    batch:     Batch,
    // Particles for wire-mesh background
    particles: Vec<Particle>,
    time:      f32,
}

impl MenuRenderer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // ── Shader ────────────────────────────────────────────────────────
        // Vertex: pos in pixels → NDC, colour pass-through
        let vert_src = r#"#version 300 es
precision mediump float;
layout(location=0) in vec2 aPos;
layout(location=1) in vec4 aCol;
uniform vec2 uRes;
out vec4 vCol;
void main() {
    vec2 ndc = (aPos / uRes) * 2.0 - 1.0;
    ndc.y = -ndc.y;
    gl_Position = vec4(ndc, 0.0, 1.0);
    vCol = aCol;
}
"#;
        let frag_src = r#"#version 300 es
precision mediump float;
in vec4 vCol;
out vec4 fragColor;
void main() { fragColor = vCol; }
"#;
        let vert = Shader::vertex(vert_src).map_err(|e| e)?;
        let frag = Shader::fragment(frag_src).map_err(|e| e)?;
        let shader_id = unsafe {
            let id = gl::CreateProgram();
            vert.attach(id); frag.attach(id);
            gl::LinkProgram(id);
            let mut ok = 0i32;
            gl::GetProgramiv(id, gl::LINK_STATUS, &mut ok);
            if ok != gl::TRUE as i32 {
                return Err("Menu shader link failed".into());
            }
            vert.detach(id); vert.delete();
            frag.detach(id); frag.delete();
            id
        };
        let loc_res = {
            let c = std::ffi::CString::new("uRes").unwrap();
            unsafe { gl::GetUniformLocation(shader_id, c.as_ptr()) }
        };

        // ── Persistent VAO + VBO ──────────────────────────────────────────
        let (vao_id, vbo_id) = unsafe {
            let mut vao_id = 0u32;
            let mut vbo_id = 0u32;
            gl::GenVertexArrays(1, &mut vao_id);
            gl::GenBuffers(1, &mut vbo_id);
            gl::BindVertexArray(vao_id);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo_id);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (MAX_VERTS * FLOATS_PER_VERT * std::mem::size_of::<f32>()) as isize,
                std::ptr::null(),
                gl::DYNAMIC_DRAW,
            );
            let stride = (FLOATS_PER_VERT * std::mem::size_of::<f32>()) as i32;
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, stride, std::ptr::null());
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribPointer(1, 4, gl::FLOAT, gl::FALSE, stride,
                (2 * std::mem::size_of::<f32>()) as *const _);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
            (vao_id, vbo_id)
        };

        Ok(Self {
            shader_id, loc_res,
            vao_id, vbo_id,
            batch: Batch::new(),
            particles: Vec::new(),
            time: 0.0,
        })
    }

    // ── Public update ─────────────────────────────────────────────────────
    pub fn update(&mut self, hint_time: f32, w: f32, h: f32) {
        self.time = hint_time;
        let target = ((w * h) / 14_000.0) as usize;
        if self.particles.len() != target {
            use rand::Rng;
            let mut rng = rand::rng();
            self.particles = (0..target).map(|_| Particle {
                x:  rng.random_range(0.0..w),
                y:  rng.random_range(0.0..h),
                vx: rng.random_range(-0.18_f32..0.18),
                vy: rng.random_range(-0.18_f32..0.18),
            }).collect();
        }
        for p in &mut self.particles {
            p.x += p.vx; p.y += p.vy;
            if p.x < 0.0 || p.x > w { p.vx = -p.vx; p.x = p.x.clamp(0.0, w); }
            if p.y < 0.0 || p.y > h { p.vy = -p.vy; p.y = p.y.clamp(0.0, h); }
        }
    }

    // ── Flush the batch to GPU and draw ───────────────────────────────────
    fn flush_tris(&mut self) {
        if self.batch.tris.is_empty() { return; }
        let count = (self.batch.tris.len() / FLOATS_PER_VERT) as i32;
        unsafe {
            gl::BufferSubData(
                gl::ARRAY_BUFFER, 0,
                (self.batch.tris.len() * std::mem::size_of::<f32>()) as isize,
                self.batch.tris.as_ptr() as *const _,
            );
            gl::DrawArrays(gl::TRIANGLES, 0, count);
        }
        self.batch.tris.clear();
    }

    fn flush_lines(&mut self) {
        if self.batch.lines.is_empty() { return; }
        let count = (self.batch.lines.len() / FLOATS_PER_VERT) as i32;
        unsafe {
            gl::BufferSubData(
                gl::ARRAY_BUFFER, 0,
                (self.batch.lines.len() * std::mem::size_of::<f32>()) as isize,
                self.batch.lines.as_ptr() as *const _,
            );
            gl::DrawArrays(gl::LINES, 0, count);
        }
        self.batch.lines.clear();
    }

    // ── Drawing helpers (build into batch) ────────────────────────────────
    fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: [f32; 4]) {
        self.batch.fill_rect(x, y, w, h, c);
    }
    fn border_rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: [f32; 4]) {
        self.batch.border_rect(x, y, w, h, c);
    }
    fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, c: [f32; 4]) {
        self.batch.line(x1, y1, x2, y2, c);
    }
    fn dot(&mut self, cx: f32, cy: f32, r: f32, c: [f32; 4]) {
        self.batch.fill_rect(cx - r, cy - r, r * 2.0, r * 2.0, c);
    }

    fn text(&mut self, s: &str, x: f32, y: f32, scale: f32, c: [f32; 4]) {
        let mut cx = x;
        for ch in s.chars() {
            if ch == ' ' { cx += scale * 3.0; continue; }
            for &(x1, y1, x2, y2) in char_segs(ch) {
                self.batch.line(
                    cx + x1 * scale, y + y1 * scale,
                    cx + x2 * scale, y + y2 * scale,
                    c,
                );
            }
            cx += scale * 5.0;
        }
    }

    // ── Wire mesh ─────────────────────────────────────────────────────────
    fn build_mesh(&mut self, w: f32, h: f32) {
        let max_d2 = (w.min(h) * 0.22).powi(2);
        let n      = self.particles.len();
        for i in 0..n {
            for j in (i + 1)..n {
                let dx = self.particles[i].x - self.particles[j].x;
                let dy = self.particles[i].y - self.particles[j].y;
                if dx*dx + dy*dy < max_d2 {
                    let col = match (i + j) % 5 {
                        0|1 => [0.75f32, 0.19, 0.29, 0.28],
                        2|3 => [0.19, 0.31, 0.75, 0.24],
                        _   => [0.12, 0.13, 0.21, 0.40],
                    };
                    self.batch.line(
                        self.particles[i].x, self.particles[i].y,
                        self.particles[j].x, self.particles[j].y,
                        col,
                    );
                }
            }
            // Node dot
            self.batch.fill_rect(
                self.particles[i].x - 1.5, self.particles[i].y - 1.5,
                3.0, 3.0, [0.78, 0.82, 0.94, 0.45],
            );
        }
    }

    // ── Main render entry ─────────────────────────────────────────────────
    pub fn render_menu(&mut self, data: &MenuFrameData, w: f32, h: f32) {
        if !data.active { return; }
        self.batch.clear();

        // Set up GL state
        unsafe {
            gl::Disable(gl::DEPTH_TEST);
            gl::Disable(gl::CULL_FACE);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::UseProgram(self.shader_id);
            if self.loc_res >= 0 { gl::Uniform2f(self.loc_res, w, h); }
            // Bind our VAO (captures VBO + attrib pointer state)
            gl::BindVertexArray(self.vao_id);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo_id);
        }

        // Full-screen backdrop (opaque enough to cover the 3D scene)
        self.fill_rect(0.0, 0.0, w, h, BG);
        self.flush_tris();

        // Wire mesh (lines)
        self.build_mesh(w, h);
        self.flush_lines();

        // ── Layout ────────────────────────────────────────────────────────
        let pad       = w * 0.025;
        let sidebar_w = w * 0.28;
        let content_x = sidebar_w + pad;
        let content_w = w - content_x - pad;

        // Sidebar divider
        self.line(sidebar_w, 0.0, sidebar_w, h, BORDER);

        // Logo
        let logo_s = h * 0.026;
        self.text("FORMO", pad, h * 0.07, logo_s, TEXT);
        self.text("SAIC",  pad, h * 0.07 + logo_s * 9.5, logo_s, TEXT);

        let sub_s = h * 0.012;
        self.text("PUZZLE GAME", pad, h * 0.07 + logo_s * 20.0, sub_s, TEXT_DIM);

        // Nav items
        let nav_items: &[(&str, &str, bool)] = &[
            ("L", "LEVELS", data.panel == MenuPanel::Levels),
            ("N", "ONLINE", data.panel == MenuPanel::Online),
            ("R", "RANDOM", false),
        ];
        let nav_y0   = h * 0.42;
        let nav_step = h * 0.075;
        let nav_s    = h * 0.013;

        for (i, &(key, label, active)) in nav_items.iter().enumerate() {
            let ny  = nav_y0 + i as f32 * nav_step;
            let col = if active { TEXT } else { TEXT_DIM };
            let acc = if active { ACCENT_R } else { BORDER };

            if active {
                self.fill_rect(0.0, ny - 2.0, 3.0, nav_step * 0.8, ACCENT_R);
                self.fill_rect(0.0, ny - 2.0, sidebar_w, nav_step * 0.8,
                    [0.75, 0.19, 0.29, 0.06]);
            }
            self.flush_tris();

            // Key badge
            self.border_rect(pad, ny, nav_s * 4.5, nav_s * 6.0, acc);
            self.text(key, pad + nav_s, ny + nav_s, nav_s, acc);
            self.text(label, pad + nav_s * 7.0, ny + nav_s, nav_s, col);
        }

        // Version
        self.text("V0.1.0", pad, h * 0.93, h * 0.010, TEXT_DIM);

        // Flush sidebar lines + borders
        self.flush_lines();

        // ── Content panel ─────────────────────────────────────────────────
        match data.panel {
            MenuPanel::Levels => self.draw_levels(data, content_x, content_w, h, pad),
            MenuPanel::Online => self.draw_online(data, content_x, content_w, h, pad),
        }

        // ── Controls bar ──────────────────────────────────────────────────
        let bar_y = h - h * 0.052;
        self.fill_rect(0.0, bar_y, w, h * 0.052, [0.039, 0.043, 0.055, 0.95]);
        self.flush_tris();
        self.line(0.0, bar_y, w, bar_y, BORDER);

        let cs = h * 0.010;
        let controls: &[(&str, &str)] = &[
            ("H","HINT"), ("L","LEVELS"), ("N","ONLINE"),
            ("R","RANDOM"), ("K","SOLVE"), ("ESC","MENU"),
        ];
        let mut cx = pad;
        let cy = bar_y + h * 0.017;
        for &(k, desc) in controls {
            self.border_rect(cx, cy - cs, cs * 4.5, cs * 5.5, BORDER);
            self.text(k, cx + cs, cy, cs, TEXT_DIM);
            cx += cs * 6.5;
            self.text(desc, cx, cy, cs, TEXT_DIM);
            cx += text_width(desc, cs) + cs * 4.0;
        }
        self.flush_lines();

        // Restore GL state
        unsafe {
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
            gl::UseProgram(0);
            gl::Enable(gl::DEPTH_TEST);
            gl::Enable(gl::CULL_FACE);
            gl::Disable(gl::BLEND);
        }
    }

    // ── Level panel ───────────────────────────────────────────────────────
    fn draw_levels(&mut self, data: &MenuFrameData, x: f32, w: f32, h: f32, pad: f32) {
        let title_s = h * 0.011;
        self.text("SAVED LEVELS", x, h * 0.06, title_s, TEXT_DIM);
        let fetch = "[ N  FETCH MORE ]";
        self.text(fetch, x + w - text_width(fetch, title_s) - pad, h * 0.06, title_s, ACCENT_B);
        self.flush_lines();

        if data.levels.is_empty() {
            let es = h * 0.014;
            let msg = "NO SAVED LEVELS";
            self.text(msg, x + (w - text_width(msg, es)) * 0.5, h * 0.44, es, TEXT_DIM);
            let sub = "PRESS N TO FETCH FROM POLY.PIZZA";
            let ss = h * 0.011;
            self.text(sub, x + (w - text_width(sub, ss)) * 0.5, h * 0.52, ss, TEXT_DIM);
            self.flush_lines();
            return;
        }

        let cols   = 3usize;
        let card_w = (w - pad * (cols as f32 + 1.0)) / cols as f32;
        let card_h = (h * 0.72 / ((data.levels.len() / cols + 1).max(1) as f32)).min(h * 0.18);
        let grid_y = h * 0.12;

        for (idx, level) in data.levels.iter().enumerate() {
            let col = idx % cols;
            let row = idx / cols;
            let cx  = x + pad + col as f32 * (card_w + pad);
            let cy  = grid_y + row as f32 * (card_h + pad);

            let is_sel  = idx == data.selected_level;
            let bg_col  = if is_sel { [0.08, 0.09, 0.14, 1.0] } else { SURFACE };
            let bdr_col = if is_sel { ACCENT_R } else { BORDER };

            self.fill_rect(cx, cy, card_w, card_h, bg_col);
            self.flush_tris();

            self.border_rect(cx, cy, card_w, card_h, bdr_col);

            // Solved indicator dot
            if level.best_time_secs.is_some() {
                self.dot(cx + card_w - 8.0, cy + 8.0, 3.0, SOLVED);
                self.flush_tris();
            }

            let ns = (card_h * 0.14).clamp(h * 0.010, h * 0.015);

            // Name
            let name = if level.name.len() > 11 {
                format!("{}..", &level.name[..11])
            } else { level.name.clone() };
            self.text(&name, cx + pad * 0.5, cy + pad * 0.4, ns, TEXT);

            // Author
            let author = if level.author.len() > 13 {
                format!("{}..", &level.author[..13])
            } else { level.author.clone() };
            self.text(&author, cx + pad * 0.5, cy + pad * 0.4 + ns * 8.5, ns * 0.75, TEXT_DIM);

            // Difficulty badge
            let dc = diff_color(level.difficulty);
            let dlabel = match level.difficulty {
                d if d < 0.25 => "EASY",
                d if d < 0.50 => "MED",
                d if d < 0.75 => "HARD",
                _             => "EXP",
            };
            let ds  = ns * 0.80;
            let d_y = cy + card_h - ds * 8.0 - pad * 0.3;
            self.border_rect(cx + pad * 0.5, d_y - ds,
                text_width(dlabel, ds) + ds * 2.0, ds * 6.5, dc);
            self.text(dlabel, cx + pad * 0.5 + ds, d_y, ds, dc);

            // Best time
            if let Some(t) = level.best_time_secs {
                let ts = format!("{:01}:{:04.1}", (t/60.0) as u32, t % 60.0);
                let tw = text_width(&ts, ds);
                self.text(&ts, cx + card_w - tw - pad * 0.5, d_y, ds, TEXT_DIM);
            }

            self.flush_lines();
        }
    }

    // ── Online panel ──────────────────────────────────────────────────────
    fn draw_online(&mut self, data: &MenuFrameData, x: f32, w: f32, h: f32, _pad: f32) {
        let cx = x + w * 0.5;
        let cy = h * 0.42;
        let t  = self.time;

        // Animated hexagon
        let r     = h * 0.065;
        let sides = 6usize;
        for i in 0..sides {
            let a1 = std::f32::consts::TAU * i as f32 / sides as f32 + t * 0.4;
            let a2 = std::f32::consts::TAU * (i + 1) as f32 / sides as f32 + t * 0.4;
            let p  = 1.0 + 0.07 * (t * 2.0 + i as f32).sin();
            self.line(cx + r*p*a1.cos(), cy + r*p*a1.sin(),
                      cx + r*p*a2.cos(), cy + r*p*a2.sin(), ACCENT_B);
        }
        self.flush_lines();

        let is  = h * 0.020;
        let iw  = text_width("N", is);
        self.text("N", cx - iw * 0.5, cy - is * 3.0, is, ACCENT_B);

        let ts = h * 0.016;
        let title = "POLY PIZZA ONLINE";
        self.text(title, cx - text_width(title, ts) * 0.5, cy + r + h * 0.04, ts, TEXT);

        let ds = h * 0.011;
        let lines_txt = [
            "DOWNLOAD A RANDOM LOW-POLY MODEL",
            "FROM POLY.PIZZA, SCRAMBLE IT,",
            "AND SAVE IT LOCALLY FOR REPLAY.",
        ];
        for (i, ln) in lines_txt.iter().enumerate() {
            let lw = text_width(ln, ds);
            self.text(ln, cx - lw * 0.5, cy + r + h * 0.11 + i as f32 * ds * 8.5, ds, TEXT_DIM);
        }

        let btn_y = cy + r + h * 0.30;
        if data.is_downloading {
            // Spinner dots
            for i in 0..8u32 {
                let a   = std::f32::consts::TAU * i as f32 / 8.0 + t * 3.0;
                let sr  = h * 0.032;
                let dr  = h * 0.007;
                let alp = 0.2 + 0.8 * ((i as f32 / 8.0 + t) % 1.0);
                self.dot(cx + sr * a.cos(), btn_y + sr * a.sin(), dr,
                    [ACCENT_B[0], ACCENT_B[1], ACCENT_B[2], alp]);
            }
            self.flush_tris();
            let ft = "FETCHING...";
            self.text(ft, cx - text_width(ft, ds)*0.5, btn_y + h*0.06, ds, TEXT_DIM);
            self.flush_lines();
        } else {
            let btn = "[ PRESS N TO FETCH ]";
            let bw  = text_width(btn, ds * 1.1) + ds * 4.0;
            self.fill_rect(cx - bw * 0.5, btn_y - ds * 1.5, bw, ds * 8.0, ACCENT_B);
            self.flush_tris();
            self.text(btn, cx - text_width(btn, ds*1.1)*0.5, btn_y + ds, ds * 1.1, TEXT);
            self.flush_lines();
        }
    }
}

impl IRenderer for MenuRenderer {
    fn render(&mut self, _ctx: &SceneContext) {}
    fn any_processed(&self) -> bool { false }
    fn finish(&mut self) {}
}

impl Drop for MenuRenderer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.shader_id);
            gl::DeleteBuffers(1, &self.vbo_id);
            gl::DeleteVertexArrays(1, &self.vao_id);
        }
    }
}
