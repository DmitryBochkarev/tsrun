# Makefile for tsrun C embedding examples
#
# Prerequisites:
#   - Rust toolchain (cargo)
#   - C compiler (gcc or clang)
#   - PCRE2 library (optional, for regexp example)
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

# PCRE2 support (for regexp example)
PCRE2_CFLAGS := $(shell pkg-config --cflags libpcre2-8 2>/dev/null)
PCRE2_LDFLAGS := $(shell pkg-config --libs libpcre2-8 2>/dev/null)
HAS_PCRE2 := $(if $(PCRE2_LDFLAGS),yes,no)

# Examples (regexp only built if PCRE2 is available)
EXAMPLES = basic native_functions module_loading async_orders internal_modules event_system
ifeq ($(HAS_PCRE2),yes)
    EXAMPLES += regexp
endif

# Targets
.PHONY: all clean lib check

all: check $(EXAMPLES)

# Build the Rust library first
lib:
	cd ../.. && cargo build --release --features c-api

# Check if library exists and show PCRE2 status
check:
	@if [ ! -f ../../target/release/libtsrun.so ] && [ ! -f ../../target/release/libtsrun.dylib ] && [ ! -f ../../target/release/libtsrun.a ]; then \
		echo "Error: libtsrun not found. Run 'make lib' or 'cargo build --release --features c-api' first."; \
		exit 1; \
	fi
ifeq ($(HAS_PCRE2),no)
	@echo "Note: PCRE2 not found. The 'regexp' example will be skipped."
	@echo "      Install PCRE2: apt install libpcre2-dev (Debian/Ubuntu)"
	@echo "                     brew install pcre2 (macOS)"
endif

# Console helper object (linked with all examples)
tsrun_console.o: tsrun_console.c tsrun_console.h tsrun.h
	$(CC) $(CFLAGS) $(INCLUDES) -c -o $@ $<

# Build each example (all link with tsrun_console.o)
basic: basic.c tsrun.h tsrun_console.o
	$(CC) $(CFLAGS) $(INCLUDES) -o $@ $< tsrun_console.o $(LDFLAGS)

native_functions: native_functions.c tsrun.h tsrun_console.o
	$(CC) $(CFLAGS) $(INCLUDES) -o $@ $< tsrun_console.o $(LDFLAGS)

module_loading: module_loading.c tsrun.h tsrun_console.o
	$(CC) $(CFLAGS) $(INCLUDES) -o $@ $< tsrun_console.o $(LDFLAGS)

async_orders: async_orders.c tsrun.h tsrun_console.o
	$(CC) $(CFLAGS) $(INCLUDES) -o $@ $< tsrun_console.o $(LDFLAGS)

internal_modules: internal_modules.c tsrun.h tsrun_console.o
	$(CC) $(CFLAGS) $(INCLUDES) -o $@ $< tsrun_console.o $(LDFLAGS)

event_system: event_system.c tsrun.h tsrun_console.o
	$(CC) $(CFLAGS) $(INCLUDES) -o $@ $< tsrun_console.o $(LDFLAGS)

# RegExp provider object (requires PCRE2)
regexp_provider.o: regexp_provider.c regexp_provider.h tsrun.h
	$(CC) $(CFLAGS) $(INCLUDES) $(PCRE2_CFLAGS) -c -o $@ $<

# RegExp example (requires PCRE2)
regexp: regexp.c tsrun.h tsrun_console.o regexp_provider.o
	$(CC) $(CFLAGS) $(INCLUDES) $(PCRE2_CFLAGS) -o $@ $< tsrun_console.o regexp_provider.o $(LDFLAGS) $(PCRE2_LDFLAGS)

# Run examples (need to set library path)
run-basic: basic
	LD_LIBRARY_PATH=../../target/release ./basic

run-native: native_functions
	LD_LIBRARY_PATH=../../target/release ./native_functions

run-modules: module_loading
	LD_LIBRARY_PATH=../../target/release ./module_loading

run-async: async_orders
	LD_LIBRARY_PATH=../../target/release ./async_orders

run-internal: internal_modules
	LD_LIBRARY_PATH=../../target/release ./internal_modules

run-event: event_system
	LD_LIBRARY_PATH=../../target/release ./event_system

run-regexp: regexp
	LD_LIBRARY_PATH=../../target/release ./regexp

run-all: $(EXAMPLES)
	@echo "=== Running basic ===" && LD_LIBRARY_PATH=../../target/release ./basic
	@echo ""
	@echo "=== Running native_functions ===" && LD_LIBRARY_PATH=../../target/release ./native_functions
	@echo ""
	@echo "=== Running module_loading ===" && LD_LIBRARY_PATH=../../target/release ./module_loading
	@echo ""
	@echo "=== Running async_orders ===" && LD_LIBRARY_PATH=../../target/release ./async_orders
	@echo ""
	@echo "=== Running internal_modules ===" && LD_LIBRARY_PATH=../../target/release ./internal_modules
	@echo ""
	@echo "=== Running event_system ===" && LD_LIBRARY_PATH=../../target/release ./event_system
ifeq ($(HAS_PCRE2),yes)
	@echo ""
	@echo "=== Running regexp ===" && LD_LIBRARY_PATH=../../target/release ./regexp
endif

clean:
	rm -f $(EXAMPLES) tsrun_console.o regexp_provider.o regexp

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
	@echo ""
	@echo "PCRE2 status: $(HAS_PCRE2)"
ifeq ($(HAS_PCRE2),no)
	@echo "  The 'regexp' example requires PCRE2."
	@echo "  Install: apt install libpcre2-dev (Debian/Ubuntu)"
	@echo "           brew install pcre2 (macOS)"
endif
