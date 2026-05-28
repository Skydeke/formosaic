pub trait Processable {
    fn process(&mut self);
}

/// Zero-size marker that satisfies `Processable` for renderers that have no
/// scene entity (fullscreen-quad passes, etc.).  Use
/// `RenderState::new_without_instance` so no instance is bound.
pub struct NoopProcessable;

impl Processable for NoopProcessable {
    fn process(&mut self) {}
}
