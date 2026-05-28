use cgmath::{InnerSpace, Matrix3, Matrix4, SquareMatrix, Vector3};

pub fn compute_scramble_offsets(
    vertex_count: usize,
    axis: Vector3<f32>,
    min_disp: f32,
    max_disp: f32,
    mesh_transform: Matrix4<f32>,
) -> Vec<f32> {
    use rand::Rng;

    if vertex_count == 0 {
        return vec![];
    }

    let axis = axis.normalize();
    let basis = Matrix3::new(
        mesh_transform.x.x,
        mesh_transform.x.y,
        mesh_transform.x.z,
        mesh_transform.y.x,
        mesh_transform.y.y,
        mesh_transform.y.z,
        mesh_transform.z.x,
        mesh_transform.z.y,
        mesh_transform.z.z,
    );
    let inv_basis = basis.invert().unwrap_or(Matrix3::from_scale(1.0));
    let mut rng = rand::rng();
    let mut offsets = vec![0.0f32; vertex_count];

    let tri_count = vertex_count / 9;
    for tri in 0..tri_count {
        let amount: f32 = rng.random_range(min_disp..max_disp);
        let disp = inv_basis * (axis * amount);
        let base = tri * 9;
        for corner in 0..3 {
            let v = base + corner * 3;
            offsets[v] = disp.x;
            offsets[v + 1] = disp.y;
            offsets[v + 2] = disp.z;
        }
    }
    offsets
}

pub fn lerp_positions(positions: &[f32], offsets: &[f32], t: f32) -> Vec<f32> {
    let n = positions.len().min(offsets.len());
    let mut data = vec![0.0f32; n];
    for i in 0..n {
        data[i] = positions[i] + offsets[i] * t;
    }
    data
}
