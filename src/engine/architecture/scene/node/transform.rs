use cgmath::{Deg, InnerSpace, Matrix3, Matrix4, One, Quaternion, Rotation3, Vector3};
use std::cell::RefCell;
use std::rc::Weak;

use crate::engine::architecture::scene::node::node::NodeBehavior;

#[derive(Clone, Debug)]
pub struct Transform {
    pub position: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub scale: Vector3<f32>,

    // Weak reference to parent node
    pub parent: Option<Weak<RefCell<dyn NodeBehavior>>>,
}

impl Transform {
    pub fn new() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::one(),
            scale: Vector3::new(1.0, 1.0, 1.0),
            parent: None,
        }
    }

    pub fn from_position(pos: Vector3<f32>) -> Self {
        Self {
            position: pos,
            ..Self::new()
        }
    }

    pub fn set_parent(&mut self, parent: Option<Weak<RefCell<dyn NodeBehavior>>>) {
        self.parent = parent;
    }

    pub fn add_transformation(&mut self, other: &Transform) {
        self.position += other.position;
        self.rotation = other.rotation * self.rotation;
        self.scale.x *= other.scale.x;
        self.scale.y *= other.scale.y;
        self.scale.z *= other.scale.z;
    }

    /// Get world position
    pub fn get_world_position(&self) -> Vector3<f32> {
        if let Some(parent_weak) = &self.parent {
            if let Some(parent_rc) = parent_weak.upgrade() {
                let parent_ref = parent_rc.borrow(); // keep borrow alive
                let parent_transform = parent_ref.transform();
                let parent_world_pos = parent_transform.get_world_position();
                parent_world_pos + self.position
            } else {
                self.position
            }
        } else {
            self.position
        }
    }

    /// Get world rotation
    pub fn get_world_rotation(&self) -> Quaternion<f32> {
        if let Some(parent_weak) = &self.parent {
            if let Some(parent_rc) = parent_weak.upgrade() {
                let parent_ref = parent_rc.borrow();
                let parent_transform = parent_ref.transform();
                let parent_world_rot = parent_transform.get_world_rotation();
                parent_world_rot * self.rotation
            } else {
                self.rotation
            }
        } else {
            self.rotation
        }
    }

    /// Get world scale
    pub fn get_world_scale(&self) -> Vector3<f32> {
        if let Some(parent_weak) = &self.parent {
            if let Some(parent_rc) = parent_weak.upgrade() {
                let parent_ref = parent_rc.borrow();
                let parent_transform = parent_ref.transform();
                let parent_world_scale = parent_transform.get_world_scale();

                Vector3::new(
                    self.scale.x * parent_world_scale.x,
                    self.scale.y * parent_world_scale.y,
                    self.scale.z * parent_world_scale.z,
                )
            } else {
                self.scale
            }
        } else {
            self.scale
        }
    }

    /// Final transformation matrix
    pub fn get_matrix(&self) -> Matrix4<f32> {
        let world_pos = self.get_world_position();
        let world_rot = self.get_world_rotation();
        let world_scale = self.get_world_scale();

        Matrix4::from_translation(world_pos)
            * Matrix4::from(world_rot)
            * Matrix4::from_nonuniform_scale(world_scale.x, world_scale.y, world_scale.z)
    }

    /// Look along a direction
    pub fn look_along(&mut self, direction: Vector3<f32>, up: Vector3<f32>) {
        let dir = direction.normalize();
        if dir.magnitude2() > 0.0 {
            let forward = -dir;
            let right = up.cross(forward).normalize();
            let real_up = forward.cross(right).normalize();

            let rot_matrix = Matrix3::from_cols(right, real_up, forward);
            self.rotation = Quaternion::from(rot_matrix);
        }
    }

    /// Look at a target point
    pub fn look_at(&mut self, target: Vector3<f32>, up: Vector3<f32>) {
        let dir = (target - self.position).normalize();
        self.look_along(dir, up);
    }

    pub fn forward(&self) -> Vector3<f32> {
        self.rotation * -Vector3::unit_z()
    }

    pub fn up(&self) -> Vector3<f32> {
        self.rotation * Vector3::unit_y()
    }

    pub fn set_rotation_euler(&mut self, x: f32, y: f32, z: f32) {
        self.rotation = Quaternion::from_angle_x(Deg(x))
            * Quaternion::from_angle_y(Deg(y))
            * Quaternion::from_angle_z(Deg(z));
    }

    pub fn add_rotation_euler_local(&mut self, x: f32, y: f32, z: f32) {
        let delta = Quaternion::from_angle_x(Deg(x))
            * Quaternion::from_angle_y(Deg(y))
            * Quaternion::from_angle_z(Deg(z));
        self.rotation = self.rotation * delta;
    }

    pub fn add_rotation_euler_world(&mut self, x: f32, y: f32, z: f32) {
        let delta = Quaternion::from_angle_x(Deg(x))
            * Quaternion::from_angle_y(Deg(y))
            * Quaternion::from_angle_z(Deg(z));
        self.rotation = delta * self.rotation;
    }

    pub fn set_scale(&mut self, scale: Vector3<f32>) {
        self.scale = scale;
    }

    pub fn set_position(&mut self, position: Vector3<f32>) {
        self.position = position;
    }

    pub fn set_rotation(&mut self, rotation: Quaternion<f32>) {
        self.rotation = rotation;
    }

    pub fn scale_by(&mut self, factor: f32) {
        self.scale.x *= factor;
        self.scale.y *= factor;
        self.scale.z *= factor;
    }

    pub fn rotate_around_world(&mut self, world_point: Vector3<f32>, rotation: Quaternion<f32>) {
        let current = Matrix4::from_translation(self.position) * Matrix4::from(self.rotation);
        let new_m = Matrix4::from_translation(world_point)
            * Matrix4::from(rotation)
            * Matrix4::from_translation(-world_point)
            * current;

        let rotation_matrix =
            Matrix3::from_cols(new_m.x.truncate(), new_m.y.truncate(), new_m.z.truncate());
        self.rotation = Quaternion::from(rotation_matrix).normalize();
        self.position = new_m.w.truncate();
    }
}
