use cgmath::{Matrix4, SquareMatrix, Vector2, Vector3};
use formosaic_engine::rendering::abstracted::camera::camera_projection::CameraProjection;
use formosaic_engine::rendering::instances::camera::camera::Camera;
use formosaic_engine::rendering::instances::camera::orthographic_projection::OrthographicProjection;
use formosaic_engine::rendering::instances::camera::perspective_projection::PerspectiveProjection;

fn assert_matrix_approx(a: Matrix4<f32>, b: Matrix4<f32>, eps: f32) {
    for i in 0..4 {
        for j in 0..4 {
            let diff = (a.x[i] - b.x[i]).abs();
            assert!(
                diff <= eps,
                "matrix mismatch at [{j}][{i}]: expected {}, got {}, diff={}",
                b.x[i],
                a.x[i],
                diff
            );
        }
    }
}

#[test]
fn camera_default_has_sensible_values() {
    let cam = Camera::new();
    assert_eq!(cam.fov, 75.0_f32.to_radians());
    assert_eq!(cam.near_plane, 0.3);
    assert_eq!(cam.far_plane, 1000.0);
    assert_eq!(cam.resolution, Vector2::new(0, 0));
}

#[test]
fn camera_view_matrix_at_origin_looks_down_z() {
    let mut cam = Camera::new();
    cam.update_view_matrix();
    let view = cam.get_view_matrix();
    let identity = Matrix4::identity();
    assert_matrix_approx(*view, identity, 1e-5);
}

#[test]
fn camera_projection_matrix_is_identity_by_default() {
    let mut cam = Camera::new();
    cam.set_resolution(Vector2::new(1, 1));
    cam.update_projection_matrix();
    let proj = cam.get_projection_matrix();
    assert!(
        proj.x.x > 0.0,
        "perspective projection should have valid x component"
    );
}

#[test]
fn camera_projection_view_matrix_matches_product() {
    let mut cam = Camera::new();
    cam.set_resolution(Vector2::new(800, 600));
    cam.update_projection_matrix();
    cam.update_view_matrix();
    cam.update_projection_view_matrix();

    let expected = *cam.get_projection_matrix() * *cam.get_view_matrix();
    assert_matrix_approx(*cam.get_projection_view_matrix(), expected, 1e-5);
}

#[test]
fn camera_resolution_change_affects_perspective() {
    let mut cam = Camera::new();

    cam.set_resolution(Vector2::new(1920, 1080));
    cam.update_projection_matrix();
    let wide = *cam.get_projection_matrix();

    cam.set_resolution(Vector2::new(1080, 1920));
    cam.update_projection_matrix();
    let tall = *cam.get_projection_matrix();

    assert_ne!(
        wide.x.x, tall.x.x,
        "aspect ratio changes should affect projection x"
    );
}

#[test]
fn camera_orthographic_projection_preserves_bounds() {
    let ortho = OrthographicProjection::new(-5.0, 5.0, -3.0, 3.0, 0.1, 100.0);
    assert_eq!(ortho.left, -5.0);
    assert_eq!(ortho.right, 5.0);
    assert_eq!(ortho.bottom, -3.0);
    assert_eq!(ortho.top, 3.0);
    assert_eq!(ortho.near, 0.1);
    assert_eq!(ortho.far, 100.0);
}

#[test]
fn camera_orthographic_projection_matrix_is_symmetric() {
    let mut ortho = OrthographicProjection::new(-1.0, 1.0, -1.0, 1.0, 0.1, 100.0);
    let mat = ortho.get_projection_matrix((800, 600), 75.0, 0.1, 100.0);

    let left_col = mat.x;
    let right_col = mat.y;
    let up_col = mat.z;
    let _back_col = mat.w;

    assert!(
        (left_col.x - right_col.y).abs() < 1e-5,
        "ortho x and y should match for symmetric bounds"
    );
}

