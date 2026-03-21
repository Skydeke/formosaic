//! Menu background renderer — OpenGL ES 3.0 compatible.
//!
//! Draws ONLY the decorative background: a dark full-screen backdrop and an
//! animated wire-mesh particle system.  All interactive UI (buttons, text,
//! level cards) is handled by imgui on top.
//!
//! Also provides the in-game touch button overlay for Android via
//! `render_touch_buttons` / `hit_test_touch_buttons`.

use crate::engine::architecture::scene::scene_context::SceneContext;
use crate::engine::rendering::abstracted::irenderer::{IRenderer, RenderPass};
use crate::engine::rendering::pipeline::FrameData;
use crate::opengl::{
    constants::{data_type::DataType, vbo_target::VboTarget, vbo_usage::VboUsage},
    objects::{attribute::Attribute, vao::Vao, vbo::Vbo},
    shaders::SimpleProgram,
};

// ─── Colour palette ───────────────────────────────────────────────────────────
const BG:     [f32; 4] = [0.039, 0.043, 0.055, 0.97];
const BORDER: [f32; 4] = [0.118, 0.133, 0.208, 1.0];

// ─── Wire-mesh particle ───────────────────────────────────────────────────────
struct Particle { x: f32, y: f32, vx: f32, vy: f32 }

// ─── Vertex batch ─────────────────────────────────────────────────────────────
// Each vertex: (x, y, r, g, b, a) — 6 floats.
const FLOATS_PER_VERT: usize = 6;
const MAX_VERTS: usize = 65536;

struct Batch {
    tris:  Vec<f32>,
    lines: Vec<f32>,
}

impl Batch {
    fn new() -> Self { Self { tris: Vec::new(), lines: Vec::new() } }
    fn clear(&mut self) { self.tris.clear(); self.lines.clear(); }

    fn push_vert(buf: &mut Vec<f32>, x: f32, y: f32, c: [f32; 4]) {
        buf.extend_from_slice(&[x, y, c[0], c[1], c[2], c[3]]);
    }

    fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: [f32; 4]) {
        let (x2, y2) = (x + w, y + h);
        Self::push_vert(&mut self.tris, x,  y,  c);
        Self::push_vert(&mut self.tris, x2, y,  c);
        Self::push_vert(&mut self.tris, x2, y2, c);
        Self::push_vert(&mut self.tris, x,  y,  c);
        Self::push_vert(&mut self.tris, x2, y2, c);
        Self::push_vert(&mut self.tris, x,  y2, c);
    }

    fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, c: [f32; 4]) {
        Self::push_vert(&mut self.lines, x1, y1, c);
        Self::push_vert(&mut self.lines, x2, y2, c);
    }
}

// ─── Renderer ─────────────────────────────────────────────────────────────────
pub struct MenuRenderer {
    program:    SimpleProgram,
    loc_res:    i32,
    vao:        Vao,
    vbo:        Vbo,
    batch:      Batch,
    particles:  Vec<Particle>,
    // Frame state set via IRenderer::prepare each frame.
    show_menu:  bool,
    is_touch:   bool,
    viewport_w: f32,
    viewport_h: f32,
    delta_time: f32,
}

