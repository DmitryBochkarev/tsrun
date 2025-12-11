//! String dictionary for deduplicating JsString instances.
//!
//! This module provides a dictionary that ensures identical strings share the same
//! `Rc<str>` instance, reducing memory allocations and improving cache locality.

use rustc_hash::FxHashMap;

use crate::value::{CheapClone, JsString};

/// A dictionary for deduplicating JsString instances.
///
/// Strings inserted into the dictionary are stored once and subsequent
/// requests for the same string return a cheap clone of the existing instance.
pub struct StringDict {
    /// Map from string content to shared JsString instance.
    /// Using Box<str> as key to avoid double-indirection through Rc.
    strings: FxHashMap<Box<str>, JsString>,
}

impl StringDict {
    /// Create an empty dictionary.
    pub fn new() -> Self {
        Self {
            strings: FxHashMap::default(),
        }
    }

    /// Create a dictionary pre-populated with common strings.
    pub fn with_common_strings() -> Self {
        let mut dict = Self::new();
        for s in COMMON_STRINGS {
            dict.get_or_insert(s);
        }
        dict
    }

    /// Get an existing string or insert a new one.
    /// Returns a cheap clone of the shared JsString instance.
    pub fn get_or_insert(&mut self, s: &str) -> JsString {
        if let Some(existing) = self.strings.get(s) {
            return existing.cheap_clone();
        }
        let js_str = JsString::from(s);
        self.strings.insert(s.into(), js_str.cheap_clone());
        js_str
    }

    /// Get an existing string without inserting.
    /// Returns None if the string is not in the dictionary.
    #[allow(dead_code)]
    pub fn get(&self, s: &str) -> Option<JsString> {
        self.strings.get(s).map(|s| s.cheap_clone())
    }

    /// Insert a JsString that was created elsewhere.
    /// If the string already exists, returns the existing instance.
    #[allow(dead_code)]
    pub fn insert(&mut self, js_str: JsString) -> JsString {
        if let Some(existing) = self.strings.get(js_str.as_str()) {
            return existing.cheap_clone();
        }
        self.strings
            .insert(js_str.as_str().into(), js_str.cheap_clone());
        js_str
    }

    /// Number of unique strings in the dictionary.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Check if dictionary is empty.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
}

impl Default for StringDict {
    fn default() -> Self {
        Self::new()
    }
}

/// Strings that appear frequently in JavaScript code and runtime.
const COMMON_STRINGS: &[&str] = &[
    // Object properties
    "length",
    "prototype",
    "constructor",
    "__proto__",
    "name",
    "message",
    "stack",
    // Property descriptors
    "value",
    "writable",
    "enumerable",
    "configurable",
    "get",
    "set",
    // Common methods
    "toString",
    "valueOf",
    "hasOwnProperty",
    "toJSON",
    // Array iteration
    "next",
    "done",
    "return",
    "throw",
    // Type names
    "undefined",
    "null",
    "boolean",
    "number",
    "string",
    "object",
    "function",
    "symbol",
    // Built-in constructors
    "Object",
    "Array",
    "String",
    "Number",
    "Boolean",
    "Function",
    "Error",
    "TypeError",
    "ReferenceError",
    "SyntaxError",
    "RangeError",
    "Map",
    "Set",
    "Date",
    "RegExp",
    "Promise",
    "Symbol",
    // Common identifiers
    "this",
    "arguments",
    "callee",
    "caller",
    // Console
    "log",
    "error",
    "warn",
    "info",
    "debug",
    // Math
    "PI",
    "E",
    "abs",
    "floor",
    "ceil",
    "round",
    "max",
    "min",
    // Common variable names
    "i",
    "j",
    "k",
    "x",
    "y",
    "n",
    "s",
    "v",
    "key",
    "val",
    "arr",
    "obj",
    "fn",
    "cb",
    "err",
    "res",
    "req",
    // Array methods
    "push",
    "pop",
    "shift",
    "unshift",
    "slice",
    "splice",
    "concat",
    "join",
    "reverse",
    "sort",
    "indexOf",
    "lastIndexOf",
    "includes",
    "find",
    "findIndex",
    "filter",
    "map",
    "forEach",
    "reduce",
    "reduceRight",
    "every",
    "some",
    "flat",
    "flatMap",
    "fill",
    "copyWithin",
    "at",
    // String methods
    "charAt",
    "charCodeAt",
    "substring",
    "substr",
    "toLowerCase",
    "toUpperCase",
    "trim",
    "trimStart",
    "trimEnd",
    "split",
    "repeat",
    "replace",
    "replaceAll",
    "padStart",
    "padEnd",
    "startsWith",
    "endsWith",
    "match",
    "search",
    // Object methods
    "keys",
    "values",
    "entries",
    "assign",
    "freeze",
    "seal",
    "create",
    "defineProperty",
    "getOwnPropertyDescriptor",
    // Number methods
    "toFixed",
    "toPrecision",
    "toExponential",
    "isNaN",
    "isFinite",
    "isInteger",
    "isSafeInteger",
    "parseInt",
    "parseFloat",
    // Promise methods
    "then",
    "catch",
    "finally",
    "resolve",
    "reject",
    "all",
    "race",
    "allSettled",
    "any",
    // Function methods
    "call",
    "apply",
    "bind",
    // RegExp properties
    "source",
    "flags",
    "global",
    "ignoreCase",
    "multiline",
    "test",
    "exec",
    // Map/Set methods
    "has",
    "delete",
    "clear",
    "size",
    "add",
    // Date methods
    "now",
    "UTC",
    "parse",
    "getTime",
    "getFullYear",
    "getMonth",
    "getDate",
    "getDay",
    "getHours",
    "getMinutes",
    "getSeconds",
    "getMilliseconds",
    "toISOString",
    // JSON
    "stringify",
    // Generator
    "yield",
    // Class related
    "super",
    "static",
    "extends",
    "implements",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_dict_deduplication() {
        let mut dict = StringDict::new();
        let s1 = dict.get_or_insert("hello");
        let s2 = dict.get_or_insert("hello");

        // Should be the same string value
        assert_eq!(s1, s2);
        // Should point to the same memory (same Rc)
        assert!(std::ptr::eq(s1.as_str(), s2.as_str()));
    }

    #[test]
    fn test_string_dict_different_strings() {
        let mut dict = StringDict::new();
        let s1 = dict.get_or_insert("hello");
        let s2 = dict.get_or_insert("world");

        // Different strings
        assert_ne!(s1, s2);
        // Different memory locations
        assert!(!std::ptr::eq(s1.as_str(), s2.as_str()));
    }

    #[test]
    fn test_common_strings_preloaded() {
        let dict = StringDict::with_common_strings();
        assert!(dict.get("length").is_some());
        assert!(dict.get("prototype").is_some());
        assert!(dict.get("toString").is_some());
    }

    #[test]
    fn test_string_dict_len() {
        let mut dict = StringDict::new();
        assert_eq!(dict.len(), 0);
        assert!(dict.is_empty());

        dict.get_or_insert("hello");
        assert_eq!(dict.len(), 1);
        assert!(!dict.is_empty());

        // Same string doesn't increase count
        dict.get_or_insert("hello");
        assert_eq!(dict.len(), 1);

        dict.get_or_insert("world");
        assert_eq!(dict.len(), 2);
    }
}
