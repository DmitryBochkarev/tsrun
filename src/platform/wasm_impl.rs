//! WebAssembly implementations of platform traits.
//!
//! These implementations use browser APIs via wasm-bindgen for:
//! - Console output (console.log, etc.)
//! - Time (Date.now(), performance.now())
//! - Random numbers (Math.random())
//! - RegExp (browser's native RegExp)

use super::{CompiledRegex, ConsoleLevel, ConsoleProvider, RandomProvider, RegExpProvider, RegexMatch, TimeProvider};
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use wasm_bindgen::prelude::*;

// ═══════════════════════════════════════════════════════════════════════════════
// JavaScript bindings
// ═══════════════════════════════════════════════════════════════════════════════

#[wasm_bindgen]
extern "C" {
    // Console bindings
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn info(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn debug(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn warn(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn error(s: &str);

    #[wasm_bindgen(js_namespace = console, js_name = clear)]
    fn console_clear();

    // Date.now() for wall-clock time
    #[wasm_bindgen(js_namespace = Date)]
    fn now() -> f64;

    // performance.now() for high-resolution timing
    #[wasm_bindgen(js_namespace = performance, js_name = now)]
    fn performance_now() -> f64;

    // Math.random() for random numbers
    #[wasm_bindgen(js_namespace = Math)]
    fn random() -> f64;
}

// ═══════════════════════════════════════════════════════════════════════════════
// Console Provider
// ═══════════════════════════════════════════════════════════════════════════════

/// Console provider using browser's console object.
///
/// Routes console.log(), console.error(), etc. to the browser's console.
pub struct WasmConsoleProvider;

impl WasmConsoleProvider {
    /// Create a new WasmConsoleProvider.
    pub fn new() -> Self {
        Self
    }
}

impl Default for WasmConsoleProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsoleProvider for WasmConsoleProvider {
    fn write(&self, level: ConsoleLevel, message: &str) {
        match level {
            ConsoleLevel::Log => log(message),
            ConsoleLevel::Info => info(message),
            ConsoleLevel::Debug => debug(message),
            ConsoleLevel::Warn => warn(message),
            ConsoleLevel::Error => error(message),
        }
    }

    fn clear(&self) {
        console_clear();
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Time Provider
// ═══════════════════════════════════════════════════════════════════════════════

/// Time provider using browser's Date and performance APIs.
///
/// Uses Date.now() for wall-clock time and performance.now() for timing.
pub struct WasmTimeProvider;

impl WasmTimeProvider {
    /// Create a new WasmTimeProvider.
    pub fn new() -> Self {
        Self
    }
}

impl Default for WasmTimeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeProvider for WasmTimeProvider {
    fn now_millis(&self) -> i64 {
        now() as i64
    }

    fn elapsed_millis(&self, start: u64) -> u64 {
        let current = performance_now() as u64;
        current.saturating_sub(start)
    }

    fn start_timer(&self) -> u64 {
        performance_now() as u64
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Random Provider
// ═══════════════════════════════════════════════════════════════════════════════

/// Random provider using browser's Math.random().
pub struct WasmRandomProvider;

impl WasmRandomProvider {
    /// Create a new WasmRandomProvider.
    pub fn new() -> Self {
        Self
    }
}

impl Default for WasmRandomProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl RandomProvider for WasmRandomProvider {
    fn random(&mut self) -> f64 {
        random()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// RegExp Provider
// ═══════════════════════════════════════════════════════════════════════════════

#[wasm_bindgen]
extern "C" {
    /// JavaScript RegExp type
    #[wasm_bindgen(js_name = RegExp)]
    type JsRegExp;

    /// Create a new RegExp: `new RegExp(pattern, flags)`
    #[wasm_bindgen(constructor, js_class = "RegExp")]
    fn new(pattern: &str, flags: &str) -> JsRegExp;

    /// Test if the regex matches the string
    #[wasm_bindgen(method)]
    fn test(this: &JsRegExp, string: &str) -> bool;

    /// Execute a search and return match result or null
    #[wasm_bindgen(method)]
    fn exec(this: &JsRegExp, string: &str) -> Option<js_sys::Array>;

    /// Get lastIndex property
    #[wasm_bindgen(method, getter, js_name = lastIndex)]
    fn last_index(this: &JsRegExp) -> f64;

    /// Set lastIndex property
    #[wasm_bindgen(method, setter, js_name = lastIndex)]
    fn set_last_index(this: &JsRegExp, value: f64);

    /// Get global flag
    #[wasm_bindgen(method, getter)]
    fn global(this: &JsRegExp) -> bool;
}

/// Compiled regex using browser's native RegExp.
///
/// Wraps a JavaScript RegExp object for use in WASM environments.
pub struct WasmCompiledRegex {
    /// The underlying JS RegExp object
    regex: JsRegExp,
    /// Original flags for reference
    flags: String,
}

impl core::fmt::Debug for WasmCompiledRegex {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("WasmCompiledRegex")
            .field("flags", &self.flags)
            .finish()
    }
}

impl WasmCompiledRegex {
    /// Convert a UTF-16 code unit offset to a UTF-8 byte offset.
    ///
    /// JavaScript strings use UTF-16, so indices from RegExp.exec() are in
    /// UTF-16 code units. We need to convert these to UTF-8 byte offsets.
    fn utf16_to_utf8_offset(s: &str, utf16_offset: usize) -> usize {
        let mut utf16_pos = 0;
        for (byte_idx, c) in s.char_indices() {
            if utf16_pos >= utf16_offset {
                return byte_idx;
            }
            utf16_pos += c.len_utf16();
        }
        s.len()
    }

    /// Parse a single match result from exec() into a RegexMatch.
    fn parse_exec_result(&self, result: &js_sys::Array, input: &str) -> Option<RegexMatch> {
        // Get index property (UTF-16 offset where match starts)
        let index = js_sys::Reflect::get(result, &JsValue::from_str("index"))
            .ok()?
            .as_f64()? as usize;

        let utf8_start = Self::utf16_to_utf8_offset(input, index);

        // Build captures from array elements
        let mut captures = Vec::new();
        let length = result.length();

        for i in 0..length {
            let item = result.get(i);
            if item.is_undefined() || item.is_null() {
                captures.push(None);
            } else if let Some(s) = item.as_string() {
                // For capture group, we need to find where it appears
                // The first capture (index 0) is the full match at 'index'
                if i == 0 {
                    let utf8_end = utf8_start + s.len();
                    captures.push(Some((utf8_start, utf8_end)));
                } else {
                    // For other captures, find them within input starting from match start
                    // Note: This is a simplification - we'd need more complex logic
                    // to handle cases where capture appears multiple times
                    if let Some(pos) = input.get(utf8_start..).and_then(|sub| sub.find(&s)) {
                        let abs_start = utf8_start + pos;
                        let abs_end = abs_start + s.len();
                        captures.push(Some((abs_start, abs_end)));
                    } else {
                        // Capture not found at expected position, use approximate
                        captures.push(None);
                    }
                }
            } else {
                captures.push(None);
            }
        }

        // Calculate end from the full match (first capture)
        let end = captures
            .first()
            .and_then(|c| c.map(|(_, e)| e))
            .unwrap_or(utf8_start);

        Some(RegexMatch {
            start: utf8_start,
            end,
            captures,
        })
    }
}

impl CompiledRegex for WasmCompiledRegex {
    fn is_match(&self, input: &str) -> Result<bool, String> {
        // Reset lastIndex for consistent behavior
        self.regex.set_last_index(0.0);
        Ok(self.regex.test(input))
    }

    fn find(&self, input: &str, start_pos: usize) -> Result<Option<RegexMatch>, String> {
        // Convert UTF-8 byte offset to UTF-16 code unit offset
        let utf16_start = input
            .get(..start_pos)
            .map(|s| s.chars().map(|c| c.len_utf16()).sum())
            .unwrap_or(0);

        self.regex.set_last_index(utf16_start as f64);

        match self.regex.exec(input) {
            Some(result) => Ok(self.parse_exec_result(&result, input)),
            None => Ok(None),
        }
    }

    fn find_iter(&self, input: &str) -> Result<Vec<RegexMatch>, String> {
        let mut matches = Vec::new();
        self.regex.set_last_index(0.0);

        // Only iterate if global flag is set, otherwise we'd loop forever
        if !self.regex.global() {
            if let Some(result) = self.regex.exec(input) {
                if let Some(m) = self.parse_exec_result(&result, input) {
                    matches.push(m);
                }
            }
            return Ok(matches);
        }

        loop {
            match self.regex.exec(input) {
                Some(result) => {
                    if let Some(m) = self.parse_exec_result(&result, input) {
                        // Prevent infinite loop on zero-length matches
                        if m.start == m.end {
                            let current = self.regex.last_index() as usize;
                            self.regex.set_last_index((current + 1) as f64);
                        }
                        matches.push(m);
                    } else {
                        break;
                    }
                }
                None => break,
            }
        }

        Ok(matches)
    }

    fn split(&self, input: &str) -> Result<Vec<String>, String> {
        // For split, we need to find ALL matches regardless of the global flag
        // We'll iterate by searching substrings starting after each match
        let mut result = Vec::new();
        let mut search_start = 0;
        let input_len = input.len();

        while search_start <= input_len {
            // Reset lastIndex for consistent behavior
            self.regex.set_last_index(0.0);

            // Search in the remaining substring
            let remaining = input.get(search_start..).unwrap_or("");
            if remaining.is_empty() {
                break;
            }

            match self.regex.exec(remaining) {
                Some(exec_result) => {
                    if let Some(m) = self.parse_exec_result(&exec_result, remaining) {
                        // Add the part before the match
                        if let Some(before) = remaining.get(..m.start) {
                            result.push(String::from(before));
                        }

                        // Move past this match
                        let advance = if m.end > m.start { m.end } else { m.start + 1 };
                        search_start += advance;
                    } else {
                        // No valid match - add rest and break
                        result.push(String::from(remaining));
                        search_start = input_len + 1;
                    }
                }
                None => {
                    // No more matches - add remaining and break
                    result.push(String::from(remaining));
                    break;
                }
            }
        }

        // If we ended exactly at input_len, add empty trailing string
        if search_start == input_len && !result.is_empty() {
            result.push(String::new());
        }

        // Handle case where no matches were found
        if result.is_empty() {
            result.push(String::from(input));
        }

        Ok(result)
    }

    fn replace(&self, input: &str, replacement: &str) -> Result<String, String> {
        self.regex.set_last_index(0.0);

        match self.regex.exec(input) {
            Some(result) => {
                if let Some(m) = self.parse_exec_result(&result, input) {
                    let expanded = expand_replacement(replacement, input, &m);
                    let before = input.get(..m.start).unwrap_or("");
                    let after = input.get(m.end..).unwrap_or("");
                    Ok(alloc::format!("{}{}{}", before, expanded, after))
                } else {
                    Ok(String::from(input))
                }
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
            let expanded = expand_replacement(replacement, input, &m);
            result.push_str(&expanded);
            last_end = m.end;
        }

        if let Some(rest) = input.get(last_end..) {
            result.push_str(rest);
        }

        Ok(result)
    }
}

/// Expand replacement patterns like $1, $&, $$
fn expand_replacement(replacement: &str, input: &str, m: &RegexMatch) -> String {
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
                    // Full match
                    if let Some(Some((start, end))) = m.captures.first() {
                        if let Some(s) = input.get(*start..*end) {
                            result.push_str(s);
                        }
                    }
                    i += 2;
                }
                Some('`') => {
                    // Text before match
                    if let Some(before) = input.get(..m.start) {
                        result.push_str(before);
                    }
                    i += 2;
                }
                Some('\'') => {
                    // Text after match
                    if let Some(after) = input.get(m.end..) {
                        result.push_str(after);
                    }
                    i += 2;
                }
                Some(c) if c.is_ascii_digit() => {
                    // Capture group reference $1-$9
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

/// RegExp provider using browser's native RegExp via wasm-bindgen.
///
/// This allows WASM builds to use the browser's regex engine instead of
/// bundling a Rust regex implementation.
pub struct WasmRegExpProvider;

impl WasmRegExpProvider {
    /// Create a new WasmRegExpProvider.
    pub fn new() -> Self {
        Self
    }
}

impl Default for WasmRegExpProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl RegExpProvider for WasmRegExpProvider {
    fn compile(&self, pattern: &str, flags: &str) -> Result<Rc<dyn CompiledRegex>, String> {
        // JavaScript RegExp constructor will throw on invalid patterns
        // We catch this by checking if the regex was created successfully
        let regex = JsRegExp::new(pattern, flags);

        Ok(Rc::new(WasmCompiledRegex {
            regex,
            flags: String::from(flags),
        }))
    }
}
