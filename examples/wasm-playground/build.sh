#!/bin/bash
# Build the tsrun WASM module for the playground

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "Building tsrun WASM module..."
echo "Project directory: $PROJECT_DIR"

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    echo "Error: wasm-pack is not installed."
    echo "Install it with: cargo install wasm-pack"
    exit 1
fi

# Build the WASM module for web target
cd "$PROJECT_DIR"
wasm-pack build \
    --target web \
    --out-dir "$SCRIPT_DIR/pkg" \
    --no-default-features \
    --features wasm

echo ""
echo "Build complete!"
echo "Output: $SCRIPT_DIR/pkg/"

# Copy to site directory
SITE_PKG_DIR="$PROJECT_DIR/site/playground/pkg"
if [[ -d "$PROJECT_DIR/site/playground" ]]; then
    echo "Copying to site: $SITE_PKG_DIR"
    cp -r "$SCRIPT_DIR/pkg/"* "$SITE_PKG_DIR/"
fi
echo ""

# Run tests if --test flag is provided
if [[ "$1" == "--test" ]]; then
    echo "Running e2e tests..."
    echo ""

    # Build for Node.js target for testing
    wasm-pack build \
        --target nodejs \
        --out-dir "$SCRIPT_DIR/tests/pkg" \
        --no-default-features \
        --features wasm

    cd "$SCRIPT_DIR/tests"

    # Install puppeteer if not present
    if [[ ! -d "node_modules/puppeteer" ]]; then
        npm install
    fi

    # Run all tests
    npm run test:all
fi

echo ""
echo "To serve the playground locally:"
echo "  cd $SCRIPT_DIR"
echo "  python3 -m http.server 8080"
echo "  # Then open http://localhost:8080"
echo ""
echo "To run all tests (Node.js + browser):"
echo "  $0 --test"
