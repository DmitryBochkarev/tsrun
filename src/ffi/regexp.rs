//! C FFI for custom RegExp providers.
//!
//! This module allows C/C++ embedders to provide their own regex implementation
//! by registering callback functions.
//!
//! # Example
//!
//! ```c
//! // Define callbacks
//! static void* my_compile(void* userdata, const char* pattern, const char* flags,
//!                         const char** error_out) {
//!     // Compile pattern, return handle
//!     return my_regex_compile(pattern, flags);
//! }
//!
//! static int my_is_match(void* userdata, void* handle, const char* input,
//!                        size_t input_len, const char** error_out) {
//!     return my_regex_test(handle, input, input_len) ? 1 : 0;
//! }
//!
//! // ... other callbacks ...
//!
//! static void my_free(void* userdata, void* handle) {
//!     my_regex_free(handle);
//! }
//!
//! // Register provider
//! TsRunRegexCallbacks callbacks = {
//!     .compile = my_compile,
//!     .is_match = my_is_match,
//!     .find = my_find,
//!     .free = my_free,
//!     .userdata = NULL
//! };
//! tsrun_set_regexp_provider(ctx, &callbacks);
//! ```

extern crate alloc;

use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::ffi::{c_char, c_void};
use core::fmt;
use core::ptr;

use crate::platform::{CompiledRegex, RegExpProvider, RegexMatch};

use super::{TsRunContext, TsRunResult, c_str_to_str};

// ============================================================================
// C Callback Type Definitions
// ============================================================================

/// Compile a regex pattern.
///
/// # Parameters
/// - `userdata`: User-provided context pointer
/// - `pattern`: The regex pattern (UTF-8, null-terminated)
/// - `flags`: The regex flags string (e.g., "gi", null-terminated)
/// - `error_out`: On error, set to a static error message (valid until next call)
///
/// # Returns
/// - On success: An opaque handle to the compiled regex
/// - On error: NULL (and *error_out contains error message)
pub type TsRunRegexCompileFn = extern "C" fn(
    userdata: *mut c_void,
    pattern: *const c_char,
    flags: *const c_char,
    error_out: *mut *const c_char,
) -> *mut c_void;

/// Test if a regex matches the input string.
///
/// # Parameters
/// - `userdata`: User-provided context pointer
/// - `handle`: Compiled regex handle from `compile`
/// - `input`: The input string (UTF-8, not null-terminated)
/// - `input_len`: Length of input in bytes
/// - `error_out`: On error, set to a static error message
///
/// # Returns
/// - 1: Match found
/// - 0: No match
/// - -1: Error (check *error_out)
pub type TsRunRegexIsMatchFn = extern "C" fn(
    userdata: *mut c_void,
    handle: *mut c_void,
    input: *const c_char,
    input_len: usize,
    error_out: *mut *const c_char,
) -> i32;

/// Result from a regex find operation.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct TsRunRegexMatch {
    /// Byte offset where match starts
    pub start: usize,
    /// Byte offset where match ends (exclusive)
    pub end: usize,
    /// Array of capture groups (start, end pairs)
    /// NULL if no captures or not supported
    pub captures: *mut TsRunRegexCapture,
    /// Number of capture groups (including group 0 = full match)
    pub capture_count: usize,
}

/// A single capture group result.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TsRunRegexCapture {
    /// Start byte offset (-1 if group didn't participate)
    pub start: isize,
    /// End byte offset (-1 if group didn't participate)
    pub end: isize,
}

/// Find the first match starting at a given position.
///
/// # Parameters
/// - `userdata`: User-provided context pointer
/// - `handle`: Compiled regex handle
/// - `input`: The input string (UTF-8, not null-terminated)
/// - `input_len`: Length of input in bytes
/// - `start_pos`: Byte offset to start searching from
/// - `match_out`: Output match result (caller-allocated)
/// - `error_out`: On error, set to a static error message
///
/// # Returns
/// - 1: Match found (match_out populated)
/// - 0: No match
/// - -1: Error
pub type TsRunRegexFindFn = extern "C" fn(
    userdata: *mut c_void,
    handle: *mut c_void,
    input: *const c_char,
    input_len: usize,
    start_pos: usize,
    match_out: *mut TsRunRegexMatch,
    error_out: *mut *const c_char,
) -> i32;

