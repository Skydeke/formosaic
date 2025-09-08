use std::{cell::RefCell, rc::Rc};

use cgmath::Vector2;

use crate::engine::{
    architecture::scene::{entity::simple_entity::SimpleEntity, scene_context::SceneContext},
    rendering::{abstracted::irenderer::IRenderer, instances::entity_render::EntityRenderer},
};

pub struct Pipeline {
    renderers: Vec<Box<dyn IRenderer>>,
    context: Rc<RefCell<SceneContext>>,
}

impl Pipeline {
    pub fn new(context: Rc<RefCell<SceneContext>>) -> Self {
        let mut pipeline = Self {
            renderers: Vec::new(),
            context,
        };

        let simple_triangle_renderer: EntityRenderer<SimpleEntity> =
            EntityRenderer::new().expect("Cant create EntityRenderer.");
        pipeline.add_renderer(Box::new(simple_triangle_renderer));

        pipeline
    }

    pub fn add_renderer(&mut self, renderer: Box<dyn IRenderer>) {
        self.renderers.push(renderer);
    }

    pub fn draw(&mut self, width: u32, height: u32) {
        self.context
            .borrow_mut()
            .set_resolution(Vector2::new(width, height));
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT);
        }

        self.context.borrow_mut().update();

        for renderer in &mut self.renderers {
            renderer.render(&self.context.borrow());
        }

        self.finish();
    }

    fn finish(&mut self) {
        for renderer in &mut self.renderers {
            renderer.finish();
        }
    }
}
