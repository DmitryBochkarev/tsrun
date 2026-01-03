# Makefile for tsrun C embedding examples
#
# Prerequisites:
#   - Rust toolchain (cargo)
#   - C compiler (gcc or clang)
#
# Build the library first:
#   cd ../.. && cargo build --release --features c-api
#
# Then build examples:
#   make all

# Configuration
CC = gcc
CFLAGS = -Wall -Wextra -std=c11 -g -O2
LDFLAGS = -L../../target/release -ltsrun -lpthread -ldl -lm

# On macOS, use different flags
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
    LDFLAGS = -L../../target/release -ltsrun -lpthread -lm
endif

# Include path
INCLUDES = -I.

# Examples
EXAMPLES = basic native_functions module_loading async_orders

# Targets
.PHONY: all clean lib check

all: check $(EXAMPLES)

# Build the Rust library first
lib:
	cd ../.. && cargo build --release --features c-api

# Check if library exists
check:
	@if [ ! -f ../../target/release/libtsrun.so ] && [ ! -f ../../target/release/libtsrun.dylib ] && [ ! -f ../../target/release/libtsrun.a ]; then \
		echo "Error: libtsrun not found. Run 'make lib' or 'cargo build --release --features c-api' first."; \
		exit 1; \
	fi

# Build each example
basic: basic.c tsrun.h
	$(CC) $(CFLAGS) $(INCLUDES) -o $@ $< $(LDFLAGS)

native_functions: native_functions.c tsrun.h
	$(CC) $(CFLAGS) $(INCLUDES) -o $@ $< $(LDFLAGS)

module_loading: module_loading.c tsrun.h
	$(CC) $(CFLAGS) $(INCLUDES) -o $@ $< $(LDFLAGS)

async_orders: async_orders.c tsrun.h
	$(CC) $(CFLAGS) $(INCLUDES) -o $@ $< $(LDFLAGS)

# Run examples (need to set library path)
run-basic: basic
	LD_LIBRARY_PATH=../../target/release ./basic

run-native: native_functions
	LD_LIBRARY_PATH=../../target/release ./native_functions

run-modules: module_loading
	LD_LIBRARY_PATH=../../target/release ./module_loading

run-async: async_orders
	LD_LIBRARY_PATH=../../target/release ./async_orders

run-all: $(EXAMPLES)
	@echo "=== Running basic ===" && LD_LIBRARY_PATH=../../target/release ./basic
	@echo ""
	@echo "=== Running native_functions ===" && LD_LIBRARY_PATH=../../target/release ./native_functions
	@echo ""
	@echo "=== Running module_loading ===" && LD_LIBRARY_PATH=../../target/release ./module_loading
	@echo ""
	@echo "=== Running async_orders ===" && LD_LIBRARY_PATH=../../target/release ./async_orders

clean:
	rm -f $(EXAMPLES)

# For static linking (no LD_LIBRARY_PATH needed)
static: LDFLAGS = ../../target/release/libtsrun.a -lpthread -ldl -lm
static: clean $(EXAMPLES)
	@echo "Built with static linking"

# Help
help:
	@echo "tsrun C embedding examples"
	@echo ""
	@echo "Targets:"
	@echo "  make lib          - Build the Rust library"
	@echo "  make all          - Build all examples"
	@echo "  make <example>    - Build specific example"
	@echo "  make run-<example>- Run specific example"
	@echo "  make run-all      - Run all examples"
	@echo "  make static       - Build with static linking"
	@echo "  make clean        - Remove built examples"
	@echo ""
	@echo "Examples: $(EXAMPLES)"
