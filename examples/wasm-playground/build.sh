#!/bin/bash
# Build the tsrun WASM module for the playground
#
# Uses C-style FFI exports for unified API across all runtimes
# (browser, Go/wazero, Rust/wasmtime, etc.)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
WASM_TARGET="wasm32-unknown-unknown"
WASM_OUTPUT="$PROJECT_DIR/target/$WASM_TARGET/release/tsrun.wasm"

echo "Building tsrun WASM module..."
echo "Project directory: $PROJECT_DIR"

# Build the WASM module
cd "$PROJECT_DIR"
cargo build --release \
    --target "$WASM_TARGET" \
    --features wasm \
    --no-default-features

# Ensure pkg directory exists
mkdir -p "$SCRIPT_DIR/pkg"

# Copy WASM file
cp "$WASM_OUTPUT" "$SCRIPT_DIR/pkg/tsrun.wasm"

echo ""
echo "Build complete!"
echo "WASM output: $SCRIPT_DIR/pkg/tsrun.wasm"
echo "Size: $(ls -lh "$SCRIPT_DIR/pkg/tsrun.wasm" | awk '{print $5}')"

# Copy to site directory
SITE_PLAYGROUND_DIR="$PROJECT_DIR/site/playground"
if [[ -d "$SITE_PLAYGROUND_DIR" ]]; then
    echo ""
    echo "Copying to site: $SITE_PLAYGROUND_DIR"
    mkdir -p "$SITE_PLAYGROUND_DIR/pkg"
    cp "$SCRIPT_DIR/pkg/tsrun.wasm" "$SITE_PLAYGROUND_DIR/pkg/"
    cp "$SCRIPT_DIR/pkg/tsrun.js" "$SITE_PLAYGROUND_DIR/pkg/"
    cp "$SCRIPT_DIR/main.js" "$SITE_PLAYGROUND_DIR/"
    # Note: index.html is NOT copied - site has its own version with site navigation
fi

# Run tests if --test flag is provided
if [[ "$1" == "--test" ]]; then
    echo ""
    echo "Running e2e tests..."

    cd "$SCRIPT_DIR"

    # Install puppeteer if not present
    if [[ ! -d "node_modules/puppeteer" ]]; then
        npm install
    fi

    # Run tests
    npm run test:browser
fi

echo ""
echo "To serve the playground locally:"
echo "  cd $SCRIPT_DIR"
echo "  python3 -m http.server 8080"
echo "  # Then open http://localhost:8080"
echo ""
echo "To run tests:"
echo "  $0 --test"
