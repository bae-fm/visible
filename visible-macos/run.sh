#!/usr/bin/env bash
set -euo pipefail

SKIP_RUST=false
RELEASE=false
OPEN=true

for arg in "$@"; do
    case "$arg" in
        --skip-rust) SKIP_RUST=true ;;
        --release) RELEASE=true ;;
        --no-open) OPEN=false ;;
        -h|--help)
            echo "Usage: $0 [--skip-rust] [--release] [--no-open]"
            echo "  Builds (and optionally runs) the macOS app."
            echo "  --skip-rust  Skip the Rust bridge build"
            echo "  --release    Build Rust in release mode, Swift in Release config"
            echo "  --no-open    Build only, don't launch the app"
            exit 0
            ;;
        *) echo "Unknown flag: $arg"; exit 1 ;;
    esac
done

cd "$(dirname "$0")/.."

# build-macos.sh copies the generated Swift bindings into the macOS app source
# tree itself, so there's no separate copy step here.
if [[ "$SKIP_RUST" == false ]]; then
    if [[ "$RELEASE" == true ]]; then
        ./visible-bridge/build-macos.sh --release
    else
        ./visible-bridge/build-macos.sh
    fi
fi

if [[ "$RELEASE" == true ]]; then
    CONFIG=Release
else
    CONFIG=Debug
fi

cd visible-macos/visible && xcodegen generate && cd ../..
# -allowProvisioningUpdates lets automatic signing create/refresh the managed
# provisioning profile (carrying the keychain-access-group + sandbox entitlements)
# the first time, so a fresh checkout builds without opening Xcode by hand.
xcodebuild -project visible-macos/visible/visible.xcodeproj -scheme visible -configuration "$CONFIG" -derivedDataPath build -allowProvisioningUpdates build

if [[ "$OPEN" == true ]]; then
    open "build/Build/Products/$CONFIG/visible.app"
fi
