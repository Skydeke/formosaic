//! Dear ImGui GL renderer.
//!
//! Owns `imgui::Context` and `WinitPlatform` so that `DrawData` lifetime is
//! entirely contained within a single `render()` call — no raw pointers.
//!
//! Frame flow each `RedrawRequested`:
//!   1. `GameEngine` calls `imgui_renderer.prepare(window, pw, ph, scale, sc)`
//!      which starts a new imgui frame and calls all visible `UiNode`s.
//!   2. `pipeline.draw()` → `imgui_pass()` → `IRenderer::render()` calls
//!      `imgui.render()` and immediately draws — `&DrawData` never escapes.

use std::{cell::RefCell, mem, rc::Rc, sync::Arc};

use cgmath::Matrix4;
use imgui::{DrawData, DrawIdx, DrawVert, TextureId};
use winit::window::Window;

use crate::architecture::scene::{node::ui_node::UiNode, scene_context::SceneContext};
use crate::architecture::scene::node::node::NodeBehavior;
use crate::opengl::{
    constants::{data_type::DataType, vbo_target::VboTarget, vbo_usage::VboUsage},
    fbos::simple_texture::SimpleTexture,
    objects::{attribute::Attribute, vao::Vao, vbo::Vbo},
    shaders::{
        uniform::{UniformAdapter, UniformInt, UniformMatrix4},
        RenderState, ShaderProgram,
    },
    textures::{
        parameters::{
            mag_filter_parameter::MagFilterParameter,
            min_filter_parameter::MinFilterParameter,
        },
        texture::Texture,
        texture_configs::TextureConfigs,
    },
};
use crate::rendering::abstracted::{
    irenderer::{IRenderer, RenderPass},
    processable::NoopProcessable,
};

// ─── Internal GL state ────────────────────────────────────────────────────────

struct FrameState { proj: Matrix4<f32> }

// ─── GL resource bundle (borrowable independently from imgui::Context) ────────

struct GlResources {
    shader:   ShaderProgram<NoopProcessable>,
    frame:    Rc<RefCell<FrameState>>,
    vao:      Vao,
    vbo:      Vbo,
    ebo:      Vbo,
    #[allow(dead_code)]
    font_tex: SimpleTexture,
}

impl IRenderer for GlResources {
    fn render(&mut self, _ctx: &SceneContext) {}
    fn any_processed(&self) -> bool { false }
    fn finish(&mut self) {}
}

