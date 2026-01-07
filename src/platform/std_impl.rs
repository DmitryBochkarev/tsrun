//! Standard library implementations of platform traits.
//!
//! These implementations are only available when the `std` feature is enabled.

use super::{ConsoleLevel, ConsoleProvider, RandomProvider, TimeProvider};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[cfg(feature = "regex")]
use super::{CompiledRegex, RegExpProvider, RegexMatch};
#[cfg(feature = "regex")]
use std::rc::Rc;

/// Time provider using std::time.
pub struct StdTimeProvider {
    /// Reference instant for timer calculations
    epoch: Instant,
}

impl StdTimeProvider {
    /// Create a new StdTimeProvider.
    pub fn new() -> Self {
        Self {
            epoch: Instant::now(),
        }
    }
}

impl Default for StdTimeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeProvider for StdTimeProvider {
    fn now_millis(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }

    fn elapsed_millis(&self, start: u64) -> u64 {
        let now = self.epoch.elapsed().as_millis() as u64;
        now.saturating_sub(start)
    }

    fn start_timer(&self) -> u64 {
        self.epoch.elapsed().as_millis() as u64
    }
}

/// Random provider using a simple xorshift64 PRNG.
///
/// This is a fast, decent-quality PRNG suitable for Math.random().
/// It's seeded from the current time on creation.
pub struct StdRandomProvider {
    state: u64,
}

impl StdRandomProvider {
    /// Create a new StdRandomProvider with time-based seed.
    pub fn new() -> Self {
        // Seed from current time
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x12345678_9abcdef0);

        // Ensure non-zero seed
        let seed = if seed == 0 { 0x12345678_9abcdef0 } else { seed };

        Self { state: seed }
    }

    /// Create with a specific seed (for testing).
    #[allow(dead_code)]
    pub fn with_seed(seed: u64) -> Self {
        let seed = if seed == 0 { 1 } else { seed };
        Self { state: seed }
    }
}

impl Default for StdRandomProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl RandomProvider for StdRandomProvider {
    fn random(&mut self) -> f64 {
        // xorshift64 algorithm
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;

        // Convert to f64 in [0, 1)
        // Use the upper 53 bits for better distribution
        let mantissa = x >> 11; // 53 bits
        (mantissa as f64) / ((1u64 << 53) as f64)
    }
}

/// Console provider using std print macros.
///
/// Writes to stdout for Log/Info/Debug and stderr for Warn/Error.
pub struct StdConsoleProvider;

impl StdConsoleProvider {
    /// Create a new StdConsoleProvider.
    pub fn new() -> Self {
        Self
    }
}

impl Default for StdConsoleProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsoleProvider for StdConsoleProvider {
    fn write(&self, level: ConsoleLevel, message: &str) {
        match level {
            ConsoleLevel::Log | ConsoleLevel::Info | ConsoleLevel::Debug => {
                println!("{message}");
            }
            ConsoleLevel::Warn | ConsoleLevel::Error => {
                eprintln!("{message}");
            }
        }
    }

    fn clear(&self) {
        // Print some newlines as a visual separator
        println!("\n--- Console cleared ---\n");
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// FancyRegexProvider - RegExp implementation using fancy-regex crate
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "regex")]
mod regex_impl {
    use super::*;

    /// RegExp provider using the `fancy-regex` crate.
    ///
    /// This is the default provider for std builds with the `regex` feature enabled.
    /// It supports advanced regex features like lookahead, lookbehind, and backreferences.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct FancyRegexProvider;

    impl FancyRegexProvider {
        /// Create a new FancyRegexProvider.
        pub fn new() -> Self {
            Self
        }
    }

    /// Compiled regex wrapping fancy_regex::Regex.
    #[derive(Debug)]
    pub struct FancyCompiledRegex {
        regex: fancy_regex::Regex,
        #[allow(dead_code)]
        flags: String,
    }

