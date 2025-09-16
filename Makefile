# Formosaic Build System
# Usage:
#   make            - Build and run on Linux (default)
#   make android    - Build and install APK for Android
#   make clean      - Clean build artifacts

# Paths
ANDROID_DIR := $(CURDIR)/.android
SDK_DIR := $(ANDROID_DIR)/sdk
CMDLINE_TOOLS := $(SDK_DIR)/cmdline-tools/latest
PLATFORM := "platforms;android-33"
BUILD_TOOLS := "build-tools;33.0.2"
NDK := "ndk;25.2.9519653"
NDK_PATH := "ndk/25.2.9519653"
NDK_HOME := $(SDK_DIR)/$(NDK_PATH)
NDK_SYSROOT := $(NDK_HOME)/toolchains/llvm/prebuilt/linux-x86_64/sysroot
XBUILD := ~/.cargo/bin/x

export CFLAGS := -w
export CXXFLAGS := -w
export BINDGEN_EXTRA_CLANG_ARGS := \
  --sysroot=$(NDK_SYSROOT) \
  -I$(NDK_SYSROOT)/usr/include \
  -I$(NDK_SYSROOT)/usr/include/aarch64-linux-android

# Env for all Android commands
ANDROID_ENV := ANDROID_HOME=$(SDK_DIR) ANDROID_NDK_HOME=$(SDK_DIR)/ndk/25.2.9519653 ANDROID_NDK_ROOT=$(SDK_DIR)/ndk/25.2.9519653 PATH=$(SDK_DIR)/platform-tools:$(CMDLINE_TOOLS)/bin:$(PATH)

.PHONY: linux run clean android android-debug android-example android-release android-all setup-android check-android help

# Development run with debug logging
linux run debug:
	@echo "Running with debug logging..."
	RUST_LOG=debug cargo run --example desktop

# Release build for Linux
release:
	@echo "Building release for Linux..."
	cargo run --example desktop --release

# Android setup (self-contained)
setup-android:
	@echo "📦 Setting up local Android SDK/NDK in $(ANDROID_DIR)..."
	@mkdir -p $(SDK_DIR)
	@if [ ! -d "$(CMDLINE_TOOLS)" ]; then \
		echo "⬇️  Downloading Android commandline-tools..."; \
		curl -L -o /tmp/cmdline-tools.zip https://dl.google.com/android/repository/commandlinetools-linux-9477386_latest.zip; \
		unzip -q /tmp/cmdline-tools.zip -d $(SDK_DIR)/cmdline-tools; \
		mv $(SDK_DIR)/cmdline-tools/cmdline-tools $(CMDLINE_TOOLS); \
		rm /tmp/cmdline-tools.zip; \
	fi
	@echo "✅ Commandline-tools installed."

	@echo "📦 Installing Android SDK components..."
	@yes | $(ANDROID_ENV) sdkmanager --sdk_root=$(SDK_DIR) $(PLATFORM) $(BUILD_TOOLS) $(NDK) "platform-tools"

	@echo "📦 Installing Rust Android targets..."
	rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android

	@echo "📦 Installing xbuild..."
	cargo install cargo-apk

	@echo "✅ Android setup complete!"
	@echo "All tools are inside $(ANDROID_DIR)."

android-debug:
	@echo "Building debug APK for Android using cargo-apk..."
	$(ANDROID_ENV) cargo apk r --example android

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	cargo clean
	@echo "Clean complete!"

# Check if Android environment is set up
check-android:
	@echo "Checking Android environment..."
	@if [ -d "$(SDK_DIR)" ]; then \
		echo "✅ ANDROID_HOME: $(SDK_DIR)"; \
	else \
		echo "❌ Android SDK not found in $(SDK_DIR)"; \
	fi
	@if [ -x "$(SDK_DIR)/platform-tools/adb" ]; then \
		echo "✅ ADB found: $(SDK_DIR)/platform-tools/adb"; \
		$(SDK_DIR)/platform-tools/adb devices; \
	else \
		echo "❌ ADB not installed. Run: make setup-android"; \
	fi
	@if $(XBUILD) --help > /dev/null 2>&1; then \
		echo "✅ xbuild installed"; \
	else \
		echo "❌ xbuild not installed"; \
		echo "   Run: make setup-android"; \
	fi



# Help message
help:
	@echo "Formosaic Build System"
	@echo ""
	@echo "Common commands:"
	@echo "  make            - Build and run on Linux (default)"
	@echo "  make debug      - Run with debug logging on Linux"
	@echo "  make release    - Build and run release version on Linux"
	@echo "  make android    - Build and install debug APK for Android"
	@echo "  make android-release - Build and install release APK for Android"
	@echo ""
	@echo "Setup commands:"
	@echo "  make setup-android - Install local Android SDK/NDK into .android/"
	@echo "  make check-android - Check Android environment setup"
	@echo ""
	@echo "Utility commands:"
	@echo "  make clean      - Clean all build artifacts"
	@echo "  make help       - Show this help message"

