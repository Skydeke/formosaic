use cgmath::Vector4;

#[derive(Debug, Clone, Copy)]
pub struct ClipPlane {
    value: Vector4<f32>,
}

impl ClipPlane {
    /// Represents no clipping
    pub const NONE: ClipPlane = ClipPlane {
        value: Vector4 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 0.0,
        },
    };

    /// Create a clip plane from coefficients a, b, c, d
    pub fn of(a: f32, b: f32, c: f32, d: f32) -> Self {
        ClipPlane {
            value: Vector4::new(a, b, c, d),
        }
    }

    /// Horizontal plane above a given height (y > height)
    pub fn of_above(height: f32) -> Self {
        ClipPlane {
            value: Vector4::new(0.0, 1.0, 0.0, -height),
        }
    }

    /// Horizontal plane below a given height (y < height)
    pub fn of_below(height: f32) -> Self {
        ClipPlane {
            value: Vector4::new(0.0, -1.0, 0.0, height),
        }
    }

    /// Get the plane coefficients
    pub fn get_value(&self) -> Vector4<f32> {
        self.value
    }
}