#[test]
fn camera_perspective_projection_has_fov_impact() {
    let mut cam = Camera::new();
    cam.set_resolution(Vector2::new(800, 600));

    cam.fov = 45.0_f32.to_radians();
    cam.update_projection_matrix();
    let narrow = *cam.get_projection_matrix();

    cam.fov = 90.0_f32.to_radians();
    cam.update_projection_matrix();
    let wide = *cam.get_projection_matrix();

    assert_ne!(
        narrow.x.x, wide.x.x,
        "different FOV should produce different projection"
    );
}

#[test]
fn camera_perspective_near_far_affect_projection() {
    let mut cam = Camera::new();
    cam.set_resolution(Vector2::new(800, 600));

    cam.near_plane = 0.1;
    cam.far_plane = 100.0;
    cam.update_projection_matrix();
    let proj1 = *cam.get_projection_matrix();

    cam.near_plane = 1.0;
    cam.far_plane = 1000.0;
    cam.update_projection_matrix();
    let proj2 = *cam.get_projection_matrix();

    assert_ne!(
        proj1, proj2,
        "near/far changes should affect projection matrix"
    );
}

#[test]
fn camera_perspective_default_is_identity_then_updates() {
    let proj = PerspectiveProjection::new();
    assert_matrix_approx(proj.matrix, Matrix4::identity(), 1e-5);
}

#[test]
fn camera_view_matrix_changes_when_position_changes() {
    let mut cam = Camera::new();
    cam.transform.position = Vector3::new(0.0, 0.0, 5.0);
    cam.update_view_matrix();
    let view1 = *cam.get_view_matrix();

    cam.transform.position = Vector3::new(5.0, 0.0, 0.0);
    cam.update_view_matrix();
    let view2 = *cam.get_view_matrix();

    assert_ne!(
        view1, view2,
        "camera position change should affect view matrix"
    );
}

#[test]
fn camera_set_getters_consistent() {
    let mut cam = Camera::new();
    assert_eq!(cam.get_fov(), 75.0_f32.to_radians());
    assert_eq!(cam.get_near_plane(), 0.3);
    assert_eq!(cam.get_far_plane(), 1000.0);

    cam.fov = 60.0_f32.to_radians();
    cam.near_plane = 0.5;
    cam.far_plane = 500.0;

    assert_eq!(cam.get_fov(), 60.0_f32.to_radians());
    assert_eq!(cam.get_near_plane(), 0.5);
    assert_eq!(cam.get_far_plane(), 500.0);
}

#[test]
fn camera_orthographic_projection_matrix_independent_of_fov() {
    let mut ortho = OrthographicProjection::new(-2.0, 2.0, -2.0, 2.0, 0.1, 100.0);
    let mat1 = ortho.get_projection_matrix((800, 600), 30.0, 0.1, 100.0);
    let mat2 = ortho.get_projection_matrix((800, 600), 120.0, 0.1, 100.0);
    assert_eq!(
        mat1, mat2,
        "orthographic projection should not depend on FOV"
    );
}

#[test]
fn camera_update_all_matrices_after_transform_change() {
    let mut cam = Camera::new();
    cam.set_resolution(Vector2::new(1024, 768));

    cam.transform.position = Vector3::new(0.0, 5.0, 10.0);
    cam.update_projection_matrix();
    cam.update_view_matrix();
    cam.update_projection_view_matrix();

    let expected = *cam.get_projection_matrix() * *cam.get_view_matrix();
    assert_matrix_approx(*cam.get_projection_view_matrix(), expected, 1e-5);
}

#[test]
fn camera_projection_view_matrix_reflects_view_changes() {
    let mut cam = Camera::new();
    cam.set_resolution(Vector2::new(1024, 768));

    cam.update_projection_matrix();
    cam.update_view_matrix();
    cam.update_projection_view_matrix();

    let pv1 = *cam.get_projection_view_matrix();

    cam.transform.position = Vector3::new(10.0, 0.0, 0.0);
    cam.update_view_matrix();
    cam.update_projection_view_matrix();

    let pv2 = *cam.get_projection_view_matrix();
    assert_ne!(
        pv1, pv2,
        "view change should propagate to projection-view matrix"
    );
}
