use crate::engine::architecture::scene::scene_context::SceneContext;
use crate::engine::rendering::pipeline::FrameData;

/// Which pass a renderer participates in.
#[derive(PartialEq, Eq)]
pub enum RenderPass {
    /// Runs inside the deferred geometry FBO (entity renderers).
    Geometry,
    /// Runs after the deferred lighting blit, before overlays.
    /// Draws directly to the default framebuffer.
    /// Use for full-screen backgrounds that must sit under all overlays.
    Late,
    /// Runs after Late, to the default framebuffer (outline, hint, imgui).
    Overlay,
}

pub trait IRenderer {
    /// Which pass this renderer belongs to. Defaults to `Geometry`.
    fn pass(&self) -> RenderPass { RenderPass::Geometry }

    /// Called once per frame with all per-frame data before any `render` calls.
    /// Renderers that don't need it can leave this as the default no-op.
    fn prepare(&mut self, _data: &FrameData) {}

    fn render(&mut self, context: &SceneContext);
    fn any_processed(&self) -> bool;
    fn finish(&mut self);
}