impl MenuRenderer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let vert_src = include_str!("../../../../assets/shaders/menu.vert.glsl");
        let frag_src = include_str!("../../../../assets/shaders/menu.frag.glsl");
        let program  = SimpleProgram::from_sources(vert_src, frag_src)
            .map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
        let loc_res  = program.uniform_location("uRes");

        let stride = (FLOATS_PER_VERT * std::mem::size_of::<f32>()) as i32;
        let mut vbo = Vbo::create(VboTarget::ArrayBuffer, VboUsage::DynamicDraw);
        let vao = Vao::create(); // creates and auto-binds

        vbo.bind();
        vbo.allocate_float(MAX_VERTS * FLOATS_PER_VERT);

        let a_pos = Attribute::of(0, 2, DataType::Float, false);
        let a_col = Attribute::of(1, 4, DataType::Float, false);
        a_pos.link(stride, 0);
        a_col.link(stride, (2 * std::mem::size_of::<f32>()) as i32);
        Attribute::enable_index(0);
        Attribute::enable_index(1);

        vao.unbind();
        vbo.unbind();

        Ok(Self {
            program, loc_res,
            vao, vbo,
            batch: Batch::new(),
            particles: Vec::new(),
            show_menu:  false,
            is_touch:   false,
            viewport_w: 1.0,
            viewport_h: 1.0,
            delta_time: 0.0,
        })
    }

    // ── Particle simulation tick ──────────────────────────────────────────
    /// `dt` is the frame delta in seconds.  Velocities are in pixels/second
    /// so motion is identical regardless of frame rate.
    pub fn update(&mut self, dt: f32, w: f32, h: f32) {
        // Target ~1 particle per 14 000 px² of screen area.
        let target = ((w * h) / 14_000.0) as usize;
        if self.particles.len() != target {
            use rand::Rng;
            let mut rng = rand::rng();
            // Velocity in pixels/second — looks good at ~30–40 px/s.
            self.particles = (0..target).map(|_| Particle {
                x:  rng.random_range(0.0..w),
                y:  rng.random_range(0.0..h),
                vx: rng.random_range(-40.0_f32..40.0),
                vy: rng.random_range(-40.0_f32..40.0),
            }).collect();
        }
        for p in &mut self.particles {
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            if p.x < 0.0 || p.x > w { p.vx = -p.vx; p.x = p.x.clamp(0.0, w); }
            if p.y < 0.0 || p.y > h { p.vy = -p.vy; p.y = p.y.clamp(0.0, h); }
        }
    }

    // ── Flush helpers ─────────────────────────────────────────────────────
    fn flush_tris(&mut self) {
        if self.batch.tris.is_empty() { return; }
        let count = (self.batch.tris.len() / FLOATS_PER_VERT) as i32;
        self.vbo.store_float(0, &self.batch.tris);
        unsafe { gl::DrawArrays(gl::TRIANGLES, 0, count); }
        self.batch.tris.clear();
    }

    fn flush_lines(&mut self) {
        if self.batch.lines.is_empty() { return; }
        let count = (self.batch.lines.len() / FLOATS_PER_VERT) as i32;
        self.vbo.store_float(0, &self.batch.lines);
        unsafe { gl::DrawArrays(gl::LINES, 0, count); }
        self.batch.lines.clear();
    }

    // ── Wire mesh ─────────────────────────────────────────────────────────
    fn build_mesh(&mut self, w: f32, h: f32) {
        let max_d2 = (w.min(h) * 0.22).powi(2);
        let n = self.particles.len();
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

    // ── GL setup / teardown helpers ───────────────────────────────────────
    fn begin_draw(&mut self, w: f32, h: f32) {
        unsafe {
            gl::Disable(gl::DEPTH_TEST);
            gl::Disable(gl::CULL_FACE);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }
        self.program.bind();
        if self.loc_res >= 0 {
            unsafe { gl::Uniform2f(self.loc_res, w, h); }
        }
        self.vao.bind();
        self.vbo.bind();
        self.batch.clear();
    }

    fn end_draw(&mut self) {
        self.vbo.unbind();
        self.vao.unbind();
        self.program.unbind();
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::Enable(gl::CULL_FACE);
            gl::Disable(gl::BLEND);
        }
    }

    // ── Public: menu background ───────────────────────────────────────────
    /// Draw the full-screen backdrop and animated wire-mesh.
    /// Call `update()` each frame before this to tick the particle simulation.
    /// imgui draws all interactive UI on top of this.
    pub fn render_background(&mut self, w: f32, h: f32) {
        self.begin_draw(w, h);

        // Dark backdrop
        self.batch.fill_rect(0.0, 0.0, w, h, BG);
        self.flush_tris();

        // Animated wire mesh
        self.build_mesh(w, h);
        self.flush_lines();

        self.end_draw();
    }

    // ── Public: Android touch buttons ────────────────────────────────────
    /// Draw ESC/HINT/AGAIN tap targets during gameplay (Android only).
    pub fn render_touch_buttons(&mut self, w: f32, h: f32) {
        let btn_h  = (h * 0.10).max(44.0);
        let btn_w  = (w * 0.14).max(80.0);
        let margin = w * 0.012;
        let y_pos  = h - btn_h - margin;

        self.begin_draw(w, h);

        let buttons: &[(&str, &str)] = &[
            ("ESC", "MENU"),
            ("H",   "HINT"),
            ("L",   "AGAIN"),
        ];

        for (i, &(_key, _label)) in buttons.iter().enumerate() {
            let bx = margin + i as f32 * (btn_w + margin);
            // Background
            self.batch.fill_rect(bx, y_pos, btn_w, btn_h,
                [0.039, 0.043, 0.055, 0.75]);
            self.flush_tris();
            // Border
            let (x2, y2) = (bx + btn_w, y_pos + btn_h);
            self.batch.line(bx, y_pos, x2, y_pos, BORDER);
            self.batch.line(x2, y_pos, x2, y2,    BORDER);
            self.batch.line(x2, y2,    bx, y2,    BORDER);
            self.batch.line(bx, y2,    bx, y_pos, BORDER);
            self.flush_lines();
        }

        self.end_draw();
    }

    /// Returns which key was hit by a touch at (tx, ty), if any.
    /// Returns `Some("Escape" | "h" | "l")` matching the EngineKey names.
    pub fn hit_test_touch_buttons(tx: f32, ty: f32, w: f32, h: f32) -> Option<&'static str> {
        let btn_h  = (h * 0.10).max(44.0);
        let btn_w  = (w * 0.14).max(80.0);
        let margin = w * 0.012;
        let y_pos  = h - btn_h - margin;
        let keys   = ["Escape", "h", "l"];
        for (i, key) in keys.iter().enumerate() {
            let bx = margin + i as f32 * (btn_w + margin);
            if tx >= bx && tx <= bx + btn_w && ty >= y_pos && ty <= y_pos + btn_h {
                return Some(key);
            }
        }
        None
    }
}

impl IRenderer for MenuRenderer {
    fn pass(&self) -> RenderPass { RenderPass::Late }

    fn prepare(&mut self, data: &FrameData) {
        self.show_menu  = data.show_menu;
        self.is_touch   = data.is_touch;
        self.viewport_w = data.viewport_w;
        self.viewport_h = data.viewport_h;
        self.delta_time = data.delta_time;
    }

    fn render(&mut self, _ctx: &SceneContext) {
        let (w, h) = (self.viewport_w, self.viewport_h);
        if self.show_menu {
            self.update(self.delta_time, w, h);
            self.render_background(w, h);
        } else if self.is_touch {
            self.render_touch_buttons(w, h);
        }
    }

    fn any_processed(&self) -> bool { self.show_menu || self.is_touch }
    fn finish(&mut self) {}
}
