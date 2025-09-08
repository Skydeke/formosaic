use crate::engine::architecture::scene::scene_context::SceneContext;

pub trait IRenderer {
    fn render(&mut self, context: &SceneContext);
    fn any_processed(&self) -> bool;
    fn finish(&mut self);
}
