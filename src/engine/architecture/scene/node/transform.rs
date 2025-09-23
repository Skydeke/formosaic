use cgmath::{Deg, InnerSpace, Matrix3, Matrix4, One, Quaternion, Rotation3, Vector3};

#[derive(Clone, Debug)]
pub struct Transform {
    pub position: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub scale: Vector3<f32>,

    /// Optional parent transformations
    pub parent_transforms: Vec<Transform>,
}

impl Transform {
    pub fn new() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::one(),
            scale: Vector3::new(1.0, 1.0, 1.0),
            parent_transforms: vec![],
        }
    }

    pub fn from_position(pos: Vector3<f32>) -> Self {
        Self {
            position: pos,
            ..Self::new()
        }
    }

    pub fn add_parent(&mut self, parent: Transform) {
        self.parent_transforms.push(parent);
    }

    /// Add a transformation to this transform (used by node system)
    pub fn add_transformation(&mut self, other: &Transform) {
        self.position += other.position;
        self.rotation = other.rotation * self.rotation;
        self.scale.x *= other.scale.x;
        self.scale.y *= other.scale.y;
        self.scale.z *= other.scale.z;
    }

    /// Returns a 4x4 transformation matrix
    pub fn get_matrix(&self) -> Matrix4<f32> {
        let mut transform = Matrix4::from_translation(self.position)
            * Matrix4::from(self.rotation)
            * Matrix4::from_nonuniform_scale(self.scale.x, self.scale.y, self.scale.z);

        for parent in &self.parent_transforms {
            transform = parent.get_matrix() * transform;
        }

        transform
    }

    /// Look along a specific direction
    pub fn look_along(&mut self, direction: Vector3<f32>, up: Vector3<f32>) {
        let dir = direction.normalize();
        if dir.magnitude2() > 0.0 {
            let forward = -dir;
            let right = up.cross(forward).normalize();
            let real_up = forward.cross(right).normalize();

            // Create rotation matrix from basis vectors
            let rot_matrix = cgmath::Matrix3::from_cols(right, real_up, forward);

            // Convert rotation matrix to quaternion
            self.rotation = Quaternion::from(rot_matrix);
        }
    }

    /// Look at a point in world space
    pub fn look_at(&mut self, target: Vector3<f32>, up: Vector3<f32>) {
        let dir = (target - self.position).normalize();
        self.look_along(dir, up);
    }

    /// Get the forward vector
    pub fn forward(&self) -> Vector3<f32> {
        self.rotation * -Vector3::unit_z()
    }

    pub fn up(&self) -> Vector3<f32> {
        self.rotation * Vector3::unit_y()
    }

    /// Set rotation from Euler angles (in degrees)
    pub fn set_rotation_euler(&mut self, x: f32, y: f32, z: f32) {
        self.rotation = Quaternion::from_angle_x(Deg(x))
            * Quaternion::from_angle_y(Deg(y))
            * Quaternion::from_angle_z(Deg(z));
    }

    /// Add rotation from Euler angles (in degrees, local space)
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

    /// Set scale
    pub fn set_scale(&mut self, scale: Vector3<f32>) {
        self.scale = scale;
    }

    /// Set position
    pub fn set_position(&mut self, position: Vector3<f32>) {
        self.position = position;
    }

    /// Set rotation
    pub fn set_rotation(&mut self, rotation: Quaternion<f32>) {
        self.rotation = rotation;
    }

    /// Scale multiplicatively
    pub fn scale_by(&mut self, factor: f32) {
        self.scale.x *= factor;
        self.scale.y *= factor;
        self.scale.z *= factor;
    }

    /// Rotate this transform around a point in world space
    pub fn rotate_around_world(&mut self, world_point: Vector3<f32>, rotation: Quaternion<f32>) {
        let current = Matrix4::from_translation(self.position) * Matrix4::from(self.rotation);
        let new_m = Matrix4::from_translation(world_point)
            * Matrix4::from(rotation)
            * Matrix4::from_translation(-world_point)
            * current;

        // extract rotation and position
        let rotation_matrix =
            Matrix3::from_cols(new_m.x.truncate(), new_m.y.truncate(), new_m.z.truncate());
        self.rotation = Quaternion::from(rotation_matrix).normalize();
        self.position = new_m.w.truncate();
    }
}
