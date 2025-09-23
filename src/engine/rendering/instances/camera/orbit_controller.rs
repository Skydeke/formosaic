use crate::engine::{
    architecture::scene::node::transform::Transform,
    rendering::instances::camera::{camera::Camera, camera_controller::CameraController},
};
use cgmath::{InnerSpace, Quaternion, Rad, Rotation, Rotation3, Vector3};

pub struct OrbitController {
    pub target: Vector3<f32>,
    pub distance: f32,
    pub sensitivity: f32,

    dragging: bool,
    last_x: f32,
    last_y: f32,

    // store mouse deltas until `control()` is called
    delta_x: f32,
    delta_y: f32,
}

impl OrbitController {
    pub fn new(target: Vector3<f32>, distance: f32) -> Self {
        Self {
            target,
            distance,
            sensitivity: 1.5,
            dragging: false,
            last_x: 0.0,
            last_y: 0.0,
            delta_x: 0.0,
            delta_y: 0.0,
        }
    }

    pub fn handle_event(&mut self, event: &crate::input::Event, width: f32, height: f32) {
        match event {
            crate::input::Event::MouseDown { x, y, .. }
            | crate::input::Event::TouchDown { x, y, .. } => {
                self.dragging = true;
                self.last_x = *x;
                self.last_y = *y;
            }

            crate::input::Event::MouseUp { .. } | crate::input::Event::TouchUp { .. } => {
                self.dragging = false;
            }

            crate::input::Event::MouseMove { x, y, .. }
            | crate::input::Event::TouchMove { x, y, .. } => {
                if self.dragging {
                    let dx = (*x - self.last_x) / width;
                    let dy = (*y - self.last_y) / height;

                    self.last_x = *x;
                    self.last_y = *y;

                    // accumulate deltas until next control()
                    self.delta_x += dx;
                    self.delta_y += dy;
                }
            }

            _ => {}
        }
    }

    fn apply_rotation(&mut self, transform: &mut Transform) {
        // Current offset from target
        let mut offset = transform.position - self.target;
        if offset.magnitude2() < 1e-6 {
            offset = Vector3::new(0.0, 0.0, self.distance);
        }

        // Rotation quaternions
        let rot_pitch =
            Quaternion::from_angle_x(Rad(-self.delta_y * self.sensitivity * std::f32::consts::PI));

        let rot_yaw =
            Quaternion::from_angle_y(Rad(-self.delta_x * self.sensitivity * std::f32::consts::PI));

        // Apply both rotations
        let rotation = rot_yaw * rot_pitch;
        offset = rotation.rotate_vector(offset);

        // Keep distance consistent
        offset = offset.normalize() * self.distance;

        // Update camera transform
        transform.position = self.target + offset;
        transform.look_at(self.target, Vector3::unit_y());

        // reset deltas
        self.delta_x = 0.0;
        self.delta_y = 0.0;
    }
}

impl CameraController for OrbitController {
    fn control(&mut self, camera: &mut Camera) {
        self.apply_rotation(&mut camera.transform);
    }

    fn handle_event(&mut self, event: &crate::input::Event, width: f32, height: f32) {
        self.handle_event(event, width, height);
    }
}

