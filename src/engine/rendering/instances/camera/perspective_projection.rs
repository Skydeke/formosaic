use cgmath::SquareMatrix;
use cgmath::{perspective, Deg, Matrix4};

use crate::engine::rendering::abstracted::camera::camera_projection::CameraProjection;

pub struct PerspectiveProjection {
    pub matrix: Matrix4<f32>,
}

impl PerspectiveProjection {
    pub fn new() -> Self {
        Self {
            matrix: Matrix4::identity(),
        }
    }
}

impl Default for PerspectiveProjection {
    fn default() -> Self {
        Self::new()
    }
}

impl CameraProjection for PerspectiveProjection {
    fn get_projection_matrix(
        &mut self,
        resolution: (u32, u32),
        fov: f32,
        near: f32,
        far: f32,
    ) -> Matrix4<f32> {
        let aspect_ratio = resolution.0 as f32 / resolution.1 as f32;
        self.matrix = perspective(Deg(fov.to_degrees()), aspect_ratio, near, far);
        self.matrix
    }
}