/// Free a compiled regex handle.
///
/// # Parameters
/// - `userdata`: User-provided context pointer
/// - `handle`: Compiled regex handle to free
pub type TsRunRegexFreeFn = extern "C" fn(userdata: *mut c_void, handle: *mut c_void);

/// Free a match result's captures array.
///
/// # Parameters
/// - `userdata`: User-provided context pointer
/// - `captures`: Captures array to free
/// - `count`: Number of captures
pub type TsRunRegexFreeCapturesFn =
    extern "C" fn(userdata: *mut c_void, captures: *mut TsRunRegexCapture, count: usize);

// ============================================================================
// C Callback Bundle
// ============================================================================

/// Bundle of regex callbacks for the C FFI.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TsRunRegexCallbacks {
    /// Compile a regex pattern
    pub compile: TsRunRegexCompileFn,
    /// Test if regex matches
    pub is_match: TsRunRegexIsMatchFn,
    /// Find first match at position
    pub find: TsRunRegexFindFn,
    /// Free a compiled regex
    pub free: TsRunRegexFreeFn,
    /// Free captures array (may be NULL if not needed)
    pub free_captures: Option<TsRunRegexFreeCapturesFn>,
    /// User-provided context pointer passed to all callbacks
    pub userdata: *mut c_void,
}

// ============================================================================
// C RegExp Provider
// ============================================================================

/// RegExp provider that delegates to C callbacks.
pub struct CRegExpProvider {
    callbacks: TsRunRegexCallbacks,
}

impl CRegExpProvider {
    /// Create a new C-backed RegExp provider.
    ///
    /// # Safety
    /// The callbacks must remain valid for the lifetime of this provider.
    pub unsafe fn new(callbacks: TsRunRegexCallbacks) -> Self {
        Self { callbacks }
    }
}

impl RegExpProvider for CRegExpProvider {
    fn compile(&self, pattern: &str, flags: &str) -> Result<Rc<dyn CompiledRegex>, String> {
        // Convert to C strings
        let pattern_cstr = match alloc::ffi::CString::new(pattern) {
            Ok(s) => s,
            Err(_) => return Err(String::from("Pattern contains null bytes")),
        };
        let flags_cstr = match alloc::ffi::CString::new(flags) {
            Ok(s) => s,
            Err(_) => return Err(String::from("Flags contain null bytes")),
        };

        let mut error_out: *const c_char = ptr::null();

        let handle = (self.callbacks.compile)(
            self.callbacks.userdata,
            pattern_cstr.as_ptr(),
            flags_cstr.as_ptr(),
            &mut error_out,
        );

        if handle.is_null() {
            let error_msg = if error_out.is_null() {
                String::from("Regex compilation failed")
            } else {
                unsafe { c_str_to_str(error_out) }
                    .map(String::from)
                    .unwrap_or_else(|| String::from("Regex compilation failed"))
            };
            return Err(error_msg);
        }

        Ok(Rc::new(CCompiledRegex {
            handle: RefCell::new(handle),
            callbacks: CRegexCallbacksRef {
                is_match: self.callbacks.is_match,
                find: self.callbacks.find,
                free: self.callbacks.free,
                free_captures: self.callbacks.free_captures,
                userdata: self.callbacks.userdata,
            },
            flags: String::from(flags),
        }))
    }
}

/// Reference to callbacks needed by CCompiledRegex.
#[derive(Clone, Copy)]
struct CRegexCallbacksRef {
    is_match: TsRunRegexIsMatchFn,
    find: TsRunRegexFindFn,
    free: TsRunRegexFreeFn,
    free_captures: Option<TsRunRegexFreeCapturesFn>,
    userdata: *mut c_void,
}

/// Compiled regex backed by C callbacks.
struct CCompiledRegex {
    handle: RefCell<*mut c_void>,
    callbacks: CRegexCallbacksRef,
    flags: String,
}

impl fmt::Debug for CCompiledRegex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CCompiledRegex")
            .field("flags", &self.flags)
            .finish()
    }
}

impl Drop for CCompiledRegex {
    fn drop(&mut self) {
        let handle = *self.handle.borrow();
        if !handle.is_null() {
            (self.callbacks.free)(self.callbacks.userdata, handle);
        }
    }
}

