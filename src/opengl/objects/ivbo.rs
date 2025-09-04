use crate::opengl::objects::vao::Vao;

/// Trait representing a Vertex Buffer Object (VBO)
pub trait IVbo {
    /// Returns the size of the VBO in "elements" or "bytes"
    fn get_size(&self) -> usize;

    /// Binds the VBO
    fn bind(&self);

    /// Binds the VBO to the given VAO
    fn bind_to_vao(&self, vao: &Vao);

    /// Unbinds the VBO
    fn unbind(&self);

    /// Deletes the VBO
    fn delete(&self);
}
