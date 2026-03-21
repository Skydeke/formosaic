use crate::engine::architecture::models::model::Model;
use crate::engine::architecture::models::simple_model::SimpleModel;

pub trait Processable {
    fn process(&mut self);

    fn get_model(&self) -> &impl Model;
}

/// Zero-size marker that satisfies `Processable` for renderers that have no
/// scene entity (fullscreen-quad passes, etc.).  `get_model` must never be
/// called on it; use `RenderState::new_without_instance` so no instance is
/// bound to the render state.
pub struct NoopProcessable;

impl Processable for NoopProcessable {
    fn process(&mut self) {}

    #[allow(refining_impl_trait)]
    fn get_model(&self) -> &SimpleModel {
        panic!("NoopProcessable::get_model must never be called")
    }
}
