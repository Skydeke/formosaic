use crate::opengl::constants::render_mode::RenderMode;

pub trait Renderable {
    fn bind(&self);
    fn render(&self, render_mode: RenderMode);
}
