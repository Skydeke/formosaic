# Formosaic

**Formosaic** (from *Form + Mosaic*) is a simple OpenGL ES game written in Rust, designed to help you learn Rust and OpenGL while having fun. In the game, your goal is to find the correct viewing angle to assemble low-poly artwork fragments into a complete image.

---

## Features

- Cross-platform: runs on **Linux** and **Android**
- Minimal OpenGL ES 2.0/3.0 rendering pipeline
- Input handling via **SDL2** for keyboard, mouse, and touch
- Clean and extendable Rust project structure

---

## Project Structure

- `src/` – Rust source files
- `shaders/` – Placeholder for OpenGL shaders
- `.android/` – Local Android SDK/NDK installation (created by `make setup-android`)
- `Makefile` – Build system for Linux and Android targets

---

## Build & Run

The project uses a **Makefile** for simplified builds and running.

### Linux

```bash
# Build and run the desktop version
make run

# Build and run with debug logging
make debug

# Build and run release version
make release
```

### Android

> Requires a device with USB debugging enabled

```bash
# Setup Android SDK/NDK and Rust targets
make setup-android

# Check that the Android environment is correctly installed
make check-android

# Build and install debug APK on connected device
make android-debug

# Build and install release APK
make android-release
```

### Clean Build Artifacts

```bash
make clean
```

---

## Notes

- The **Formosaic** project is meant as a learning tool. The current version renders a simple scene (triangle or placeholder) and demonstrates input handling, window creation, and OpenGL ES setup.
- Future improvements may include:
  - Loading and displaying 3D fragment pieces
  - Interactive rotation via mouse or touch
  - Detecting when the puzzle is correctly assembled

---

## Help

For a full list of Makefile commands:

```bash
make help
```

This will display all targets, including setup, Android builds, Linux builds, and cleaning commands.

---

**Enjoy exploring Formosaic and learning Rust with OpenGL ES!**

