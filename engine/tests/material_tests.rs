use cgmath::Vector4;
use formosaic_engine::architecture::models::material::{AlphaMode, Material};

#[test]
fn material_defaults_are_stable() {
    let mat = Material::default();
    assert_eq!(mat.name, None);
    assert_eq!(mat.diffuse_color, Vector4::new(1.0, 1.0, 1.0, 1.0));
    assert_eq!(mat.ambient_color, Vector4::new(0.0, 0.0, 0.0, 1.0));
    assert_eq!(mat.metallic_factor, 0.0);
    assert_eq!(mat.roughness_factor, 0.5);
    assert_eq!(mat.cull_backface, false);
    assert_eq!(mat.alpha_mode, AlphaMode::Opaque);
    assert_eq!(mat.emissive_strength, 1.0);
}

#[test]
fn builder_helpers_mutate_expected_fields() {
    let mat = Material::new()
        .with_diffuse_color(Vector4::new(0.2, 0.3, 0.4, 0.5))
        .with_specular_color(Vector4::new(0.6, 0.7, 0.8, 0.9))
        .with_ambient_color(Vector4::new(0.1, 0.2, 0.3, 0.4))
        .with_emissive_color(Vector4::new(0.9, 0.1, 0.2, 0.3))
        .with_transparent_color(Vector4::new(0.01, 0.02, 0.03, 0.04))
        .disable_backface_culling();

    assert_eq!(mat.diffuse_color, Vector4::new(0.2, 0.3, 0.4, 0.5));
    assert_eq!(mat.specular_color, Vector4::new(0.6, 0.7, 0.8, 0.9));
    assert_eq!(mat.ambient_color, Vector4::new(0.1, 0.2, 0.3, 0.4));
    assert_eq!(mat.emissive_color, Vector4::new(0.9, 0.1, 0.2, 0.3));
    assert_eq!(mat.transparent_color, Vector4::new(0.01, 0.02, 0.03, 0.04));
    assert_eq!(mat.cull_backface, false);
}

#[test]
fn has_emissive_detects_color_or_texture() {
    let mat = Material::new();
    assert!(!mat.has_emissive());

    let mat = Material::new().with_emissive_color(Vector4::new(0.2, 0.0, 0.0, 1.0));
    assert!(mat.has_emissive());
}
