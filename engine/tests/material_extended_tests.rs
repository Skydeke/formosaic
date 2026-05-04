use cgmath::Vector4;
use formosaic_engine::architecture::models::material::{AlphaMode, Material};

#[test]
fn alpha_mode_default_is_opaque() {
    assert_eq!(AlphaMode::default(), AlphaMode::Opaque);
}

#[test]
fn alpha_mode_equality_works() {
    assert_eq!(AlphaMode::Opaque, AlphaMode::Opaque);
    assert_eq!(AlphaMode::Mask(0.5), AlphaMode::Mask(0.5));
    assert_eq!(AlphaMode::Blend, AlphaMode::Blend);

    assert_ne!(AlphaMode::Opaque, AlphaMode::Blend);
    assert_ne!(AlphaMode::Mask(0.3), AlphaMode::Mask(0.7));
}

#[test]
fn alpha_mode_debug_repr() {
    let repr = format!("{:?}", AlphaMode::Mask(0.5));
    assert!(repr.contains("Mask"));
}

#[test]
fn material_new_equals_default() {
    let m1 = Material::new();
    let m2 = Material::default();
    assert_eq!(m1.diffuse_color, m2.diffuse_color);
    assert_eq!(m1.name, m2.name);
}

#[test]
fn material_builder_chains() {
    let mat = Material::new()
        .with_diffuse_color(Vector4::new(1.0, 0.0, 0.0, 1.0))
        .with_specular_color(Vector4::new(0.5, 0.5, 0.5, 1.0));
    assert_eq!(mat.diffuse_color, Vector4::new(1.0, 0.0, 0.0, 1.0));
    assert_eq!(mat.specular_color, Vector4::new(0.5, 0.5, 0.5, 1.0));
}

#[test]
fn material_builder_enable_culling() {
    let mat = Material::new()
        .disable_backface_culling()
        .enable_backface_culling();
    assert!(mat.cull_backface);
}

#[test]
fn material_has_emissive_with_zero_color() {
    let mat = Material::new().with_emissive_color(Vector4::new(0.0, 0.0, 0.0, 1.0));
    assert!(!mat.has_emissive());
}

#[test]
fn material_has_emissive_with_small_color_below_threshold() {
    let mat = Material::new().with_emissive_color(Vector4::new(0.0001, 0.0, 0.0, 1.0));
    assert!(!mat.has_emissive());
}

#[test]
fn material_has_emissive_with_threshold_color() {
    let mat = Material::new().with_emissive_color(Vector4::new(0.002, 0.0, 0.0, 1.0));
    assert!(mat.has_emissive());
}

#[test]
fn material_has_emissive_y_channel() {
    let mat = Material::new().with_emissive_color(Vector4::new(0.0, 0.002, 0.0, 1.0));
    assert!(mat.has_emissive());
}

#[test]
fn material_has_emissive_z_channel() {
    let mat = Material::new().with_emissive_color(Vector4::new(0.0, 0.0, 0.002, 1.0));
    assert!(mat.has_emissive());
}

#[test]
fn material_default_has_no_textures() {
    let mat = Material::default();
    assert!(mat.diffuse_texture.is_none());
    assert!(mat.normal_texture.is_none());
    assert!(mat.metallic_roughness_texture.is_none());
    assert!(mat.emissive_texture.is_none());
    assert!(mat.occlusion_texture.is_none());
    assert!(mat.specular_texture.is_none());
}

#[test]
fn material_default_culls_backfaces() {
    let mat = Material::default();
    assert!(mat.cull_backface);
}

#[test]
fn material_default_alpha_is_opaque() {
    let mat = Material::default();
    assert_eq!(mat.alpha_mode, AlphaMode::Opaque);
}

#[test]
fn material_builder_does_not_mutate_original() {
    let mat1 = Material::new();
    let mat2 = mat1.clone().with_diffuse_color(Vector4::new(0.0, 1.0, 0.0, 1.0));
    assert_eq!(mat1.diffuse_color, Vector4::new(1.0, 1.0, 1.0, 1.0));
    assert_eq!(mat2.diffuse_color, Vector4::new(0.0, 1.0, 0.0, 1.0));
}

#[test]
fn material_clone_copies_all_fields() {
    let mat1 = Material::new()
        .with_diffuse_color(Vector4::new(0.1, 0.2, 0.3, 0.4))
        .disable_backface_culling();
    let mat2 = mat1.clone();
    assert_eq!(mat1.diffuse_color, mat2.diffuse_color);
    assert_eq!(mat1.cull_backface, mat2.cull_backface);
    assert_eq!(mat1.alpha_mode, mat2.alpha_mode);
}

#[test]
fn material_specular_defaults_to_white() {
    let mat = Material::default();
    assert_eq!(mat.specular_color, Vector4::new(1.0, 1.0, 1.0, 1.0));
}

#[test]
fn material_transparent_defaults_to_zero() {
    let mat = Material::default();
    assert_eq!(mat.transparent_color, Vector4::new(0.0, 0.0, 0.0, 0.0));
}

#[test]
fn material_emissive_defaults_to_black() {
    let mat = Material::default();
    assert_eq!(mat.emissive_color, Vector4::new(0.0, 0.0, 0.0, 1.0));
}
