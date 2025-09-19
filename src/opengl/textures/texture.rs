pub trait Texture {
    /// Bind the texture to a specific texture unit
    fn bind_to_unit(&self, unit: u32);

    /// Bind the texture to the currently active unit
    fn bind(&self);

    /// Unbind the texture from the current active unit
    fn unbind(&self);

    /// Delete the texture and free its resources
    fn delete(&mut self);

    /// Get the underlying OpenGL texture ID
    fn get_id(&self) -> u32;
}
