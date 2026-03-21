//! Platform-specific asset loading.
//!
//! On desktop: reads from the `assets/` directory next to the game crate.
//! On Android: reads from the APK asset manager via JNI.
//!
//! The engine's `ModelLoader` is deliberately unaware of either platform;
//! callers obtain bytes here and pass them to `ModelLoader::load_from_bytes`.

/// Read raw bytes for an asset by path (e.g. `"models/Cactus/cactus.fbx"`).
/// The path is relative to the `3d/` asset root on both platforms.
pub fn load_3d_asset(path: &str) -> Vec<u8> {
    load_3d_asset_impl(path)
}

// ── Desktop ───────────────────────────────────────────────────────────────────

#[cfg(not(target_os = "android"))]
fn load_3d_asset_impl(path: &str) -> Vec<u8> {
    // GAME_ASSETS_DIR is set at compile time by game/build.rs to the absolute
    // path of the game crate's `assets/` directory.
    let assets_dir = env!("GAME_ASSETS_DIR");
    let full_path  = std::path::Path::new(assets_dir).join("3d").join(path);
    std::fs::read(&full_path)
        .unwrap_or_else(|e| panic!("Failed to read asset '{}': {e}", full_path.display()))
}

// ── Android ───────────────────────────────────────────────────────────────────

#[cfg(target_os = "android")]
fn load_3d_asset_impl(path: &str) -> Vec<u8> {
    use jni::{objects::{JObject, JValue}, JavaVM};

    let ctx = ndk_context::android_context();
    let vm  = unsafe { JavaVM::from_raw(ctx.vm().cast()) }.unwrap();
    let mut env = vm.attach_current_thread().unwrap();

    let activity      = unsafe { JObject::from_raw(ctx.context().cast()) };
    let asset_manager = env
        .call_method(activity, "getAssets", "()Landroid/content/res/AssetManager;", &[])
        .unwrap().l().unwrap();

    let path_jstr   = env.new_string(path).unwrap();
    let input_stream = env
        .call_method(
            asset_manager, "open",
            "(Ljava/lang/String;)Ljava/io/InputStream;",
            &[JValue::Object(&path_jstr)],
        )
        .unwrap().l().unwrap();

    let available = env
        .call_method(&input_stream, "available", "()I", &[])
        .unwrap().i().unwrap() as usize;

    let byte_array = env.new_byte_array(available as i32).unwrap();
    let bytes_read = env
        .call_method(&input_stream, "read", "([B)I", &[JValue::Object(&byte_array)])
        .unwrap().i().unwrap();

    let mut buf_i8 = vec![0i8; bytes_read as usize];
    env.get_byte_array_region(&byte_array, 0, &mut buf_i8[..]).unwrap();
    let _ = env.call_method(&input_stream, "close", "()V", &[]);

    buf_i8.into_iter().map(|b| b as u8).collect()
}
