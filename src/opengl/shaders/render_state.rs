use cgmath::Matrix4;

// Generic render state that can hold different types of uniform data
pub struct RenderState {
    pub mvp_matrix: Matrix4<f32>,
    pub time: f32,
}

impl RenderState {
    pub fn new() -> Self {
        Self {
            mvp_matrix: Matrix4::from_scale(1.0),
            time: 0.0,
        }
    }
}
