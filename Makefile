# ─────────────────────────────────────────────────────────────────────────────
# Formosaic — unified build system for Linux desktop and Android
#
#   make                  Build and run (debug) on Linux desktop
#   make release          Build and run (release) on Linux desktop
#   make build            Build desktop debug without running
#   make build-release    Build desktop release without running
#
#   make android          Build + install + run debug APK on device
#   make android-release  Build + install + run release APK on device
#   make android-build    Build debug APK only (no install / run)
#   make android-release-build  Build release APK only
#
#   make setup-android    Download SDK / NDK into .android/ (one-time)
#   make check-android    Verify Android toolchain is installed
#   make clean            Remove all build artefacts
#   make help             Print this message
# ─────────────────────────────────────────────────────────────────────────────

# ── Local Android SDK/NDK (self-contained, never touches the system) ──────────
ANDROID_DIR      := $(CURDIR)/.android
SDK_DIR          := $(ANDROID_DIR)/sdk
CMDLINE_TOOLS    := $(SDK_DIR)/cmdline-tools/latest
NDK_VERSION      := 25.2.9519653
NDK_HOME         := $(SDK_DIR)/ndk/$(NDK_VERSION)

# cargo-apk needs these three vars; we pass them only to Android targets.
ANDROID_ENV := \
  ANDROID_HOME=$(SDK_DIR) \
  ANDROID_NDK_HOME=$(NDK_HOME) \
  ANDROID_NDK_ROOT=$(NDK_HOME) \
  PATH=$(SDK_DIR)/platform-tools:$(CMDLINE_TOOLS)/bin:$(PATH)

# Suppress warnings from vendored C code inside assimp / russimp-ng.
# Applied only where needed (not globally exported).
QUIET_C := CFLAGS="-w" CXXFLAGS="-w"

DESKTOP_EXAMPLE := desktop
ANDROID_EXAMPLE := android

# ── Desktop targets ───────────────────────────────────────────────────────────

.PHONY: run debug
run debug:           ## Build and run desktop debug (default)
	@echo "▶  Running desktop (debug)…"
	RUST_LOG=debug cargo run --example $(DESKTOP_EXAMPLE)

.PHONY: release
release:             ## Build and run desktop release
	@echo "▶  Running desktop (release)…"
	cargo run --example $(DESKTOP_EXAMPLE) --release

.PHONY: build
build:               ## Build desktop debug (no run)
	@echo "🔨 Building desktop debug…"
	cargo build --example $(DESKTOP_EXAMPLE)
	@echo "✅  target/debug/examples/$(DESKTOP_EXAMPLE)"

.PHONY: build-release
build-release:       ## Build desktop release (no run)
	@echo "🔨 Building desktop release…"
	cargo build --example $(DESKTOP_EXAMPLE) --release
	@echo "✅  target/release/examples/$(DESKTOP_EXAMPLE)"

# Default target
.DEFAULT_GOAL := run

# ── Android targets ───────────────────────────────────────────────────────────

.PHONY: android
android:             ## Build + install + run debug APK
	@echo "📱 Building and running Android debug APK…"
	$(ANDROID_ENV) $(QUIET_C) cargo apk r --example $(ANDROID_EXAMPLE)

.PHONY: android-release
android-release:     ## Build + install + run release APK
	@echo "📱 Building and running Android release APK…"
	$(ANDROID_ENV) $(QUIET_C) cargo apk run --release --example $(ANDROID_EXAMPLE)

.PHONY: android-build
android-build:       ## Build debug APK only (no install)
	@echo "🔨 Building Android debug APK…"
	$(ANDROID_ENV) $(QUIET_C) cargo apk build --example $(ANDROID_EXAMPLE)
	@echo "✅  target/debug/apk/"

.PHONY: android-release-build
android-release-build: ## Build release APK only (no install)
	@echo "🔨 Building Android release APK…"
	$(ANDROID_ENV) $(QUIET_C) cargo apk build --release --example $(ANDROID_EXAMPLE)
	@echo "✅  target/release/apk/"

# ── Android setup (one-time) ──────────────────────────────────────────────────

.PHONY: setup-android
setup-android:       ## Download Android SDK/NDK into .android/ (run once)
	@echo "📦 Setting up local Android SDK/NDK in $(ANDROID_DIR)…"
	@mkdir -p $(SDK_DIR)
	@if [ ! -d "$(CMDLINE_TOOLS)" ]; then \
		echo "⬇  Downloading Android commandline-tools…"; \
		curl -L -o /tmp/cmdline-tools.zip \
			https://dl.google.com/android/repository/commandlinetools-linux-9477386_latest.zip; \
		unzip -q /tmp/cmdline-tools.zip -d $(SDK_DIR)/cmdline-tools; \
		mv $(SDK_DIR)/cmdline-tools/cmdline-tools $(CMDLINE_TOOLS); \
		rm /tmp/cmdline-tools.zip; \
	fi
	@echo "📦 Installing Android SDK components…"
	@yes | $(ANDROID_ENV) sdkmanager --sdk_root=$(SDK_DIR) \
		"platforms;android-33" \
		"build-tools;33.0.2" \
		"ndk;$(NDK_VERSION)" \
		"platform-tools"
	@echo "📦 Installing Rust Android targets…"
	rustup target add aarch64-linux-android armv7-linux-androideabi \
		x86_64-linux-android i686-linux-android
	@echo "📦 Installing cargo-apk…"
	cargo install cargo-apk
	@echo "✅ Android setup complete — tools are in $(ANDROID_DIR)"

.PHONY: check-android
check-android:       ## Check that the Android toolchain is set up
	@echo "── Android environment check ──"
	@if [ -d "$(SDK_DIR)" ]; then \
		echo "✅ SDK:  $(SDK_DIR)"; \
	else \
		echo "❌ SDK not found — run: make setup-android"; \
	fi
	@if [ -d "$(NDK_HOME)" ]; then \
		echo "✅ NDK:  $(NDK_HOME)"; \
	else \
		echo "❌ NDK not found — run: make setup-android"; \
	fi
	@if [ -x "$(SDK_DIR)/platform-tools/adb" ]; then \
		echo "✅ ADB:  $(SDK_DIR)/platform-tools/adb"; \
		$(SDK_DIR)/platform-tools/adb devices; \
	else \
		echo "❌ ADB not installed — run: make setup-android"; \
	fi
	@if cargo apk --help > /dev/null 2>&1; then \
		echo "✅ cargo-apk installed"; \
	else \
		echo "❌ cargo-apk missing — run: make setup-android"; \
	fi

# ── Misc ──────────────────────────────────────────────────────────────────────

.PHONY: fix-warnings
fix-warnings:
	$(ANDROID_ENV) cargo fix --lib -p formosaic

.PHONY: clean
clean:               ## Remove all build artefacts
	@echo "🧹 Cleaning…"
	cargo clean
	@echo "✅ Clean complete"

.PHONY: help
help:                ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-24s\033[0m %s\n", $$1, $$2}'
