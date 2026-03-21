use crate::architecture::scene::scene_context::SceneContext;

/// Which pass a renderer participates in.
#[derive(PartialEq, Eq)]
pub enum RenderPass {
    Geometry,
    Late,
    Overlay,
}

/// Generic renderer interface — knows nothing about game state.
///
/// Per-frame state is pushed to concrete renderers via their own typed setter
/// methods *before* `Pipeline::draw` is called.  There is no generic prepare
/// hook here; that would force the abstraction to depend on game-specific types.
pub trait IRenderer {
    fn pass(&self) -> RenderPass { RenderPass::Geometry }
    fn render(&mut self, context: &SceneContext);
    fn any_processed(&self) -> bool;
    fn finish(&mut self);

    /// Downcasting support — lets the Pipeline expose typed renderer accessors
    /// without storing separate handles.  Concrete renderers override this.
    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> { None }
}
