use cgmath::Matrix4;

pub trait CameraProjection {
    fn get_projection_matrix(
        &mut self,
        resolution: (u32, u32),
        fov: f32,
        near: f32,
        far: f32,
    ) -> Matrix4<f32>;
}
