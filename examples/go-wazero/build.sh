#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
WASM_TARGET="wasm32-unknown-unknown"
WASM_OUTPUT="$PROJECT_ROOT/target/$WASM_TARGET/release/tsrun.wasm"

# Activate mise for Go if available
if command -v mise &> /dev/null; then
    eval "$(mise activate bash)"
fi

usage() {
    echo "Usage: $0 [--build|--test|--run|--all]"
    echo "  --build   Build WASM module only"
    echo "  --test    Run Go unit tests"
    echo "  --run     Run all examples"
    echo "  --all     Build, test, and run (default)"
}

build_wasm() {
    echo "Building WASM module..."
    cd "$PROJECT_ROOT"
    cargo build --release \
        --target "$WASM_TARGET" \
        --features wasm \
        --no-default-features

    cp "$WASM_OUTPUT" "$SCRIPT_DIR/tsrun/tsrun.wasm"
    echo "WASM module copied to $SCRIPT_DIR/tsrun/tsrun.wasm"
    echo "Size: $(ls -lh "$SCRIPT_DIR/tsrun/tsrun.wasm" | awk '{print $5}')"
}

run_tests() {
    echo "Running Go unit tests..."
    cd "$SCRIPT_DIR"
    go test -v ./tsrun/...
}

run_examples() {
    echo "Running examples..."
    cd "$SCRIPT_DIR"

    echo -e "\n=== Basic Example ==="
    go run ./basic

    echo -e "\n=== Modules Example ==="
    go run ./modules

    echo -e "\n=== Async Example ==="
    go run ./async

    echo -e "\n=== Native Functions Example ==="
    go run ./native
}

run_example() {
    local example="$1"
    echo "Running $example example..."
    cd "$SCRIPT_DIR"
    go run "./$example"
}

# Parse arguments
ACTION="${1:---all}"
case "$ACTION" in
    --build) build_wasm ;;
    --test)  build_wasm && run_tests ;;
    --run)   build_wasm && run_examples ;;
    --all)   build_wasm && run_examples ;;
    --help)  usage ;;
    --example)
        if [ -z "$2" ]; then
            echo "Usage: $0 --example <name>"
            echo "Available: basic, modules, async, native"
            exit 1
        fi
        build_wasm && run_example "$2"
        ;;
    *)       echo "Unknown option: $ACTION" && usage && exit 1 ;;
esac

echo -e "\nDone!"
