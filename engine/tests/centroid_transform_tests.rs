//! Tests for the centroid computation applying mesh node transforms.
//!
//! Bug: the orbit camera targeted the raw vertex centroid (without mesh
//! transforms), so for hierarchical models where body parts are positioned via
//! node transforms (e.g. stylized characters from Poly.pizza, or the Character
//! Soldier FBX), the camera orbited around the scene origin instead of the
//! model's visual centre.  The character appeared to have its triangles at the
//! wrong position relative to the camera — typically at the feet level.

use cgmath::{InnerSpace, Matrix4, SquareMatrix, Vector3, Vector4};

// ─── helpers ─────────────────────────────────────────────────────────────────

fn identity() -> Matrix4<f32> {
    Matrix4::identity()
}

fn translation(x: f32, y: f32, z: f32) -> Matrix4<f32> {
    Matrix4::from_translation(Vector3::new(x, y, z))
}

/// Compute the centroid the OLD (buggy) way: average raw vertex positions
/// without applying mesh transforms.
fn centroid_without_transforms(
    vertices: &[Vector3<f32>],
    _mesh_transform: Matrix4<f32>,
) -> Option<Vector3<f32>> {
    if vertices.is_empty() {
        return None;
    }
    let sum: Vector3<f32> = vertices.iter().copied().sum();
    Some(sum / vertices.len() as f32)
}

/// Compute the centroid the NEW (fixed) way: apply the mesh's node transform
/// before averaging, so the result is in world space.
fn centroid_with_transforms(
    vertices: &[Vector3<f32>],
    mesh_transform: Matrix4<f32>,
) -> Option<Vector3<f32>> {
    if vertices.is_empty() {
        return None;
    }
    let sum: Vector3<f32> = vertices
        .iter()
        .map(|v| {
            let w = mesh_transform * Vector4::new(v.x, v.y, v.z, 1.0);
            Vector3::new(w.x, w.y, w.z)
        })
        .sum();
    Some(sum / vertices.len() as f32)
}

// ─── tests ───────────────────────────────────────────────────────────────────

/// Identity transform: both methods must agree (baseline).
#[test]
fn centroid_identity_transform_is_unchanged() {
    let verts = vec![
        Vector3::new(1.0_f32, 0.0, 0.0),
        Vector3::new(-1.0, 0.0, 0.0),
    ];
    let xform = identity();
    let old = centroid_without_transforms(&verts, xform).unwrap();
    let new = centroid_with_transforms(&verts, xform).unwrap();
    assert!(
        (old - new).magnitude() < 1e-5,
        "identity transform should not change centroid: old={:?} new={:?}",
        old, new
    );
}

/// A mesh translated 10 units up: the OLD method returns (0,0,0) (wrong),
/// the NEW method returns (0,10,0) (correct world centre).
#[test]
fn centroid_accounts_for_translation_transform() {
    // A unit cube centred at the local origin
    let verts = vec![
        Vector3::new( 1.0_f32,  1.0,  1.0),
        Vector3::new(-1.0,  1.0,  1.0),
        Vector3::new( 1.0, -1.0,  1.0),
        Vector3::new(-1.0, -1.0,  1.0),
        Vector3::new( 1.0,  1.0, -1.0),
        Vector3::new(-1.0,  1.0, -1.0),
        Vector3::new( 1.0, -1.0, -1.0),
        Vector3::new(-1.0, -1.0, -1.0),
    ];
    // Node places this mesh 10 units above the origin (typical for a body mesh
    // in a character whose feet are at Y=0).
    let xform = translation(0.0, 10.0, 0.0);

    let old = centroid_without_transforms(&verts, xform).unwrap();
    let new = centroid_with_transforms(&verts, xform).unwrap();

    // OLD method: centroid of raw local verts = (0,0,0) — completely wrong
    assert!(
        old.magnitude() < 1e-4,
        "old (buggy) method centroid should be at origin, got {:?}",
        old
    );

    // NEW method: centroid in world space = (0, 10, 0) — the actual mesh centre
    let expected = Vector3::new(0.0_f32, 10.0, 0.0);
    assert!(
        (new - expected).magnitude() < 1e-4,
        "new (fixed) method centroid should be {:?}, got {:?}",
        expected, new
    );
}

/// Multi-mesh model: each mesh is at a different node position.
/// The overall centroid must be the weighted average in world space.
#[test]
fn centroid_is_world_average_across_multiple_meshes() {
    // Mesh A: body at Y=1 (two verts centred at Y=1 locally, moved to Y=6)
    let verts_a = vec![
        Vector3::new(0.0_f32,  1.0, 0.0),
        Vector3::new(0.0,     -1.0, 0.0),
    ];
    let xform_a = translation(0.0, 5.0, 0.0); // body at world Y=5..7, local centre Y=0→world Y=5

    // Mesh B: head at Y=8 (two verts centred locally, moved to Y=8)
    let verts_b = vec![
        Vector3::new(0.0_f32,  0.5, 0.0),
        Vector3::new(0.0,     -0.5, 0.0),
    ];
    let xform_b = translation(0.0, 8.0, 0.0); // head centre at world Y=8

    // Collect all world-space vertices (mirroring the fixed loader logic)
    let all_world: Vec<Vector3<f32>> = verts_a
        .iter()
        .map(|v| {
            let w = xform_a * Vector4::new(v.x, v.y, v.z, 1.0);
            Vector3::new(w.x, w.y, w.z)
        })
        .chain(verts_b.iter().map(|v| {
            let w = xform_b * Vector4::new(v.x, v.y, v.z, 1.0);
            Vector3::new(w.x, w.y, w.z)
        }))
        .collect();

    // Sum / count (as the fixed loader does across all meshes)
    let sum: Vector3<f32> = all_world.iter().copied().sum();
    let centroid = sum / all_world.len() as f32;

    // World Y positions: 6, 4, 8.5, 7.5  → avg = (6+4+8.5+7.5)/4 = 26/4 = 6.5
    let expected_y = 6.5_f32;
    assert!(
        (centroid.y - expected_y).abs() < 1e-4,
        "centroid Y should be {}, got {}",
        expected_y,
        centroid.y
    );
}

/// A mesh with only vertices at the origin stays at the origin regardless of
/// the transform (regression check — no NaN or infinity).
#[test]
fn centroid_origin_vertices_stay_at_origin() {
    let verts = vec![
        Vector3::new(0.0_f32, 0.0, 0.0),
        Vector3::new(0.0,     0.0, 0.0),
    ];
    let xform = identity();
    let c = centroid_with_transforms(&verts, xform).unwrap();
    assert!(c.x.is_finite() && c.y.is_finite() && c.z.is_finite());
    assert!(c.magnitude() < 1e-5);
}

/// Empty vertex list should return None (no panic).
#[test]
fn centroid_empty_mesh_returns_none() {
    let c = centroid_with_transforms(&[], identity());
    assert!(c.is_none());
}
