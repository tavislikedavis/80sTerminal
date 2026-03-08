#!/bin/bash
set -e

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
APP_NAME="80sTerminal"
BUNDLE_DIR="$PROJECT_DIR/target/release/${APP_NAME}.app"

echo "Building 80sTerminal in release mode..."
cargo build --release --manifest-path "$PROJECT_DIR/Cargo.toml"

echo "Creating app bundle..."
rm -rf "$BUNDLE_DIR"
mkdir -p "$BUNDLE_DIR/Contents/MacOS"
mkdir -p "$BUNDLE_DIR/Contents/Resources"

# Copy binary
cp "$PROJECT_DIR/target/release/80sterminal" "$BUNDLE_DIR/Contents/MacOS/80sterminal"

# Copy Info.plist
cp "$PROJECT_DIR/assets/Info.plist" "$BUNDLE_DIR/Contents/Info.plist"

# Copy icon if it exists
if [ -f "$PROJECT_DIR/assets/80sTerminal.icns" ]; then
    cp "$PROJECT_DIR/assets/80sTerminal.icns" "$BUNDLE_DIR/Contents/Resources/80sTerminal.icns"
    echo "Icon included."
else
    echo "Warning: assets/80sTerminal.icns not found. Run 'python3 scripts/make_icon.py' first."
fi

echo ""
echo "App bundle created at:"
echo "  $BUNDLE_DIR"
echo ""
echo "To install, run:"
echo "  cp -r \"$BUNDLE_DIR\" /Applications/"
echo ""
echo "Or drag 80sTerminal.app from target/release/ into your Applications folder."