impl CompiledRegex for CCompiledRegex {
    fn is_match(&self, input: &str) -> Result<bool, String> {
        let handle = *self.handle.borrow();
        let mut error_out: *const c_char = ptr::null();

        let result = (self.callbacks.is_match)(
            self.callbacks.userdata,
            handle,
            input.as_ptr() as *const c_char,
            input.len(),
            &mut error_out,
        );

        match result {
            1 => Ok(true),
            0 => Ok(false),
            _ => {
                let error_msg = if error_out.is_null() {
                    String::from("Regex match failed")
                } else {
                    unsafe { c_str_to_str(error_out) }
                        .map(String::from)
                        .unwrap_or_else(|| String::from("Regex match failed"))
                };
                Err(error_msg)
            }
        }
    }

    fn find(&self, input: &str, start_pos: usize) -> Result<Option<RegexMatch>, String> {
        let handle = *self.handle.borrow();
        let mut error_out: *const c_char = ptr::null();
        let mut match_out = TsRunRegexMatch {
            start: 0,
            end: 0,
            captures: ptr::null_mut(),
            capture_count: 0,
        };

        let result = (self.callbacks.find)(
            self.callbacks.userdata,
            handle,
            input.as_ptr() as *const c_char,
            input.len(),
            start_pos,
            &mut match_out,
            &mut error_out,
        );

        match result {
            1 => {
                // Convert C match to Rust RegexMatch
                let captures = self.convert_captures(&match_out);

                // Free the C captures if needed
                if !match_out.captures.is_null() {
                    if let Some(free_fn) = self.callbacks.free_captures {
                        free_fn(
                            self.callbacks.userdata,
                            match_out.captures,
                            match_out.capture_count,
                        );
                    }
                }

                Ok(Some(RegexMatch {
                    start: match_out.start,
                    end: match_out.end,
                    captures,
                }))
            }
            0 => Ok(None),
            _ => {
                let error_msg = if error_out.is_null() {
                    String::from("Regex find failed")
                } else {
                    unsafe { c_str_to_str(error_out) }
                        .map(String::from)
                        .unwrap_or_else(|| String::from("Regex find failed"))
                };
                Err(error_msg)
            }
        }
    }

    fn find_iter(&self, input: &str) -> Result<Vec<RegexMatch>, String> {
        let mut matches = Vec::new();
        let is_global = self.flags.contains('g');

        if !is_global {
            // Non-global: return at most one match
            if let Some(m) = self.find(input, 0)? {
                matches.push(m);
            }
            return Ok(matches);
        }

        // Global: find all matches
        let mut pos = 0;
        while pos <= input.len() {
            match self.find(input, pos)? {
                Some(m) => {
                    let next_pos = if m.start == m.end {
                        m.end + 1 // Prevent infinite loop on zero-width matches
                    } else {
                        m.end
                    };
                    matches.push(m);
                    pos = next_pos;
                }
                None => break,
            }
        }

        Ok(matches)
    }

    fn split(&self, input: &str) -> Result<Vec<String>, String> {
        // split() always iterates all matches, regardless of global flag
        let matches = self.find_all_matches(input)?;
        let mut result = Vec::new();
        let mut last_end = 0;

        for m in matches {
            if let Some(before) = input.get(last_end..m.start) {
                result.push(String::from(before));
            }
            last_end = m.end;
        }

        if let Some(rest) = input.get(last_end..) {
            result.push(String::from(rest));
        } else if last_end == 0 {
            result.push(String::from(input));
        }

        Ok(result)
    }

    fn replace(&self, input: &str, replacement: &str) -> Result<String, String> {
        match self.find(input, 0)? {
            Some(m) => {
                let expanded = self.expand_replacement(replacement, input, &m);
                let before = input.get(..m.start).unwrap_or("");
                let after = input.get(m.end..).unwrap_or("");
                Ok(alloc::format!("{}{}{}", before, expanded, after))
            }
            None => Ok(String::from(input)),
        }
    }

    fn replace_all(&self, input: &str, replacement: &str) -> Result<String, String> {
        let matches = self.find_iter(input)?;

        if matches.is_empty() {
            return Ok(String::from(input));
        }

        let mut result = String::new();
        let mut last_end = 0;

        for m in matches {
            if let Some(before) = input.get(last_end..m.start) {
                result.push_str(before);
            }
            let expanded = self.expand_replacement(replacement, input, &m);
            result.push_str(&expanded);
            last_end = m.end;
        }

        if let Some(rest) = input.get(last_end..) {
            result.push_str(rest);
        }

        Ok(result)
    }
}

