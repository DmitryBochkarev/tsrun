//! Raw WASM exports for non-browser runtimes (wazero, wasmer, wasmtime).
//!
//! This module provides a WASM-friendly API that exposes the C FFI functions
//! without wasm-bindgen dependencies. The host runtime (Go, etc.) provides
//! platform services through imported functions.
//!
//! # Host Import Contract
//!
//! The host must provide these functions in the `tsrun_host` module:
//!
//! - `host_time_now() -> i64` - Current Unix timestamp in milliseconds
//! - `host_time_start_timer() -> u64` - Start a performance timer
//! - `host_time_elapsed(start: u64) -> u64` - Elapsed milliseconds since timer start
//! - `host_random() -> f64` - Random float in [0, 1)
//! - `host_console_write(level: u32, ptr: u32, len: u32)` - Write console message
//! - `host_console_clear()` - Clear console
//!
//! Console levels: 0=log, 1=info, 2=debug, 3=warn, 4=error
//!
//! # Memory Management
//!
//! Strings are passed by pointer + length. The host can allocate WASM memory via:
//! - `tsrun_alloc(size: u32) -> u32` - Allocate memory
//! - `tsrun_dealloc(ptr: u32, size: u32)` - Free memory
//!
//! # Example Usage (Go with wazero)
//!
//! ```go
//! // Load WASM module
//! module, _ := runtime.Instantiate(ctx, wasmBytes)
//!
//! // Allocate string in WASM memory
//! code := "console.log('hello')"
//! ptr := module.ExportedFunction("tsrun_alloc").Call(ctx, len(code))
//! memory.Write(ptr, []byte(code))
//!
//! // Create context and execute
//! ctxPtr := module.ExportedFunction("tsrun_wasm_new").Call(ctx)
//! module.ExportedFunction("tsrun_prepare").Call(ctx, ctxPtr, ptr, 0)
//! result := module.ExportedFunction("tsrun_run").Call(ctx, ctxPtr)
//! ```

extern crate alloc;

use alloc::boxed::Box;
use core::alloc::Layout;
use core::ffi::c_void;

use crate::platform::{ConsoleLevel, ConsoleProvider, RandomProvider, TimeProvider};

use crate::ffi::{TsRunContext, console::FfiConsoleProvider};

// ============================================================================
// Global Allocator and Panic Handler
// ============================================================================

// Use dlmalloc as the global allocator for WASM builds.
use dlmalloc::GlobalDlmalloc;

#[global_allocator]
static ALLOCATOR: GlobalDlmalloc = GlobalDlmalloc;

/// Panic handler for no_std WASM builds.
/// Aborts execution - the host can detect this via wasm trap.
#[cfg(not(feature = "std"))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

// ============================================================================
// Host Imports
// ============================================================================

#[link(wasm_import_module = "tsrun_host")]
unsafe extern "C" {
    /// Get current time as milliseconds since Unix epoch.
    fn host_time_now() -> i64;

    /// Start a timer and return an opaque handle.
    fn host_time_start_timer() -> u64;

    /// Get elapsed milliseconds since the timer was started.
    fn host_time_elapsed(start: u64) -> u64;

    /// Generate a random f64 in the range [0, 1).
    fn host_random() -> f64;

    /// Write a console message.
    /// level: 0=log, 1=info, 2=debug, 3=warn, 4=error
    fn host_console_write(level: u32, ptr: *const u8, len: u32);

    /// Clear the console.
    fn host_console_clear();
}

// ============================================================================
// Platform Providers
// ============================================================================

/// Time provider that delegates to host imports.
pub struct WasmRawTimeProvider;

impl TimeProvider for WasmRawTimeProvider {
    fn now_millis(&self) -> i64 {
        unsafe { host_time_now() }
    }

    fn start_timer(&self) -> u64 {
        unsafe { host_time_start_timer() }
    }

    fn elapsed_millis(&self, start: u64) -> u64 {
        unsafe { host_time_elapsed(start) }
    }
}

