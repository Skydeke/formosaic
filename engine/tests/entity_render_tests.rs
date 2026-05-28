use formosaic_engine::architecture::models::material::{AlphaMode, Material};
use formosaic_engine::rendering::instances::entity_render::{material_key, MaterialKey};

fn make_mat(cull_backface: bool, alpha_mode: AlphaMode) -> Material {
    Material {
        cull_backface,
        alpha_mode,
        ..Material::default()
    }
}

#[test]
fn material_key_handles_culling() {
    let with_cull = make_mat(true, AlphaMode::Opaque);
    let no_cull = make_mat(false, AlphaMode::Opaque);
    assert_ne!(material_key(Some(&with_cull)), material_key(Some(&no_cull)));
    assert!(material_key(Some(&with_cull)).cull_backface);
    assert!(!material_key(Some(&no_cull)).cull_backface);
}

#[test]
fn material_key_handles_alpha_blend() {
    let opaque = make_mat(true, AlphaMode::Opaque);
    let blend = make_mat(true, AlphaMode::Blend);
    assert_ne!(material_key(Some(&opaque)), material_key(Some(&blend)));
    let key = material_key(Some(&blend));
    assert!(key.alpha_blend);
    assert!(!key.alpha_mask);
}

#[test]
fn material_key_handles_alpha_mask() {
    let opaque = make_mat(true, AlphaMode::Opaque);
    let mask = make_mat(true, AlphaMode::Mask(0.5));
    assert_ne!(material_key(Some(&opaque)), material_key(Some(&mask)));
    let key = material_key(Some(&mask));
    assert!(!key.alpha_blend);
    assert!(key.alpha_mask);
}

#[test]
fn material_key_none() {
    assert_eq!(
        material_key(None),
        MaterialKey {
            cull_backface: false,
            alpha_blend: false,
            alpha_mask: false,
            diffuse_tex_id: None,
        }
    );
}

#[test]
fn material_key_groups_identical() {
    let a = make_mat(true, AlphaMode::Opaque);
    let b = make_mat(true, AlphaMode::Opaque);
    assert_eq!(material_key(Some(&a)), material_key(Some(&b)));
}

#[test]
fn material_sort_groups_same_keys_together() {
    let mat_a = make_mat(false, AlphaMode::Blend);
    let mat_b = make_mat(true, AlphaMode::Opaque);
    let key_a = material_key(Some(&mat_a));
    let key_b = material_key(Some(&mat_b));

    let mut keys = vec![key_b.clone(), key_a.clone(), key_b.clone(), key_a.clone()];
    keys.sort_by(|a, b| a.cmp(b));

    assert_eq!(keys[0], key_a);
    assert_eq!(keys[1], key_a);
    assert_eq!(keys[2], key_b);
    assert_eq!(keys[3], key_b);
}