impl CCompiledRegex {
    /// Find all matches regardless of global flag.
    /// Used by split() which always needs all matches.
    fn find_all_matches(&self, input: &str) -> Result<Vec<RegexMatch>, String> {
        let mut matches = Vec::new();
        let mut pos = 0;

        while pos <= input.len() {
            match self.find(input, pos)? {
                Some(m) => {
                    let next_pos = if m.start == m.end {
                        m.end + 1 // Prevent infinite loop on zero-width matches
                    } else {
                        m.end
                    };
                    matches.push(m);
                    pos = next_pos;
                }
                None => break,
            }
        }

        Ok(matches)
    }

    /// Convert C captures to Rust format.
    fn convert_captures(&self, match_out: &TsRunRegexMatch) -> Vec<Option<(usize, usize)>> {
        let mut captures = Vec::new();

        if match_out.captures.is_null() || match_out.capture_count == 0 {
            // No captures provided - use match bounds as group 0
            captures.push(Some((match_out.start, match_out.end)));
            return captures;
        }

        for i in 0..match_out.capture_count {
            let cap = unsafe { *match_out.captures.add(i) };
            if cap.start >= 0 && cap.end >= 0 {
                captures.push(Some((cap.start as usize, cap.end as usize)));
            } else {
                captures.push(None);
            }
        }

        captures
    }

    /// Expand replacement patterns like $1, $&, $$.
    fn expand_replacement(&self, replacement: &str, input: &str, m: &RegexMatch) -> String {
        let mut result = String::new();
        let chars: Vec<char> = replacement.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            if chars.get(i) == Some(&'$') && i + 1 < len {
                match chars.get(i + 1) {
                    Some('$') => {
                        result.push('$');
                        i += 2;
                    }
                    Some('&') => {
                        if let Some(Some((start, end))) = m.captures.first() {
                            if let Some(s) = input.get(*start..*end) {
                                result.push_str(s);
                            }
                        }
                        i += 2;
                    }
                    Some('`') => {
                        if let Some(before) = input.get(..m.start) {
                            result.push_str(before);
                        }
                        i += 2;
                    }
                    Some('\'') => {
                        if let Some(after) = input.get(m.end..) {
                            result.push_str(after);
                        }
                        i += 2;
                    }
                    Some(c) if c.is_ascii_digit() => {
                        let group_num = (*c as usize) - ('0' as usize);
                        if group_num > 0 {
                            if let Some(Some((start, end))) = m.captures.get(group_num) {
                                if let Some(s) = input.get(*start..*end) {
                                    result.push_str(s);
                                }
                            }
                        }
                        i += 2;
                    }
                    _ => {
                        result.push('$');
                        i += 1;
                    }
                }
            } else if let Some(c) = chars.get(i) {
                result.push(*c);
                i += 1;
            } else {
                break;
            }
        }

        result
    }
}

// ============================================================================
// FFI Functions
// ============================================================================

/// Set a custom RegExp provider using C callbacks.
///
/// # Safety
/// - `ctx` must be a valid `TsRunContext` pointer
/// - `callbacks` must be a valid pointer to a `TsRunRegexCallbacks` struct
/// - The callbacks must remain valid for as long as the context is used
///
/// # Returns
/// A `TsRunResult` indicating success or failure.
#[unsafe(no_mangle)]
pub extern "C" fn tsrun_set_regexp_provider(
    ctx: *mut TsRunContext,
    callbacks: *const TsRunRegexCallbacks,
) -> TsRunResult {
    if ctx.is_null() {
        return TsRunResult {
            ok: false,
            error: ptr::null(),
        };
    }

    if callbacks.is_null() {
        let ctx = unsafe { &mut *ctx };
        return TsRunResult::err(ctx, String::from("callbacks is null"));
    }

    let ctx = unsafe { &mut *ctx };
    let callbacks = unsafe { *callbacks };

    // Create provider
    let provider = unsafe { CRegExpProvider::new(callbacks) };

    // Set on interpreter
    ctx.interp.set_regexp_provider(Rc::new(provider));

    TsRunResult::success()
}
