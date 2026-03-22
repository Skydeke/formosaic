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

    // Poly Pizza API key — baked in at compile time so it works on Android too.
    //
    // Priority order:
    //   1. POLY_PIZZA_API_KEY environment variable (CI / shell export)
    //   2. .env file in the workspace root (local dev, never commit this file)
    //
    // The key is embedded in the binary via env!() — never written to disk or
    // committed to source.  Add `.env` to your .gitignore.
    let api_key = std::env::var("POLY_PIZZA_API_KEY").unwrap_or_else(|_| {
        // Walk up from CARGO_MANIFEST_DIR to find a .env file.
        // Handles both `cargo build` from game/ and from workspace root.
        let manifest = std::path::PathBuf::from(
            std::env::var("CARGO_MANIFEST_DIR").unwrap()
        );
        let candidates = [
            manifest.join(".env"),
            manifest.parent().map(|p| p.join(".env")).unwrap_or_default(),
        ];
        for path in &candidates {
            if let Ok(contents) = std::fs::read_to_string(path) {
                for line in contents.lines() {
                    let line = line.trim();
                    if line.starts_with('#') || line.is_empty() { continue; }
                    if let Some(val) = line.strip_prefix("POLY_PIZZA_API_KEY=") {
                        return val.trim_matches('"').trim_matches('\'').to_string();
                    }
                }
            }
        }
        String::new()
    });
    println!("cargo:rustc-env=POLY_PIZZA_API_KEY={api_key}");
    println!("cargo:rerun-if-env-changed=POLY_PIZZA_API_KEY");
    // Re-run if .env changes
    println!("cargo:rerun-if-changed=.env");
    println!("cargo:rerun-if-changed=../.env");
}
