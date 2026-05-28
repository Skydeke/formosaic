use cgmath::Vector2;
use formosaic_engine::rendering::instances::camera::camera::Camera;
use formosaic_engine::rendering::instances::camera::orthographic_projection::OrthographicProjection;

#[test]
fn perspective_projection_updates_on_resolution_change() {
    let mut cam = Camera::new();
    cam.set_resolution(Vector2::new(1280, 720));
    cam.update_projection_matrix();
    let first = cam.projection_matrix;

    cam.set_resolution(Vector2::new(720, 1280));
    cam.update_projection_matrix();
    let second = cam.projection_matrix;

    assert_ne!(
        first, second,
        "aspect ratio should affect projection matrix"
    );
}

#[test]
fn orthographic_projection_is_deterministic() {
    let mut cam = Camera::new();
    cam.set_projection(Box::new(OrthographicProjection::new(
        -2.0, 2.0, -1.0, 1.0, 0.1, 100.0,
    )));
    cam.set_resolution(Vector2::new(800, 600));
    cam.update_projection_matrix();

    let expected = cam.projection_matrix;
    cam.update_projection_matrix();
    assert_eq!(expected, cam.projection_matrix);
}

#[test]
fn projection_view_matrix_is_product_of_projection_and_view() {
    let mut cam = Camera::new();
    cam.set_resolution(Vector2::new(1024, 768));
    cam.update_projection_matrix();
    cam.update_view_matrix();
    cam.update_projection_view_matrix();

    let expected = cam.projection_matrix * cam.view_matrix;
    assert_eq!(expected, cam.projection_view_matrix);
}