impl GlResources {
    fn draw(&mut self, draw_data: &DrawData) {
        let fb_w = draw_data.display_size[0] * draw_data.framebuffer_scale[0];
        let fb_h = draw_data.display_size[1] * draw_data.framebuffer_scale[1];
        if fb_w <= 0.0 || fb_h <= 0.0 || draw_data.total_vtx_count == 0 { return; }

        let [dl, dt] = draw_data.display_pos;
        let [dr, db] = [dl + draw_data.display_size[0], dt + draw_data.display_size[1]];
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

        let co = draw_data.display_pos;
        let cs = draw_data.framebuffer_scale;

        for draw_list in draw_data.draw_lists() {
            self.vbo.store_raw(draw_list.vtx_buffer());
            self.ebo.store_raw(draw_list.idx_buffer());
            for cmd in draw_list.commands() {
                match cmd {
                    imgui::DrawCmd::Elements { count, cmd_params } => {
                        let cx = (cmd_params.clip_rect[0] - co[0]) * cs[0];
                        let cy = (cmd_params.clip_rect[1] - co[1]) * cs[1];
                        let cw = (cmd_params.clip_rect[2] - co[0]) * cs[0];
                        let ch = (cmd_params.clip_rect[3] - co[1]) * cs[1];
                        if cw <= cx || ch <= cy { continue; }
                        let idx_type = if mem::size_of::<DrawIdx>() == 2 {
                            gl::UNSIGNED_SHORT
                        } else { gl::UNSIGNED_INT };
                        unsafe {
                            gl::Scissor(cx as i32, (fb_h - ch) as i32,
                                        (cw - cx) as i32, (ch - cy) as i32);
                            gl::ActiveTexture(gl::TEXTURE0);
                            gl::BindTexture(gl::TEXTURE_2D, cmd_params.texture_id.id() as u32);
                            gl::DrawElements(gl::TRIANGLES, count as i32, idx_type,
                                (cmd_params.idx_offset * mem::size_of::<DrawIdx>()) as *const _);
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

// ─── ImguiGlRenderer ─────────────────────────────────────────────────────────

pub struct ImguiGlRenderer {
    gl:       GlResources,
    imgui:    imgui::Context,
    platform: imgui_winit_support::WinitPlatform,
}

const DEFAULT_VERT: &str = include_str!("../../../assets/shaders/imgui.vert.glsl");
const DEFAULT_FRAG: &str = include_str!("../../../assets/shaders/imgui.frag.glsl");

impl ImguiGlRenderer {
    pub fn new(
        mut imgui: imgui::Context,
        platform:  imgui_winit_support::WinitPlatform,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let frame = Rc::new(RefCell::new(FrameState { proj: Matrix4::from_scale(1.0) }));

        let mut shader = ShaderProgram::<NoopProcessable>::from_sources(DEFAULT_VERT, DEFAULT_FRAG)?;
        {
            let f = Rc::clone(&frame);
            shader.add_per_render_uniform(Rc::new(RefCell::new(UniformAdapter {
                uniform:   UniformMatrix4::new("uProjMtx"),
                extractor: Box::new(move |_: &RenderState<NoopProcessable>| f.borrow().proj),
            })));
        }
        shader.add_per_render_uniform(Rc::new(RefCell::new(UniformAdapter {
            uniform:   UniformInt::new("uTexture"),
            extractor: Box::new(|_: &RenderState<NoopProcessable>| 0i32),
        })));

        let stride = mem::size_of::<DrawVert>() as i32;
        let vao = Vao::create();
        let vbo = Vbo::create(VboTarget::ArrayBuffer,        VboUsage::StreamDraw);
        let ebo = Vbo::create(VboTarget::ElementArrayBuffer, VboUsage::StreamDraw);
        vbo.bind(); ebo.bind();
        Attribute::of(0, 2, DataType::Float, false).link(stride, 0);
        Attribute::of(1, 2, DataType::Float, false).link(stride, 8);
        Attribute::of(2, 4, DataType::UByte, true ).link(stride, 16);
        Attribute::enable_index(0);
        Attribute::enable_index(1);
        Attribute::enable_index(2);
        vao.unbind(); vbo.unbind();

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
        {
            let atlas = imgui.fonts();
            let tex   = atlas.build_rgba32_texture();
            font_tex.upload_rgba8(tex.width as i32, tex.height as i32, tex.data);
        }
        font_tex.unbind();
        imgui.fonts().tex_id = TextureId::new(font_tex.get_id() as usize);

        Ok(Self {
            gl: GlResources { shader, frame, vao, vbo, ebo, font_tex },
            imgui,
            platform,
        })
    }

    /// Run imgui new-frame, call all visible UiNodes, then prepare_render.
    /// Must be called every frame before `pipeline.draw()`.
    pub fn prepare(
        &mut self,
        window: &Arc<Window>,
        pw: f32, ph: f32, scale: f32,
        sc: &mut SceneContext,
    ) {
        let lw = pw / scale;
        let lh = ph / scale;
        self.imgui.io_mut().display_size              = [lw, lh];
        self.imgui.io_mut().display_framebuffer_scale = [scale, scale];
        self.platform.prepare_frame(self.imgui.io_mut(), window)
            .expect("imgui prepare_frame failed");

        let ui = self.imgui.frame();

        // Collect node handles first (drops scene borrow), then build each.
        let nodes: Vec<_> = sc.scene()
            .map(|sg| sg.collect_nodes_of_type::<UiNode>())
            .unwrap_or_default();
        for node_rc in nodes {
            let mut b = node_rc.borrow_mut();
            if !b.is_hidden() {
                if let Some(n) = b.as_any_mut().downcast_mut::<UiNode>() {
                    n.build(ui, lw, lh, sc);
                }
            }
        }

        self.platform.prepare_render(ui, window);
    }

    /// Forward a winit event into imgui.
    pub fn handle_event(&mut self, window: &Arc<Window>, event: &winit::event::Event<()>) {
        self.platform.handle_event(self.imgui.io_mut(), window, event);
    }

    pub fn wants_mouse(&self)    -> bool { self.imgui.io().want_capture_mouse }
    pub fn wants_keyboard(&self) -> bool { self.imgui.io().want_capture_keyboard }
}

impl IRenderer for ImguiGlRenderer {
    fn pass(&self) -> RenderPass { RenderPass::Overlay }

    /// Finalise the imgui frame (built in `prepare()`) and draw immediately.
    /// `&DrawData` is borrowed from `self.imgui` and passed to `self.gl.draw()`.
    /// These are separate fields so the borrow checker is satisfied.
    fn render(&mut self, _context: &SceneContext) {
        // self.imgui and self.gl are distinct fields — no aliasing.
        let draw_data = self.imgui.render();
        self.gl.draw(draw_data);
    }

    fn any_processed(&self) -> bool { true }
    fn finish(&mut self) {}
}
