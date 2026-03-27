//! xtask — project automation for Formosaic.
//!
//! Run via cargo aliases defined in .cargo/config.toml:
//!
//!   cargo generate-icons          Generate Android mipmap PNGs from assets/icons/*.png
//!   cargo setup-android           Download Android SDK/NDK, install Rust targets
//!   cargo check-android           Verify the Android toolchain is ready
//!   cargo android                 Build + install + run debug APK
//!   cargo android-release         Build + install + run release APK
//!   cargo build-android           Build debug APK only
//!   cargo build-android-release   Build release APK only
//!   cargo clean-all               Remove all build artefacts

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{self, Command},
};

fn main() {
    let task = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: cargo <task>");
        eprintln!("Tasks: generate-icons");
        eprintln!("       setup-android  check-android  android  android-release");
        eprintln!("       build-android  build-android-release  clean-all");
        process::exit(1);
    });

    let workspace = workspace_root();

    match task.as_str() {
        "generate-icons"        => generate_icons(&workspace),
        "setup-android"         => setup_android(&workspace),
        "check-android"         => check_android(&workspace),
        "android"               => android_run(&workspace, false),
        "android-release"       => android_run(&workspace, true),
        "build-android"         => android_build(&workspace, false),
        "build-android-release" => android_build(&workspace, true),
        "clean-all"             => clean_all(&workspace),
        other => {
            eprintln!("Unknown task: {other}");
            process::exit(1);
        }
    }
}

// ── Paths ─────────────────────────────────────────────────────────────────────

fn workspace_root() -> PathBuf {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent().unwrap().to_owned()
}

fn sdk_dir(workspace: &Path)   -> PathBuf { workspace.join(".android/sdk") }
fn ndk_home(workspace: &Path)  -> PathBuf { sdk_dir(workspace).join("ndk").join(NDK_VERSION) }
fn cmdline_tools(workspace: &Path) -> PathBuf {
    sdk_dir(workspace).join("cmdline-tools/latest")
}

const NDK_VERSION:       &str = "25.2.9519653";
const CMDLINE_TOOLS_URL: &str =
    "https://dl.google.com/android/repository/commandlinetools-linux-9477386_latest.zip";

// ── Icon generation ───────────────────────────────────────────────────────────

/// Generate all Android mipmap icon sizes from assets/icons/{fg,bg}.png using
/// ImageMagick (magick).  Source PNGs live at assets/icons/ in the workspace
/// root; generated files go into game/assets/res/mipmap-*/
fn generate_icons(workspace: &Path) {
    if !command_exists("magick") {
        eprintln!("❌ ImageMagick (magick) is required — install it first.");
        process::exit(1);
    }

    let icons_dir = workspace.join("assets/icons");
    let fg_src    = icons_dir.join("ic_launcher_foreground.png");
    let bg_src    = icons_dir.join("ic_launcher_background.png");

    for src in [&fg_src, &bg_src] {
        if !src.exists() {
            eprintln!("❌ Missing source icon: {}", src.display());
            eprintln!("   Place ic_launcher_foreground.png and ic_launcher_background.png");
            eprintln!("   inside assets/icons/ in the workspace root.");
            process::exit(1);
        }
    }

    let res_dir = workspace.join("game/assets/res");

    // Remove old mipmap dirs so stale sizes never linger
    if let Ok(entries) = fs::read_dir(&res_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("mipmap-") && name != "mipmap-anydpi-v26" {
                fs::remove_dir_all(entry.path()).ok();
            }
        }
    }

    const SIZES: &[(&str, u32)] = &[
        ("mipmap-mdpi",    48),
        ("mipmap-hdpi",    72),
        ("mipmap-xhdpi",   96),
        ("mipmap-xxhdpi",  144),
        ("mipmap-xxxhdpi", 192),
    ];

    println!("🚀 Generating Android icons from assets/icons/…");

    for (dir, size) in SIZES {
        let out_dir = res_dir.join(dir);
        fs::create_dir_all(&out_dir).unwrap();
        println!("   → {dir} ({size}×{size})");

        let sz = format!("{size}x{size}");
        let fg_out = out_dir.join("ic_launcher_foreground.png");
        let bg_out = out_dir.join("ic_launcher_background.png");
        let ic_out = out_dir.join("ic_launcher.png");

        // Foreground — preserve transparency
        run_cmd(Command::new("magick").args([
            fg_src.to_str().unwrap(),
            "-resize", &sz, "-background", "none",
            "-gravity", "center", "-extent", &sz, "-alpha", "on",
            fg_out.to_str().unwrap(),
        ]));

        // Background
        run_cmd(Command::new("magick").args([
            bg_src.to_str().unwrap(),
            "-resize", &sz,
            bg_out.to_str().unwrap(),
        ]));

        // Legacy flattened icon
        run_cmd(Command::new("magick").args([
            bg_src.to_str().unwrap(), fg_src.to_str().unwrap(),
            "-resize", &sz, "-gravity", "center",
            "-compose", "over", "-composite",
            ic_out.to_str().unwrap(),
        ]));
    }

    // Adaptive icon XML
    let adaptive_dir = res_dir.join("mipmap-anydpi-v26");
    fs::create_dir_all(&adaptive_dir).unwrap();
    fs::write(
        adaptive_dir.join("ic_launcher.xml"),
        "<adaptive-icon xmlns:android=\"http://schemas.android.com/apk/res/android\">\n\
         \x20   <background android:drawable=\"@mipmap/ic_launcher_background\"/>\n\
         \x20   <foreground android:drawable=\"@mipmap/ic_launcher_foreground\"/>\n\
         </adaptive-icon>\n",
    ).unwrap();

    println!("✅ Icons generated successfully.");
}

