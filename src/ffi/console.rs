//! Console provider for C FFI.

extern crate alloc;

use core::ffi::{c_char, c_void};

use crate::platform::{ConsoleLevel, ConsoleProvider};

use super::{ConsoleCallbackWrapper, TsRunConsoleFn, TsRunConsoleLevel, TsRunContext, TsRunResult};

// ============================================================================
// FFI Console Provider
// ============================================================================

/// Console provider that delegates to a C callback.
///
/// This provider looks up the callback from `TsRunContext.console_callback`
/// via the `ffi_context` pointer stored in the interpreter during step/run.
///
/// If no callback is set (console_callback is None), output is discarded.
pub struct FfiConsoleProvider {
    /// Raw pointer to TsRunContext. Only valid during step/run execution.
    ctx_ptr: *mut c_void,
}

impl FfiConsoleProvider {
    /// Create a new FFI console provider.
    ///
    /// The ctx_ptr must point to a valid TsRunContext during all callback invocations.
    pub fn new(ctx_ptr: *mut c_void) -> Self {
        Self { ctx_ptr }
    }

    fn get_callback(&self) -> Option<&ConsoleCallbackWrapper> {
        if self.ctx_ptr.is_null() {
            return None;
        }
        // SAFETY: ctx_ptr is set by tsrun_step/tsrun_run and points to a valid TsRunContext
        let ctx = unsafe { &*(self.ctx_ptr as *const TsRunContext) };
        ctx.console_callback.as_ref()
    }
}

impl ConsoleProvider for FfiConsoleProvider {
    fn write(&self, level: ConsoleLevel, message: &str) {
        let Some(wrapper) = self.get_callback() else {
            return; // No callback set, discard output
        };

        let c_level = match level {
            ConsoleLevel::Log => TsRunConsoleLevel::Log,
            ConsoleLevel::Info => TsRunConsoleLevel::Info,
            ConsoleLevel::Debug => TsRunConsoleLevel::Debug,
            ConsoleLevel::Warn => TsRunConsoleLevel::Warn,
            ConsoleLevel::Error => TsRunConsoleLevel::Error,
        };

        (wrapper.callback)(
            c_level,
            message.as_ptr() as *const c_char,
            message.len(),
            wrapper.userdata,
        );
    }

    fn clear(&self) {
        let Some(wrapper) = self.get_callback() else {
            return;
        };

        // Call with Clear level and empty message
        (wrapper.callback)(TsRunConsoleLevel::Clear, c"".as_ptr(), 0, wrapper.userdata);
    }
}

// ============================================================================
// C API
// ============================================================================

/// Set a custom console provider callback.
///
/// If func is NULL, console output is discarded (no-op).
/// The callback is invoked synchronously during step/run.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_set_console(
    ctx: *mut TsRunContext,
    func: Option<TsRunConsoleFn>,
    userdata: *mut c_void,
) -> TsRunResult {
    let ctx = match unsafe { ctx.as_mut() } {
        Some(c) => c,
        None => {
            return TsRunResult {
                ok: false,
                error: c"NULL context".as_ptr(),
            };
        }
    };

    ctx.clear_error();

    // Update the console callback
    ctx.console_callback = func.map(|callback| ConsoleCallbackWrapper { callback, userdata });

    TsRunResult::success()
}
