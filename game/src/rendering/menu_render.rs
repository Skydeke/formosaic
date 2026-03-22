//! Menu background renderer — OpenGL ES 3.0 compatible.
//!
//! Draws ONLY the decorative background: a dark full-screen backdrop and an
//! animated wire-mesh particle system.  All interactive UI (buttons, text,
//! level cards) is handled by imgui on top.
//!

use formosaic_engine::{
    architecture::scene::scene_context::SceneContext,
    rendering::abstracted::{irenderer::{IRenderer, RenderPass}, processable::NoopProcessable},
    opengl::{
        constants::{data_type::DataType, vbo_target::VboTarget, vbo_usage::VboUsage},
        objects::{attribute::Attribute, vao::Vao, vbo::Vbo},
        shaders::{uniform::{UniformAdapter, UniformVec2}, RenderState, ShaderProgram},
    },
};
use cgmath::Vector2;
use std::{cell::RefCell, rc::Rc};

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
struct FrameState { res: Vector2<f32> }

pub struct MenuRenderer {
    shader:    ShaderProgram<NoopProcessable>,
    frame:     Rc<RefCell<FrameState>>,
    vao:       Vao,
    vbo:       Vbo,
    batch:     Batch,
    particles: Vec<Particle>,
}

const DEFAULT_VERT: &str = include_str!("../../assets/shaders/menu.vert.glsl");
const DEFAULT_FRAG: &str = include_str!("../../assets/shaders/menu.frag.glsl");

impl MenuRenderer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_shaders(DEFAULT_VERT, DEFAULT_FRAG)
    }

    pub fn with_shaders(vert_src: &str, frag_src: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let frame = Rc::new(RefCell::new(FrameState { res: Vector2::new(1.0, 1.0) }));
        let mut shader = ShaderProgram::<NoopProcessable>::from_sources(vert_src, frag_src)?;
        {
            let f = Rc::clone(&frame);
            shader.add_per_render_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform: UniformVec2::new("uRes"),
                extractor: Box::new(move |_: &RenderState<NoopProcessable>| f.borrow().res),
            })));
        }

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
            shader, frame,
            vao, vbo,
            batch: Batch::new(),
            particles: Vec::new(),
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
        self.frame.borrow_mut().res = Vector2::new(w, h);
        unsafe {
            gl::Disable(gl::DEPTH_TEST);
            gl::Disable(gl::CULL_FACE);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }
        self.shader.bind();
        let state = RenderState::new_screenspace(self);
        self.shader.update_per_render_uniforms(&state);
        self.vao.bind();
        self.vbo.bind();
        self.batch.clear();
    }

    fn end_draw(&mut self) {
        self.vbo.unbind();
        self.vao.unbind();
        self.shader.unbind();
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::Enable(gl::CULL_FACE);
            gl::Disable(gl::BLEND);
        }
    }

    // ── Public: menu background ───────────────────────────────────────────
    /// Draw the full-screen backdrop and animated wire-mesh.
    /// `bg` is the scene clear colour (from `SceneContext::lights.clear_color`)
    /// so the menu backdrop exactly matches the in-game background.
    pub fn render_background(&mut self, w: f32, h: f32, bg: [f32; 3]) {
        self.begin_draw(w, h);

        // Solid backdrop — clears whatever the lighting pass left on screen.
        let bg4 = [bg[0], bg[1], bg[2], 1.0];
        self.batch.fill_rect(0.0, 0.0, w, h, bg4);
        self.flush_tris();

        // Animated wire mesh on top.
        self.build_mesh(w, h);
        self.flush_lines();
        self.flush_tris(); // node dots

        self.end_draw();
    }
}

impl IRenderer for MenuRenderer {
    fn pass(&self) -> RenderPass { RenderPass::Late }

    fn render(&mut self, ctx: &SceneContext) {
        // Draw animated backdrop only when in menu.
        // Touch buttons are drawn by imgui (build_ui in formosaic.rs).
        if ctx.show_menu {
            let res = ctx.get_camera().borrow().resolution;
            let w   = res.x as f32;
            let h   = res.y as f32;
            self.update(ctx.delta_time, w, h);
            self.render_background(w, h, ctx.lights.clear_color);
        }
    }

    fn any_processed(&self) -> bool { true }
    fn finish(&mut self) {}
}