    impl CompiledRegex for FancyCompiledRegex {
        fn is_match(&self, input: &str) -> Result<bool, String> {
            self.regex.is_match(input).map_err(|e| e.to_string())
        }

        fn find(&self, input: &str, start_pos: usize) -> Result<Option<RegexMatch>, String> {
            let slice = input.get(start_pos..).unwrap_or("");
            match self.regex.captures(slice) {
                Ok(Some(caps)) => {
                    let full_match = caps.get(0).ok_or("No match found")?;
                    let captures: Vec<Option<(usize, usize)>> = caps
                        .iter()
                        .map(|m| m.map(|c| (start_pos + c.start(), start_pos + c.end())))
                        .collect();
                    Ok(Some(RegexMatch {
                        start: start_pos + full_match.start(),
                        end: start_pos + full_match.end(),
                        captures,
                    }))
                }
                Ok(None) => Ok(None),
                Err(e) => Err(e.to_string()),
            }
        }

        fn find_iter(&self, input: &str) -> Result<Vec<RegexMatch>, String> {
            let mut results = Vec::new();
            for caps_result in self.regex.captures_iter(input) {
                match caps_result {
                    Ok(caps) => {
                        let full_match = caps.get(0).ok_or("No match found")?;
                        let captures: Vec<Option<(usize, usize)>> = caps
                            .iter()
                            .map(|m| m.map(|c| (c.start(), c.end())))
                            .collect();
                        results.push(RegexMatch {
                            start: full_match.start(),
                            end: full_match.end(),
                            captures,
                        });
                    }
                    Err(e) => return Err(e.to_string()),
                }
            }
            Ok(results)
        }

        fn split(&self, input: &str) -> Result<Vec<String>, String> {
            let parts: Result<Vec<_>, _> = self
                .regex
                .split(input)
                .map(|r| r.map(|s| s.to_string()))
                .collect();
            parts.map_err(|e| e.to_string())
        }

        fn replace(&self, input: &str, replacement: &str) -> Result<String, String> {
            // fancy_regex doesn't support replacement patterns directly,
            // we need to handle $1, $2, etc. ourselves
            match self.regex.captures(input) {
                Ok(Some(caps)) => {
                    let full_match = caps.get(0).ok_or("No match found")?;
                    let replacement_str = expand_replacement(replacement, &caps);
                    let mut result = String::with_capacity(input.len());
                    result.push_str(input.get(..full_match.start()).unwrap_or(""));
                    result.push_str(&replacement_str);
                    result.push_str(input.get(full_match.end()..).unwrap_or(""));
                    Ok(result)
                }
                Ok(None) => Ok(input.to_string()),
                Err(e) => Err(e.to_string()),
            }
        }

        fn replace_all(&self, input: &str, replacement: &str) -> Result<String, String> {
            let mut result = String::with_capacity(input.len());
            let mut last_end = 0;

            for caps_result in self.regex.captures_iter(input) {
                match caps_result {
                    Ok(caps) => {
                        let full_match = caps.get(0).ok_or("No match found")?;
                        let replacement_str = expand_replacement(replacement, &caps);
                        result.push_str(input.get(last_end..full_match.start()).unwrap_or(""));
                        result.push_str(&replacement_str);
                        last_end = full_match.end();
                    }
                    Err(e) => return Err(e.to_string()),
                }
            }
            result.push_str(input.get(last_end..).unwrap_or(""));
            Ok(result)
        }
    }

