#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")/.."

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target-ios}"

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    echo "Usage: $0 [--release]"
    echo "  Builds visible-bridge for iOS (device + simulator). Debug by default."
    exit 0
fi

CARGO_PROFILE="debug"
CARGO_FLAGS=""
if [[ "${1:-}" == "--release" ]]; then
    CARGO_PROFILE="release"
    CARGO_FLAGS="--release"
fi

if command -v sccache &> /dev/null; then
    export RUSTC_WRAPPER=sccache
fi

rustup target add aarch64-apple-ios aarch64-apple-ios-sim 2>/dev/null || true

export IPHONEOS_DEPLOYMENT_TARGET=17.0

DEVICE_SDK="$(xcrun --sdk iphoneos --show-sdk-path)"
SIM_SDK="$(xcrun --sdk iphonesimulator --show-sdk-path)"

# coven bundles its own sqlite (libsqlite3-sys with `bundled`), but the crate's
# build script still runs bindgen against the system sqlite3.h, and bindgen needs
# the target SDK's sysroot to resolve the C standard headers when cross-compiling.
# Without -isysroot it fails ("could not run bindgen on header sqlite3/sqlite3.h").
echo "Building for iOS device (arm64, $CARGO_PROFILE)..."
BINDGEN_EXTRA_CLANG_ARGS="--target=arm64-apple-ios17.0 -isysroot $DEVICE_SDK" \
cargo build $CARGO_FLAGS --target aarch64-apple-ios -p visible-bridge

echo "Building for iOS simulator (arm64, $CARGO_PROFILE)..."
BINDGEN_EXTRA_CLANG_ARGS="--target=arm64-apple-ios17.0-simulator -isysroot $SIM_SDK" \
SDKROOT="$SIM_SDK" \
cargo build $CARGO_FLAGS --target aarch64-apple-ios-sim -p visible-bridge

echo "Generating Swift bindings..."
mkdir -p visible-bridge/swift-bindings
cargo run --bin uniffi-bindgen generate \
    --library "$CARGO_TARGET_DIR/aarch64-apple-ios/$CARGO_PROFILE/libvisible_bridge.a" \
    --language swift \
    --out-dir visible-bridge/swift-bindings/

echo "Copying Swift bindings into the iOS app source tree..."
mkdir -p visible-ios/visible/visible
cp visible-bridge/swift-bindings/visible_bridge.swift visible-ios/visible/visible/visible_bridge.swift

echo "Assembling the headers dir..."
mkdir -p visible-bridge/swift-bindings/headers
cp visible-bridge/swift-bindings/visible_bridgeFFI.h visible-bridge/swift-bindings/headers/
cp visible-bridge/swift-bindings/visible_bridgeFFI.modulemap visible-bridge/swift-bindings/headers/module.modulemap

echo "Creating iOS XCFramework..."
mkdir -p visible-ios
rm -rf visible-ios/VisibleBridgeFFI-ios.xcframework
xcodebuild -create-xcframework \
    -library "$CARGO_TARGET_DIR/aarch64-apple-ios/$CARGO_PROFILE/libvisible_bridge.a" \
    -headers visible-bridge/swift-bindings/headers \
    -library "$CARGO_TARGET_DIR/aarch64-apple-ios-sim/$CARGO_PROFILE/libvisible_bridge.a" \
    -headers visible-bridge/swift-bindings/headers \
    -output visible-ios/VisibleBridgeFFI-ios.xcframework

echo ""
echo "Done ($CARGO_PROFILE). Outputs:"
echo "  visible-ios/VisibleBridgeFFI-ios.xcframework/"
echo "  visible-bridge/swift-bindings/visible_bridge.swift"
echo "  visible-ios/visible/visible/visible_bridge.swift"