// ── Android build helpers ─────────────────────────────────────────────────────

fn bindgen_args(ndk: &Path, abi_triple: &str) -> String {
    let sysroot = ndk.join("toolchains/llvm/prebuilt/linux-x86_64/sysroot");
    format!(
        "--sysroot={} -I{}/usr/include -I{}/usr/include/{}",
        sysroot.display(), sysroot.display(), sysroot.display(), abi_triple,
    )
}

fn cargo_apk(workspace: &Path, apk_args: &[&str]) {
    let ndk      = ndk_home(workspace);
    let sdk      = sdk_dir(workspace);
    let toolchain = ndk.join("toolchains/llvm/prebuilt/linux-x86_64/bin");

    let bindgen_aarch64 = bindgen_args(&ndk, "aarch64-linux-android");
    let bindgen_armv7   = bindgen_args(&ndk, "arm-linux-androideabi");

    let mut cmd = Command::new("cargo");
    cmd.arg("apk").args(apk_args).current_dir(workspace);
    cmd.env("ANDROID_HOME",     &sdk);
    cmd.env("ANDROID_NDK_HOME", &ndk);
    cmd.env("ANDROID_NDK_ROOT", &ndk);

    let orig_path = env::var("PATH").unwrap_or_default();
    cmd.env("PATH", format!(
        "{}:{}:{}",
        toolchain.display(),
        sdk.join("platform-tools").display(),
        orig_path,
    ));

    cmd.env("CFLAGS_aarch64_linux_android",   "-w");
    cmd.env("CXXFLAGS_aarch64_linux_android", "-w");
    cmd.env("BINDGEN_EXTRA_CLANG_ARGS_aarch64_linux_android",   &bindgen_aarch64);
    cmd.env("BINDGEN_EXTRA_CLANG_ARGS_armv7_linux_androideabi", &bindgen_armv7);

    let status = cmd.status().unwrap_or_else(|e| {
        eprintln!("Failed to run cargo apk: {e}");
        process::exit(1);
    });
    if !status.success() { process::exit(status.code().unwrap_or(1)); }
}

fn android_run(workspace: &Path, release: bool) {
    let mut args = vec!["run", "-p", "formosaic", "--example", "android"];
    if release { args.push("--release"); }
    cargo_apk(workspace, &args);
}

fn android_build(workspace: &Path, release: bool) {
    let mut args = vec!["build", "-p", "formosaic", "--example", "android"];
    if release { args.push("--release"); }
    cargo_apk(workspace, &args);
}

// ── Setup / check ─────────────────────────────────────────────────────────────

