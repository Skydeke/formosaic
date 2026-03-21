# Formosaic

**Formosaic** (from *Form + Mosaic*) is a puzzle game written in Rust and OpenGL ES. Your goal is to find the correct viewing angle that assembles scattered low-poly fragments into a complete 3D model.

---

## Project Structure

```
formosaic/
  engine/     formosaic-engine crate — generic rendering engine, zero game knowledge
  game/       formosaic crate — game logic, platform hosting, puzzles
  xtask/      cargo xtask — project automation (Android setup, etc.)
```

Dependency direction is strictly one-way: `game → engine`. The compiler enforces this.

---

## Building & Running

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- On Linux: `pkg-config libssl-dev build-essential cmake clang libclang-dev`

### Desktop (Linux)

```sh
# Debug
cargo desktop

# Release
cargo desktop-release
```

### Android

First, set up the local Android SDK/NDK (one-time, downloads into `.android/`):

```sh
cargo setup-android
```

Verify the toolchain is ready:

```sh
cargo check-android
```

Then, with a device connected via USB (USB debugging enabled):

```sh
# Debug build + install + run
cargo android

# Release build + install + run
cargo android-release
```

### Build only (no run)

```sh
cargo build-desktop           # desktop debug
cargo build-desktop-release   # desktop release
cargo build-android           # Android debug APK
cargo build-android-release   # Android release APK
```

### Clean

```sh
cargo clean-all
```

---

## CI

GitHub Actions and GitLab CI pipelines build both targets on every push. Release APKs are signed using a keystore provided via CI secrets (`ANDROID_KEYSTORE_B64`).
