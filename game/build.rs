use cfg_aliases::cfg_aliases;

fn main() {
    // Register the same platform cfg aliases as engine/build.rs so that
    // game code and examples can use `android_platform`, `free_unix`, etc.
    cfg_aliases! {
        android_platform: { target_os = "android" },
        wasm_platform:    { target_family = "wasm" },
        macos_platform:   { target_os = "macos" },
        ios_platform:     { target_os = "ios" },
        apple:            { any(ios_platform, macos_platform) },
        free_unix:        { all(unix, not(apple), not(android_platform)) },
    }

    // Expose the game crate's assets directory as a compile-time env var.
    // Used by asset_loader.rs on desktop to find 3D model files regardless
    // of which directory `cargo desktop` is invoked from.
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-env=GAME_ASSETS_DIR={manifest_dir}/assets");
    println!("cargo:rerun-if-env-changed=CARGO_MANIFEST_DIR");
}
