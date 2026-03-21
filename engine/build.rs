use std::env;
use std::fs::File;
use std::path::PathBuf;

use cfg_aliases::cfg_aliases;
use gl_generator::{Api, Fallbacks, Profile, Registry, StructGenerator};

fn main() {
    // Setup cfg aliases (taken from glutin/build.rs).
    cfg_aliases! {
        android_platform: { target_os = "android" },
        wasm_platform: { target_family = "wasm" },
        macos_platform: { target_os = "macos" },
        ios_platform: { target_os = "ios" },
        apple: { any(ios_platform, macos_platform) },
        free_unix: { all(unix, not(apple), not(android_platform)) },

        x11_platform: { all(feature = "x11", free_unix, not(wasm_platform)) },
        wayland_platform: { all(feature = "wayland", free_unix, not(wasm_platform)) },

        egl_backend: { all(feature = "egl", any(windows, unix), not(apple), not(wasm_platform)) },
        glx_backend: { all(feature = "glx", x11_platform, not(wasm_platform)) },
        wgl_backend: { all(feature = "wgl", windows, not(wasm_platform)) },
        cgl_backend: { all(macos_platform, not(wasm_platform)) },
    }

    let dest = PathBuf::from(&env::var("OUT_DIR").unwrap());

    println!("cargo:rerun-if-changed=build.rs");

    let mut file = File::create(dest.join("gl_bindings.rs")).unwrap();
    Registry::new(Api::Gles2, (3, 0), Profile::Core, Fallbacks::All, [])
        .write_bindings(StructGenerator, &mut file)
        .unwrap();

    let target = env::var("TARGET").unwrap();

    if target.contains("android") {
        // Link against the NDK shared C++ runtime.
        println!("cargo:rustc-link-lib=c++_shared");

        // Tell bindgen (used by russimp-ng's sys crate) where the NDK sysroot
        // headers are.  We derive the path from ANDROID_NDK_ROOT, which
        // cargo-apk sets automatically; fall back to ANDROID_NDK_HOME.
        // This keeps the desktop build completely unaffected.
        let ndk_root = env::var("ANDROID_NDK_ROOT")
            .or_else(|_| env::var("ANDROID_NDK_HOME"))
            .expect(
                "Android NDK not found. Set ANDROID_NDK_ROOT or run `make setup-android`.",
            );

        let sysroot = format!(
            "{}/toolchains/llvm/prebuilt/linux-x86_64/sysroot",
            ndk_root
        );

        // Determine the ABI-specific include dir from TARGET.
        let abi_include = if target.starts_with("aarch64") {
            "aarch64-linux-android"
        } else if target.starts_with("armv7") {
            "arm-linux-androideabi"
        } else if target.starts_with("i686") {
            "i686-linux-android"
        } else {
            "x86_64-linux-android"
        };

        let clang_args = format!(
            "--sysroot={sysroot} -I{sysroot}/usr/include -I{sysroot}/usr/include/{abi_include}"
        );

        // cargo propagates BINDGEN_EXTRA_CLANG_ARGS to build scripts of deps.
        println!("cargo:rustc-env=BINDGEN_EXTRA_CLANG_ARGS={clang_args}");

        println!("cargo:rerun-if-env-changed=ANDROID_NDK_ROOT");
        println!("cargo:rerun-if-env-changed=ANDROID_NDK_HOME");
    }
}
