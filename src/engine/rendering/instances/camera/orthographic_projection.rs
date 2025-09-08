use crate::engine::rendering::abstracted::camera::camera_projection::CameraProjection;
use cgmath::{ortho, Matrix4};

pub struct OrthographicProjection {
    pub matrix: Matrix4<f32>,
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub near: f32,
    pub far: f32,
}

impl OrthographicProjection {
    pub fn new(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        let matrix = ortho(left, right, bottom, top, near, far);
        Self {
            matrix,
            left,
            right,
            bottom,
            top,
            near,
            far,
        }
    }
}

impl CameraProjection for OrthographicProjection {
    fn get_projection_matrix(
        &mut self,
        _resolution: (u32, u32),
        _fov: f32,
        _near: f32,
        _far: f32,
    ) -> Matrix4<f32> {
        // Always use the free function `ortho`
        self.matrix = ortho(
            self.left,
            self.right,
            self.bottom,
            self.top,
            self.near,
            self.far,
        );
        self.matrix
    }
}