fn setup_android(workspace: &Path) {
    let sdk   = sdk_dir(workspace);
    let tools = cmdline_tools(workspace);
    let ndk   = ndk_home(workspace);

    if !tools.exists() {
        println!("⬇  Downloading Android commandline-tools…");
        fs::create_dir_all(sdk.join("cmdline-tools")).unwrap();
        let zip = "/tmp/cmdline-tools.zip";
        run(&["curl", "-L", "-o", zip, CMDLINE_TOOLS_URL]);
        run(&["unzip", "-q", zip, "-d", sdk.join("cmdline-tools").to_str().unwrap()]);
        fs::rename(sdk.join("cmdline-tools/cmdline-tools"), &tools)
            .expect("failed to rename cmdline-tools");
        fs::remove_file(zip).ok();
    }

    println!("📦 Installing SDK components…");
    let sdkmanager = tools.join("bin/sdkmanager");
    run_with_stdin(
        &[
            sdkmanager.to_str().unwrap(),
            &format!("--sdk_root={}", sdk.display()),
            "platforms;android-33", "build-tools;33.0.2",
            &format!("ndk;{NDK_VERSION}"), "platform-tools",
        ],
        &[("ANDROID_HOME", sdk.to_str().unwrap())],
        "y\ny\ny\ny\ny\ny\ny\ny\ny\ny\ny\ny\ny\ny\ny\ny\ny\ny\ny\ny\n",
    );

    println!("🦀 Installing Rust Android targets…");
    run(&[
        "rustup", "target", "add",
        "aarch64-linux-android", "armv7-linux-androideabi",
        "x86_64-linux-android",  "i686-linux-android",
    ]);

    if !command_exists("cargo-apk") {
        println!("📦 Installing cargo-apk…");
        run(&["cargo", "install", "cargo-apk"]);
    } else {
        println!("✅ cargo-apk already installed");
    }

    println!("\n✅ Android setup complete.");
    println!("   SDK: {}", sdk.display());
    println!("   NDK: {}", ndk.display());
}

fn check_android(workspace: &Path) {
    println!("── Android environment check ──");
    let sdk = sdk_dir(workspace);
    let ndk = ndk_home(workspace);
    let adb = sdk.join("platform-tools/adb");
    print_check("SDK",       sdk.exists(),            &sdk.display().to_string());
    print_check("NDK",       ndk.exists(),            &ndk.display().to_string());
    print_check("ADB",       adb.exists(),            &adb.display().to_string());
    print_check("cargo-apk", command_exists("cargo-apk"), "cargo-apk");
    if adb.exists() {
        println!("\nConnected devices:");
        Command::new(&adb).arg("devices").status().ok();
    }
}

fn clean_all(workspace: &Path) {
    println!("🧹 Cleaning…");
    let status = Command::new("cargo").arg("clean").current_dir(workspace).status().unwrap();
    if !status.success() { process::exit(status.code().unwrap_or(1)); }
    println!("✅ Done");
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn run(args: &[&str]) {
    let status = Command::new(args[0]).args(&args[1..])
        .status().unwrap_or_else(|e| panic!("failed to run {:?}: {e}", args[0]));
    if !status.success() {
        eprintln!("Command failed: {}", args.join(" "));
        process::exit(status.code().unwrap_or(1));
    }
}

fn run_cmd(cmd: &mut Command) {
    let status = cmd.status().unwrap_or_else(|e| {
        eprintln!("Failed to run command: {e}");
        process::exit(1);
    });
    if !status.success() { process::exit(status.code().unwrap_or(1)); }
}

fn run_with_stdin(args: &[&str], envs: &[(&str, &str)], stdin_data: &str) {
    use std::io::Write;
    let mut cmd = Command::new(args[0]);
    cmd.args(&args[1..]).stdin(process::Stdio::piped());
    for (k, v) in envs { cmd.env(k, v); }
    let mut child = cmd.spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {:?}: {e}", args[0]));
    if let Some(mut s) = child.stdin.take() {
        s.write_all(stdin_data.as_bytes()).ok();
    }
    let status = child.wait().unwrap();
    if !status.success() {
        eprintln!("Command failed: {}", args.join(" "));
        process::exit(status.code().unwrap_or(1));
    }
}

fn command_exists(name: &str) -> bool {
    Command::new("which").arg(name).output()
        .map(|o| o.status.success()).unwrap_or(false)
}

fn print_check(label: &str, ok: bool, path: &str) {
    if ok { println!("  ✅  {label}: {path}"); }
    else  { println!("  ❌  {label} not found — run: cargo setup-android"); }
}
