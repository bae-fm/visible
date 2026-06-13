#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target-android}"

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    echo "Usage: $0 [--release]"
    echo "  Builds visible-bridge for Android (arm64 + x86_64). Debug by default."
    exit 0
fi

CARGO_PROFILE="debug"
CARGO_FLAGS=""
if [[ "${1:-}" == "--release" ]]; then
    CARGO_PROFILE="release"
    CARGO_FLAGS="--release"
fi

NDK_HOME="${ANDROID_NDK_HOME:-/Users/dima/Library/Android/sdk/ndk/29.0.14206865}"
TOOLCHAIN="$NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64"

rustup target add aarch64-linux-android x86_64-linux-android 2>/dev/null || true

export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$TOOLCHAIN/bin/aarch64-linux-android35-clang"
export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$TOOLCHAIN/bin/x86_64-linux-android35-clang"

export CC_aarch64_linux_android="$TOOLCHAIN/bin/aarch64-linux-android35-clang"
export AR_aarch64_linux_android="$TOOLCHAIN/bin/llvm-ar"
export CC_x86_64_linux_android="$TOOLCHAIN/bin/x86_64-linux-android35-clang"
export AR_x86_64_linux_android="$TOOLCHAIN/bin/llvm-ar"

echo "Building for Android arm64 ($CARGO_PROFILE)..."
RUSTC_WRAPPER="" cargo build $CARGO_FLAGS --target aarch64-linux-android -p visible-bridge

echo "Building for Android x86_64 ($CARGO_PROFILE)..."
RUSTC_WRAPPER="" cargo build $CARGO_FLAGS --target x86_64-linux-android -p visible-bridge

# Generate bindings from the actual Android library, not a host build, so the
# Kotlin API matches the .so exactly. uniffi-bindgen reads metadata statically
# from the cross-arch object, so it doesn't load it; the bindgen binary itself
# still builds for the host via `cargo run`.
echo "Generating Kotlin bindings..."
mkdir -p visible-bridge/kotlin-bindings
cargo run --bin uniffi-bindgen generate \
    --library "$CARGO_TARGET_DIR/aarch64-linux-android/$CARGO_PROFILE/libvisible_bridge.so" \
    --language kotlin \
    --out-dir visible-bridge/kotlin-bindings/ \
    --no-format

echo "Copying .so files to Android project..."
mkdir -p visible-android/app/src/main/jniLibs/arm64-v8a
mkdir -p visible-android/app/src/main/jniLibs/x86_64
cp "$CARGO_TARGET_DIR/aarch64-linux-android/$CARGO_PROFILE/libvisible_bridge.so" visible-android/app/src/main/jniLibs/arm64-v8a/
cp "$CARGO_TARGET_DIR/x86_64-linux-android/$CARGO_PROFILE/libvisible_bridge.so" visible-android/app/src/main/jniLibs/x86_64/

echo ""
echo "Done ($CARGO_PROFILE)."
echo "Outputs:"
echo "  visible-android/app/src/main/jniLibs/arm64-v8a/libvisible_bridge.so"
echo "  visible-android/app/src/main/jniLibs/x86_64/libvisible_bridge.so"
echo "  visible-bridge/kotlin-bindings/"
