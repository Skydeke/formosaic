use crate::engine::architecture::models::model::Model;

pub trait Processable {
    fn process(&mut self);

    fn get_model(&self) -> &impl Model;
}
