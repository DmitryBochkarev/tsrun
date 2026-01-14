//! WASM RegExp provider that delegates to host's native RegExp.
//!
//! This module implements the `RegExpProvider` trait by calling imported host functions
//! that use the browser's native `RegExp` object (or equivalent in other WASM hosts).

use crate::platform::{CompiledRegex, RegExpProvider, RegexMatch};
use crate::prelude::{Rc, String, Vec};
use alloc::string::ToString;
use core::fmt;

/// RegExp provider that delegates to host via WASM imports.
pub struct WasmRegExpProvider;

impl RegExpProvider for WasmRegExpProvider {
    fn compile(&self, pattern: &str, flags: &str) -> Result<Rc<dyn CompiledRegex>, String> {
        let mut error_ptr: u32 = 0;
        let mut error_len: u32 = 0;

        let handle = unsafe {
            super::host_regex_compile(
                pattern.as_ptr(),
                pattern.len() as u32,
                flags.as_ptr(),
                flags.len() as u32,
                &mut error_ptr,
                &mut error_len,
            )
        };

        if handle == 0 {
            let error_msg = if error_ptr != 0 && error_len > 0 {
                let error_bytes = unsafe {
                    core::slice::from_raw_parts(error_ptr as *const u8, error_len as usize)
                };
                let msg = core::str::from_utf8(error_bytes)
                    .unwrap_or("Invalid regex")
                    .to_string();
                unsafe { super::host_free_string(error_ptr, error_len) };
                msg
            } else {
                String::from("Invalid regular expression")
            };
            return Err(error_msg);
        }

        Ok(Rc::new(WasmCompiledRegex {
            handle,
            flags: String::from(flags),
        }))
    }
}

/// Compiled regex backed by host's native RegExp.
struct WasmCompiledRegex {
    handle: u32,
    flags: String,
}

impl fmt::Debug for WasmCompiledRegex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WasmCompiledRegex")
            .field("handle", &self.handle)
            .field("flags", &self.flags)
            .finish()
    }
}

impl Drop for WasmCompiledRegex {
    fn drop(&mut self) {
        if self.handle != 0 {
            unsafe { super::host_regex_free(self.handle) };
        }
    }
}

impl CompiledRegex for WasmCompiledRegex {
    fn is_match(&self, input: &str) -> Result<bool, String> {
        let result =
            unsafe { super::host_regex_test(self.handle, input.as_ptr(), input.len() as u32) };
        Ok(result == 1)
    }

    fn find(&self, input: &str, start_pos: usize) -> Result<Option<RegexMatch>, String> {
        let mut match_start: u32 = 0;
        let mut match_end: u32 = 0;
        let mut captures_ptr: u32 = 0;
        let mut captures_count: u32 = 0;

        let found = unsafe {
            super::host_regex_exec(
                self.handle,
                input.as_ptr(),
                input.len() as u32,
                start_pos as u32,
                &mut match_start,
                &mut match_end,
                &mut captures_ptr,
                &mut captures_count,
            )
        };

        if found != 1 {
            return Ok(None);
        }

        // Parse captures from binary array: pairs of i32 (start, end), -1 = non-participating
        let captures = if captures_ptr != 0 && captures_count > 0 {
            let data = unsafe {
                core::slice::from_raw_parts(
                    captures_ptr as *const i32,
                    (captures_count * 2) as usize,
                )
            };

            let mut caps = Vec::with_capacity(captures_count as usize);
            for i in 0..(captures_count as usize) {
                let start = data[i * 2];
                let end = data[i * 2 + 1];
                if start >= 0 && end >= 0 {
                    caps.push(Some((start as usize, end as usize)));
                } else {
                    caps.push(None);
                }
            }

            unsafe { super::host_free_captures(captures_ptr, captures_count) };
            caps
        } else {
            Vec::new()
        };

        Ok(Some(RegexMatch {
            start: match_start as usize,
            end: match_end as usize,
            captures,
        }))
    }

    fn find_iter(&self, input: &str) -> Result<Vec<RegexMatch>, String> {
        let mut matches = Vec::new();
        let is_global = self.flags.contains('g');

        if !is_global {
            if let Some(m) = self.find(input, 0)? {
                matches.push(m);
            }
            return Ok(matches);
        }

        let mut pos = 0;
        while pos <= input.len() {
            match self.find(input, pos)? {
                Some(m) => {
                    let next_pos = if m.start == m.end { m.end + 1 } else { m.end };
                    matches.push(m);
                    pos = next_pos;
                }
                None => break,
            }
        }

        Ok(matches)
    }

    fn split(&self, input: &str) -> Result<Vec<String>, String> {
        let mut parts_ptr: u32 = 0;
        let mut parts_count: u32 = 0;

        let status = unsafe {
            super::host_regex_split(
                self.handle,
                input.as_ptr(),
                input.len() as u32,
                &mut parts_ptr,
                &mut parts_count,
            )
        };

        if status != 1 {
            return Err(String::from("Regex split failed"));
        }

        if parts_ptr == 0 || parts_count == 0 {
            return Ok(Vec::new());
        }

        // Parse parts from binary array: pairs of (ptr: u32, len: u32)
        let data = unsafe {
            core::slice::from_raw_parts(parts_ptr as *const u32, (parts_count * 2) as usize)
        };

        let mut result = Vec::with_capacity(parts_count as usize);
        for i in 0..(parts_count as usize) {
            let ptr = data[i * 2];
            let len = data[i * 2 + 1];
            if ptr != 0 && len > 0 {
                let bytes = unsafe { core::slice::from_raw_parts(ptr as *const u8, len as usize) };
                let s = core::str::from_utf8(bytes).unwrap_or("").to_string();
                result.push(s);
            } else {
                result.push(String::new());
            }
        }

        unsafe { super::host_free_split_result(parts_ptr, parts_count) };

        Ok(result)
    }

    fn replace(&self, input: &str, replacement: &str) -> Result<String, String> {
        let mut result_ptr: u32 = 0;
        let mut result_len: u32 = 0;

        let status = unsafe {
            super::host_regex_replace(
                self.handle,
                input.as_ptr(),
                input.len() as u32,
                replacement.as_ptr(),
                replacement.len() as u32,
                0, // not global
                &mut result_ptr,
                &mut result_len,
            )
        };

        if status != 1 || result_ptr == 0 {
            return Err(String::from("Regex replace failed"));
        }

        let result_bytes =
            unsafe { core::slice::from_raw_parts(result_ptr as *const u8, result_len as usize) };
        let result = core::str::from_utf8(result_bytes).unwrap_or("").to_string();

        unsafe { super::host_free_string(result_ptr, result_len) };

        Ok(result)
    }

    fn replace_all(&self, input: &str, replacement: &str) -> Result<String, String> {
        let mut result_ptr: u32 = 0;
        let mut result_len: u32 = 0;

        let status = unsafe {
            super::host_regex_replace(
                self.handle,
                input.as_ptr(),
                input.len() as u32,
                replacement.as_ptr(),
                replacement.len() as u32,
                1, // global
                &mut result_ptr,
                &mut result_len,
            )
        };

        if status != 1 || result_ptr == 0 {
            return Err(String::from("Regex replace_all failed"));
        }

        let result_bytes =
            unsafe { core::slice::from_raw_parts(result_ptr as *const u8, result_len as usize) };
        let result = core::str::from_utf8(result_bytes).unwrap_or("").to_string();

        unsafe { super::host_free_string(result_ptr, result_len) };

        Ok(result)
    }
}
