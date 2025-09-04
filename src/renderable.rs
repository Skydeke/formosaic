use crate::opengl::constants::render_mode::RenderMode;

pub trait Renderable {
    fn bind(&mut self);
    fn render(&self, render_mode: RenderMode);
}
