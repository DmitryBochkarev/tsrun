#!/bin/bash
# Local fuzzing helper script
# Usage: ./fuzz/scripts/fuzz.sh [target] [duration_seconds]
#
# Examples:
#   ./fuzz/scripts/fuzz.sh              # Run all targets for 60s each
#   ./fuzz/scripts/fuzz.sh lexer        # Run lexer fuzzer for 60s
#   ./fuzz/scripts/fuzz.sh interpreter 300  # Run interpreter for 5 minutes

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$PROJECT_ROOT"

TARGET="${1:-all}"
DURATION="${2:-60}"

# Extract corpus if not already done
if [ ! -d "fuzz/corpus/interpreter" ] || [ -z "$(ls -A fuzz/corpus/interpreter 2>/dev/null)" ]; then
    echo "Extracting initial corpus..."
    python3 fuzz/scripts/extract_corpus.py
fi

run_target() {
    local target=$1
    echo ""
    echo "=========================================="
    echo "Running $target for ${DURATION}s..."
    echo "=========================================="
    cargo +nightly fuzz run "$target" -- \
        -max_total_time="$DURATION" \
        -max_len=10000 \
        -print_final_stats=1
}

case "$TARGET" in
    lexer|fuzz_lexer)
        run_target fuzz_lexer
        ;;
    parser|fuzz_parser)
        run_target fuzz_parser
        ;;
    interpreter|fuzz_interpreter)
        run_target fuzz_interpreter
        ;;
    all)
        run_target fuzz_lexer
        run_target fuzz_parser
        run_target fuzz_interpreter
        ;;
    list)
        cargo +nightly fuzz list
        ;;
    *)
        echo "Unknown target: $TARGET"
        echo ""
        echo "Usage: $0 [target] [duration_seconds]"
        echo ""
        echo "Targets:"
        echo "  lexer|fuzz_lexer       - Fuzz the lexer"
        echo "  parser|fuzz_parser     - Fuzz the parser"
        echo "  interpreter|fuzz_interpreter - Fuzz the full interpreter"
        echo "  all                    - Run all targets (default)"
        echo "  list                   - List available fuzz targets"
        echo ""
        echo "Examples:"
        echo "  $0                     # All targets, 60s each"
        echo "  $0 lexer 30            # Lexer only, 30s"
        echo "  $0 interpreter 300     # Interpreter, 5 minutes"
        exit 1
        ;;
esac

echo ""
echo "Fuzzing complete!"