/// Random provider that delegates to host imports.
pub struct WasmRawRandomProvider;

impl RandomProvider for WasmRawRandomProvider {
    fn random(&mut self) -> f64 {
        unsafe { host_random() }
    }
}

/// Console provider that delegates to host imports.
pub struct WasmRawConsoleProvider;

impl ConsoleProvider for WasmRawConsoleProvider {
    fn write(&self, level: ConsoleLevel, message: &str) {
        let level_num = match level {
            ConsoleLevel::Log => 0,
            ConsoleLevel::Info => 1,
            ConsoleLevel::Debug => 2,
            ConsoleLevel::Warn => 3,
            ConsoleLevel::Error => 4,
        };
        unsafe {
            host_console_write(level_num, message.as_ptr(), message.len() as u32);
        }
    }

    fn clear(&self) {
        unsafe {
            host_console_clear();
        }
    }
}

// ============================================================================
// Memory Allocator Exports
// ============================================================================

/// Allocate memory in WASM linear memory.
///
/// Returns a pointer to the allocated memory, or 0 on failure.
/// The caller is responsible for calling `tsrun_dealloc` to free the memory.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_alloc(size: u32) -> u32 {
    if size == 0 {
        return 0;
    }
    // Use 8-byte alignment for struct safety (u64 fields need this)
    let Ok(layout) = Layout::from_size_align(size as usize, 8) else {
        return 0;
    };
    let ptr = unsafe { alloc::alloc::alloc(layout) };
    if ptr.is_null() { 0 } else { ptr as u32 }
}

/// Free memory allocated by `tsrun_alloc`.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_dealloc(ptr: u32, size: u32) {
    if ptr == 0 || size == 0 {
        return;
    }
    // Must match the alignment used in tsrun_alloc
    let Ok(layout) = Layout::from_size_align(size as usize, 8) else {
        return;
    };
    unsafe {
        alloc::alloc::dealloc(ptr as *mut u8, layout);
    }
}

// ============================================================================
// WASM-Specific Context Creation
// ============================================================================

/// Create a new interpreter context with WASM host platform providers.
///
/// This is similar to `tsrun_new()` but installs providers that delegate
/// to host-provided import functions instead of std library functions.
///
/// Returns a pointer to the context, or NULL on failure.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_wasm_new() -> *mut TsRunContext {
    let ctx = Box::new(TsRunContext::new());
    let ctx_ptr = Box::into_raw(ctx);

    // Get a mutable reference to set up providers
    let ctx_ref = unsafe { &mut *ctx_ptr };

    // Install WASM platform providers
    ctx_ref
        .interp
        .set_time_provider(Box::new(WasmRawTimeProvider));
    ctx_ref
        .interp
        .set_random_provider(Box::new(WasmRawRandomProvider));

    // For console, we can either use the raw provider or the FFI callback system.
    // Using the raw provider directly for simplicity in WASM context.
    ctx_ref.interp.set_console(Box::new(WasmRawConsoleProvider));

    ctx_ptr
}

/// Alternative: Create context that uses FFI console callback system.
///
/// This allows the host to set a console callback via `tsrun_set_console_callback`,
/// which may be preferred for some use cases.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_wasm_new_with_callback_console() -> *mut TsRunContext {
    let ctx = Box::new(TsRunContext::new());
    let ctx_ptr = Box::into_raw(ctx);

    let ctx_ref = unsafe { &mut *ctx_ptr };

    // Install WASM platform providers for time and random
    ctx_ref
        .interp
        .set_time_provider(Box::new(WasmRawTimeProvider));
    ctx_ref
        .interp
        .set_random_provider(Box::new(WasmRawRandomProvider));

    // Use FFI console provider that looks up callback from context
    let provider = FfiConsoleProvider::new(ctx_ptr as *mut c_void);
    ctx_ref.interp.set_console(Box::new(provider));

    ctx_ptr
}
