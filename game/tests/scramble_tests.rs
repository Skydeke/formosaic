use cgmath::{Matrix4, SquareMatrix, Vector3};
use formosaic::puzzle::scramble_math::{compute_scramble_offsets, lerp_positions};

fn pos(x: f32, y: f32, z: f32) -> [f32; 3] {
    [x, y, z]
}

fn two_tri_quad() -> Vec<f32> {
    let mut v = Vec::new();
    v.extend_from_slice(&pos(-1.0, 0.0, -1.0));
    v.extend_from_slice(&pos(1.0, 0.0, -1.0));
    v.extend_from_slice(&pos(1.0, 0.0, 1.0));
    v.extend_from_slice(&pos(-1.0, 0.0, -1.0));
    v.extend_from_slice(&pos(1.0, 0.0, 1.0));
    v.extend_from_slice(&pos(-1.0, 0.0, 1.0));
    v
}

#[test]
fn each_triangle_receives_same_offset_per_corner() {
    let verts = two_tri_quad();
    let off = compute_scramble_offsets(
        verts.len(),
        Vector3::unit_y(),
        0.1,
        0.5,
        Matrix4::identity(),
    );
    assert_eq!(off[0..3], off[3..6], "tri0 corner0 ≠ corner1");
    assert_eq!(off[0..3], off[6..9], "tri0 corner0 ≠ corner2");
    assert_eq!(off[9..12], off[12..15], "tri1 corner0 ≠ corner1");
    assert_eq!(off[9..12], off[15..18], "tri1 corner0 ≠ corner2");
}

#[test]
fn different_triangles_can_have_different_offsets() {
    let verts = two_tri_quad();
    let off = compute_scramble_offsets(
        verts.len(),
        Vector3::unit_y(),
        0.1,
        999.0,
        Matrix4::identity(),
    );
    assert_ne!(off[0..3], off[9..12], "tri0 and tri1 have same offset");
}

#[test]
fn offset_along_axis_with_identity_transform() {
    let verts = two_tri_quad();
    let off = compute_scramble_offsets(
        verts.len(),
        Vector3::unit_y(),
        0.1,
        0.5,
        Matrix4::identity(),
    );
    for i in 0..off.len() / 3 {
        assert_eq!(off[i * 3], 0.0, "x at tri {}", i);
        assert_eq!(off[i * 3 + 2], 0.0, "z at tri {}", i);
        let y = off[i * 3 + 1];
        assert!(y >= 0.1 && y <= 0.5, "y={} out of range at tri {}", y, i);
    }
}

#[test]
fn offset_along_rotated_axis() {
    let rot = Matrix4::from_angle_z(cgmath::Deg(90.0));
    let verts = two_tri_quad();
    let off = compute_scramble_offsets(verts.len(), Vector3::unit_y(), 0.1, 0.5, rot);
    for i in 0..off.len() / 3 {
        assert!(
            off[i * 3 + 1].abs() < 1e-6,
            "y={} not near zero at tri {}",
            off[i * 3 + 1],
            i
        );
        assert!(
            off[i * 3 + 2].abs() < 1e-6,
            "z={} not near zero at tri {}",
            off[i * 3 + 2],
            i
        );
        let x = off[i * 3];
        assert!(
            x.abs() >= 0.1 && x.abs() <= 0.5,
            "|x|={} out of range at tri {}",
            x.abs(),
            i
        );
    }
}

#[test]
fn empty_mesh_returns_empty_offsets() {
    let off = compute_scramble_offsets(0, Vector3::unit_y(), 0.1, 0.5, Matrix4::identity());
    assert!(off.is_empty());
}

#[test]
fn offset_range_is_respected() {
    let verts = two_tri_quad();
    let off = compute_scramble_offsets(
        verts.len(),
        Vector3::unit_y(),
        0.3,
        0.301,
        Matrix4::identity(),
    );
    for i in 0..off.len() / 3 {
        let y = off[i * 3 + 1];
        assert!(y >= 0.3 && y <= 0.301, "y={} out of range at tri {}", y, i);
    }
}

// ── lerp_positions ─────────────────────────────────────────────

#[test]
fn lerp_t_zero_returns_original() {
    let pos = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let off = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6];
    let r = lerp_positions(&pos, &off, 0.0);
    assert_eq!(r, pos);
}

#[test]
fn lerp_t_one_returns_pos_plus_offsets() {
    let pos = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let off = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6];
    let r = lerp_positions(&pos, &off, 1.0);
    for i in 0..6 {
        assert_eq!(r[i], pos[i] + off[i]);
    }
}

#[test]
fn lerp_t_half_is_midpoint() {
    let pos = vec![1.0, 2.0, 3.0];
    let off = vec![0.2, 0.4, 0.6];
    let r = lerp_positions(&pos, &off, 0.5);
    assert_eq!(r[0], 1.1);
    assert_eq!(r[1], 2.2);
    assert_eq!(r[2], 3.3);
}

#[test]
fn lerp_t_shorter_offsets_truncated() {
    let pos = vec![1.0, 2.0, 3.0, 4.0];
    let off = vec![10.0, 20.0];
    let r = lerp_positions(&pos, &off, 1.0);
    assert_eq!(r.len(), 2);
    assert_eq!(r[0], 11.0);
    assert_eq!(r[1], 22.0);
}

// ── Integration: compute then lerp ─────────────────────────────

#[test]
fn compute_then_lerp_restores_original() {
    let verts = two_tri_quad();
    let off = compute_scramble_offsets(
        verts.len(),
        Vector3::unit_y(),
        0.1,
        0.5,
        Matrix4::identity(),
    );
    let scrambled = lerp_positions(&verts, &off, 1.0);
    assert_ne!(scrambled, verts);
    let restored = lerp_positions(&verts, &off, 0.0);
    assert_eq!(restored, verts);
}

#[test]
fn compute_then_lerp_half_is_between() {
    let verts = two_tri_quad();
    let off = compute_scramble_offsets(
        verts.len(),
        Vector3::unit_y(),
        0.1,
        0.5,
        Matrix4::identity(),
    );
    let original = verts.clone();
    let half = lerp_positions(&verts, &off, 0.5);
    let full = lerp_positions(&verts, &off, 1.0);
    for i in 0..verts.len() {
        let lo = original[i].min(full[i]);
        let hi = original[i].max(full[i]);
        assert!(
            half[i] >= lo && half[i] <= hi,
            "half[{}]={} not between [{}, {}]",
            i,
            half[i],
            lo,
            hi
        );
    }
}
