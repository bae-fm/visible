#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")/.."

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target-macos}"

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    echo "Usage: $0 [--release]"
    echo "  Builds visible-bridge for macOS (arm64, native host). Debug by default."
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

rustup target add aarch64-apple-darwin 2>/dev/null || true

# Match the app's deployment target so the staticlib's object files aren't
# stamped with the host SDK version (which makes ld warn "built for newer macOS
# version than being linked"). Keep in sync with the app's MACOSX_DEPLOYMENT_TARGET.
export MACOSX_DEPLOYMENT_TARGET=14.0

# Native arm64 build on the arm64 host: bindgen finds the host SDK on its own,
# so no -isysroot override is needed (unlike the cross-compiled iOS build).
echo "Building for macOS (arm64, $CARGO_PROFILE)..."
cargo build $CARGO_FLAGS --target aarch64-apple-darwin -p visible-bridge

echo "Generating Swift bindings..."
mkdir -p visible-bridge/swift-bindings
cargo run --bin uniffi-bindgen generate \
    --library "$CARGO_TARGET_DIR/aarch64-apple-darwin/$CARGO_PROFILE/libvisible_bridge.a" \
    --language swift \
    --out-dir visible-bridge/swift-bindings/

echo "Copying Swift bindings into the macOS app source tree..."
mkdir -p visible-macos/visible/visible
cp visible-bridge/swift-bindings/visible_bridge.swift visible-macos/visible/visible/visible_bridge.swift

echo "Assembling the headers dir..."
mkdir -p visible-bridge/swift-bindings/headers
cp visible-bridge/swift-bindings/visible_bridgeFFI.h visible-bridge/swift-bindings/headers/
cp visible-bridge/swift-bindings/visible_bridgeFFI.modulemap visible-bridge/swift-bindings/headers/module.modulemap

echo "Creating macOS XCFramework..."
mkdir -p visible-macos
rm -rf visible-macos/VisibleBridgeFFI.xcframework
xcodebuild -create-xcframework \
    -library "$CARGO_TARGET_DIR/aarch64-apple-darwin/$CARGO_PROFILE/libvisible_bridge.a" \
    -headers visible-bridge/swift-bindings/headers \
    -output visible-macos/VisibleBridgeFFI.xcframework

echo ""
echo "Done ($CARGO_PROFILE). Outputs:"
echo "  visible-macos/VisibleBridgeFFI.xcframework/"
echo "  visible-bridge/swift-bindings/visible_bridge.swift"
echo "  visible-macos/visible/visible/visible_bridge.swift"
