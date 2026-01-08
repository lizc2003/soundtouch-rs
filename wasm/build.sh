#!/bin/bash
# Build script for SoundTouch WASM

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Building SoundTouch WASM...${NC}"

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo -e "${RED}Error: wasm-pack is not installed${NC}"
    exit 1
fi

# Default target
TARGET=${1:-web}

echo -e "${YELLOW}Target: $TARGET${NC}"

# Build directory
PKG_DIR="wasm/pkg-$TARGET"


# With aggressive size optimization
# wasm-pack build --release --target web --features wasm --out-dir wasm/pkg -- --no-default-features -Z build-std=std,panic_abort -Z build-std-features=panic_immediate_abort

# Build command
echo -e "${GREEN}Building release version...${NC}"
wasm-pack build --release --target $TARGET --features "default,wasm"
rm -rf $PKG_DIR
mv pkg $PKG_DIR

if [ "$TARGET" = "web" ]; then
    cat wasm/polyfill.js $PKG_DIR/soundtouch.js > temp && mv temp $PKG_DIR/soundtouch.js
elif [ "$TARGET" = "bundler" ]; then
    cat wasm/polyfill.js $PKG_DIR/soundtouch_bg.js > temp && mv temp $PKG_DIR/soundtouch_bg.js
    sed -i -e 's/"soundtouch-rs"/"soundtouch"/g' $PKG_DIR/package.json
    rm -f $PKG_DIR/package.json-e
    cd wasm
    rm -f soundtouch.tgz
    tar zcvf soundtouch.tgz pkg-bundler
    cd ..
fi

echo -e "${GREEN}Build complete! Output: $PKG_DIR${NC}"

# Show package size
if [ -f "$PKG_DIR/soundtouch_bg.wasm" ]; then
    SIZE=$(du -h "$PKG_DIR/soundtouch_bg.wasm" | cut -f1)
    echo -e "${YELLOW}WASM binary size: $SIZE${NC}"
fi

echo ""
echo -e "${GREEN}Usage examples:${NC}"
case $TARGET in
    web)
        echo "  In your HTML:"
        echo "  <script type=\"module\">"
        echo "    import init, { SoundTouchWasm } from './$PKG_DIR/soundtouch.js';"
        echo "    await init();"
        echo "    const st = new SoundTouchWasm(44100, 2);"
        echo "  </script>"
        ;;
    nodejs)
        echo "  In your Node.js code:"
        echo "  const { SoundTouchWasm } = require('./$PKG_DIR/soundtouch.js');"
        echo "  const st = new SoundTouchWasm(44100, 2);"
        ;;
    bundler)
        echo "  In your bundled app:"
        echo "  import { SoundTouchWasm } from './$PKG_DIR/soundtouch.js';"
        echo "  const st = new SoundTouchWasm(44100, 2);"
        ;;
esac
