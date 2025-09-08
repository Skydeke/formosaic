use crate::engine::architecture::scene::node::transform::Transform;
use crate::engine::rendering::abstracted::camera::camera_projection::CameraProjection;
use crate::engine::rendering::instances::camera::camera_controller::CameraController;
use crate::engine::rendering::instances::camera::camera_controller::NoneController;
use crate::engine::rendering::instances::camera::perspective_projection::PerspectiveProjection;
use cgmath::Vector2;
use cgmath::{EuclideanSpace, Matrix4, Point3, SquareMatrix};

pub struct Camera {
    pub projection_view_matrix: Matrix4<f32>,
    pub projection_matrix: Matrix4<f32>,
    pub view_matrix: Matrix4<f32>,

    pub near_plane: f32,
    pub far_plane: f32,
    pub fov: f32,

    controller: Box<dyn CameraController>,
    projection: Box<dyn CameraProjection>,

    pub transform: Transform,
    pub resolution: Vector2<u32>,
}

impl Camera {
    pub fn new() -> Self {
        let mut camera = Self {
            projection_view_matrix: Matrix4::identity(),
            projection_matrix: Matrix4::identity(),
            view_matrix: Matrix4::identity(),
            fov: 75.0_f32.to_radians(),
            near_plane: 0.01,
            far_plane: 1000.0,
            controller: Box::new(NoneController::new()),
            projection: Box::new(PerspectiveProjection::new()),
            transform: Transform::new(),
            resolution: Vector2::new(0, 0),
        };
        camera.update_all_matrices();
        camera
    }

    pub fn update(&mut self) {
        // Split the controller update to avoid borrowing issues
        let mut temp_controller =
            std::mem::replace(&mut self.controller, Box::new(NoneController::new()));
        temp_controller.control(self);
        self.controller = temp_controller;

        self.update_all_matrices();
    }

    fn update_all_matrices(&mut self) {
        self.update_projection_matrix();
        self.update_view_matrix();
        self.update_projection_view_matrix();
    }

    pub fn set_controller(&mut self, controller: Option<Box<dyn CameraController>>) {
        self.controller = controller.unwrap_or_else(|| Box::new(NoneController::new()));
    }

    pub fn set_projection(&mut self, projection: Box<dyn CameraProjection>) {
        self.projection = projection;
        self.update_projection_matrix();
    }

    pub fn get_transform_mut(&mut self) -> &mut Transform {
        &mut self.transform
    }

    /* ===== MATRICES ===== */

    pub fn update_view_matrix(&mut self) {
        let forward = self.transform.forward(); // camera looks along -Z
        let up = self.transform.up(); // rotated up

        self.view_matrix = Matrix4::look_at_rh(
            Point3::from_vec(self.transform.position),
            Point3::from_vec(self.transform.position + forward),
            up,
        );
    }

    pub fn update_projection_matrix(&mut self) {
        self.projection_matrix = self.projection.get_projection_matrix(
            (self.resolution.x, self.resolution.y),
            self.get_fov(),
            self.get_near_plane(),
            self.get_far_plane(),
        );
    }

    pub fn set_resolution(&mut self, size: Vector2<u32>) {
        self.resolution = size;
    }

    pub fn update_projection_view_matrix(&mut self) {
        self.projection_view_matrix = self.projection_matrix * self.view_matrix;
    }

    pub fn get_view_matrix(&self) -> &Matrix4<f32> {
        &self.view_matrix
    }

    pub fn get_projection_matrix(&self) -> &Matrix4<f32> {
        &self.projection_matrix
    }

    pub fn get_projection_view_matrix(&self) -> &Matrix4<f32> {
        &self.projection_view_matrix
    }

    pub fn get_fov(&self) -> f32 {
        self.fov
    }

    pub fn get_near_plane(&self) -> f32 {
        self.near_plane
    }

    pub fn get_far_plane(&self) -> f32 {
        self.far_plane
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}