    /// Expand replacement string with capture group references.
    ///
    /// Supports: $1-$99, $&, $$
    fn expand_replacement(replacement: &str, caps: &fancy_regex::Captures) -> String {
        let mut result = String::with_capacity(replacement.len());
        let chars: Vec<char> = replacement.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '$' && i + 1 < chars.len() {
                let next = chars[i + 1];
                if next == '$' {
                    // $$ -> literal $
                    result.push('$');
                    i += 2;
                } else if next == '&' {
                    // $& -> full match
                    if let Some(m) = caps.get(0) {
                        result.push_str(m.as_str());
                    }
                    i += 2;
                } else if next.is_ascii_digit() {
                    // $1-$99
                    let mut num_str = String::new();
                    let mut j = i + 1;
                    while j < chars.len() && chars[j].is_ascii_digit() && num_str.len() < 2 {
                        num_str.push(chars[j]);
                        j += 1;
                    }
                    if let Ok(group_num) = num_str.parse::<usize>() {
                        if let Some(m) = caps.get(group_num) {
                            result.push_str(m.as_str());
                        }
                        // If group doesn't exist, replace with empty string
                    }
                    i = j;
                } else {
                    // Not a special sequence, keep the $
                    result.push('$');
                    i += 1;
                }
            } else {
                result.push(chars[i]);
                i += 1;
            }
        }
        result
    }

    impl RegExpProvider for FancyRegexProvider {
        fn compile(&self, pattern: &str, flags: &str) -> Result<Rc<dyn CompiledRegex>, String> {
            // Convert JS regex syntax to Rust regex syntax
            let mut regex_pattern = js_regex_to_rust(pattern);

            // Build flags prefix
            let mut prefix = String::new();

            if flags.contains('i') {
                prefix.push('i');
            }
            if flags.contains('m') {
                prefix.push('m');
            }
            if flags.contains('s') {
                prefix.push('s');
            }

            if !prefix.is_empty() {
                regex_pattern = format!("(?{}){}", prefix, regex_pattern);
            }

            let regex = fancy_regex::Regex::new(&regex_pattern)
                .map_err(|e| format!("Invalid regular expression: {}", e))?;

            Ok(Rc::new(FancyCompiledRegex {
                regex,
                flags: flags.to_string(),
            }))
        }
    }

    /// Convert a JavaScript regex pattern to a Rust regex pattern.
    ///
    /// Handles differences between JS and Rust regex syntax:
    /// - In JS, `[` inside a character class is a literal character
    /// - In Rust, `[` inside a character class needs to be escaped as `\[`
    fn js_regex_to_rust(pattern: &str) -> String {
        let mut result = String::with_capacity(pattern.len() + 16);
        let chars: Vec<char> = pattern.chars().collect();
        let len = chars.len();
        let mut i = 0;
        let mut in_char_class = false;
        let mut char_class_start = false; // True right after [ or [^

        while i < len {
            let Some(c) = chars.get(i).copied() else {
                break;
            };

            if c == '\\' {
                if let Some(next) = chars.get(i + 1).copied() {
                    // Escaped character - copy both chars and skip
                    result.push(c);
                    result.push(next);
                    i += 2;
                    char_class_start = false;
                    continue;
                }
            }

            if !in_char_class {
                if c == '[' {
                    in_char_class = true;
                    char_class_start = true;
                    result.push(c);
                } else {
                    result.push(c);
                }
            } else {
                // Inside character class
                if char_class_start {
                    // First char(s) after [ have special meaning
                    if c == '^' {
                        result.push(c);
                        // Still in char_class_start mode - next char could be ]
                    } else if c == ']' {
                        // ] right after [ or [^ is a literal ]
                        result.push(c);
                        char_class_start = false;
                    } else if c == '[' {
                        // [ at start of class - needs escaping for Rust
                        result.push('\\');
                        result.push('[');
                        char_class_start = false;
                    } else {
                        result.push(c);
                        char_class_start = false;
                    }
                } else if c == ']' {
                    // End of character class
                    in_char_class = false;
                    result.push(c);
                } else if c == '[' {
                    // Unescaped [ inside character class - escape it for Rust
                    result.push('\\');
                    result.push('[');
                } else {
                    result.push(c);
                }
            }
            i += 1;
        }

        result
    }
}

#[cfg(feature = "regex")]
pub use regex_impl::FancyRegexProvider;
