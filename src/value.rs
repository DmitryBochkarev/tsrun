//! JavaScript value representation
//!
//! The core JsValue type and related structures for representing JavaScript values at runtime.

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use rustc_hash::FxHashMap;

/// Convert a JavaScript number to its canonical string representation.
///
/// According to ECMAScript spec (7.1.12.1 NumberToString):
/// - Very small numbers (< 1e-6) use exponential notation
/// - Very large numbers (>= 1e21) use exponential notation
/// - Otherwise use decimal notation
/// - Integer values are printed without decimal point
pub fn number_to_string(n: f64) -> String {
    if n.is_nan() {
        return "NaN".to_string();
    }
    if n.is_infinite() {
        return if n > 0.0 {
            "Infinity".to_string()
        } else {
            "-Infinity".to_string()
        };
    }
    if n == 0.0 {
        return "0".to_string();
    }

    let abs_n = n.abs();

    // Check if it's an integer that can be represented exactly
    if n.trunc() == n && abs_n < 1e21 {
        // Format as integer (no decimal point)
        return format!("{:.0}", n);
    }

    // Very small numbers (absolute value < 1e-6) use exponential notation
    // Very large numbers (absolute value >= 1e21) use exponential notation
    if !(1e-6..1e21).contains(&abs_n) {
        // Use exponential notation
        format_exponential(n)
    } else {
        // Use decimal notation
        // We need to produce the shortest representation that round-trips
        format_decimal(n)
    }
}

/// Format a number in exponential notation matching JavaScript's output
fn format_exponential(n: f64) -> String {
    // Get the exponent
    let abs_n = n.abs();
    let exponent = abs_n.log10().floor() as i32;
    let mantissa = n / 10_f64.powi(exponent);

    // Format mantissa - remove trailing zeros after decimal point
    let mantissa_str = if mantissa.trunc() == mantissa {
        format!("{:.0}", mantissa)
    } else {
        let s = format!("{}", mantissa);
        // Remove trailing zeros but keep at least one digit after decimal
        s.trim_end_matches('0').to_string()
    };

    // Format exponent with sign
    if exponent >= 0 {
        format!("{}e+{}", mantissa_str, exponent)
    } else {
        format!("{}e{}", mantissa_str, exponent)
    }
}

/// Format a number in decimal notation matching JavaScript's output
fn format_decimal(n: f64) -> String {
    // Use Rust's default formatting which handles most cases
    let s = format!("{}", n);

    // Remove trailing zeros after decimal point (but keep at least one digit)
    if s.contains('.') {
        let trimmed = s.trim_end_matches('0');
        if trimmed.ends_with('.') {
            format!("{}0", trimmed)
        } else {
            trimmed.to_string()
        }
    } else {
        s
    }
}

/// Convert a JavaScript string to a number according to ECMAScript ToNumber.
///
/// The string is first trimmed of leading and trailing whitespace.
/// Then it is parsed as a numeric literal:
/// - Empty string → 0
/// - "Infinity", "+Infinity", "-Infinity" → Infinity, +Infinity, -Infinity
/// - "0x" or "0X" prefix → hexadecimal
/// - "0o" or "0O" prefix → octal
/// - "0b" or "0B" prefix → binary
/// - Otherwise → decimal (with optional sign, exponent)
/// - Invalid → NaN
pub fn string_to_number(s: &str) -> f64 {
    // ECMAScript whitespace includes more than just ASCII whitespace
    // Trim using a custom function that matches JS behavior
    let trimmed = trim_js_whitespace(s);

    // Empty string → 0
    if trimmed.is_empty() {
        return 0.0;
    }

    // Handle Infinity (case-sensitive)
    if trimmed == "Infinity" || trimmed == "+Infinity" {
        return f64::INFINITY;
    }
    if trimmed == "-Infinity" {
        return f64::NEG_INFINITY;
    }

    // Check for hex/octal/binary prefixes (case-insensitive)
    if trimmed.len() >= 2 {
        let bytes = trimmed.as_bytes();
        if bytes.first() == Some(&b'0') {
            match bytes.get(1) {
                Some(b'x' | b'X') => {
                    // Hexadecimal: 0x...
                    let hex_part = trimmed.get(2..).unwrap_or("");
                    if hex_part.is_empty() {
                        return f64::NAN;
                    }
                    return match u64::from_str_radix(hex_part, 16) {
                        Ok(n) => n as f64,
                        Err(_) => f64::NAN,
                    };
                }
                Some(b'o' | b'O') => {
                    // Octal: 0o...
                    let oct_part = trimmed.get(2..).unwrap_or("");
                    if oct_part.is_empty() {
                        return f64::NAN;
                    }
                    return match u64::from_str_radix(oct_part, 8) {
                        Ok(n) => n as f64,
                        Err(_) => f64::NAN,
                    };
                }
                Some(b'b' | b'B') => {
                    // Binary: 0b...
                    let bin_part = trimmed.get(2..).unwrap_or("");
                    if bin_part.is_empty() {
                        return f64::NAN;
                    }
                    return match u64::from_str_radix(bin_part, 2) {
                        Ok(n) => n as f64,
                        Err(_) => f64::NAN,
                    };
                }
                _ => {}
            }
        }
    }

    // Reject case-insensitive infinity variants (Rust accepts them, JS doesn't)
    // We already handled "Infinity", "+Infinity", "-Infinity" above
    let lower = trimmed.to_lowercase();
    if lower.contains("infinity") || lower.contains("inf") {
        return f64::NAN;
    }

    // Reject NaN (Rust accepts "NaN", "nan", etc., but JS Number() returns NaN for all)
    // This is subtle: JS parses the string "NaN" as NaN, but so does Rust, so this is fine.
    // Actually, we need to check if it's a non-standard casing
    if lower == "nan" && trimmed != "NaN" {
        // Non-standard casing of NaN - return NaN (which is correct anyway)
        return f64::NAN;
    }

    // Try to parse as a decimal number
    // Rust's parse::<f64> handles most cases, but we need to ensure
    // the entire string is consumed and matches JS semantics
    match trimmed.parse::<f64>() {
        Ok(n) => n,
        Err(_) => f64::NAN,
    }
}

/// Trim JavaScript whitespace from both ends of a string.
/// JavaScript whitespace includes:
/// - ASCII whitespace: space, tab, LF, CR, form feed, vertical tab
/// - Unicode: no-break space (00A0), BOM (FEFF), line separator (2028), paragraph separator (2029)
/// - And other Unicode space separators
fn trim_js_whitespace(s: &str) -> &str {
    fn is_js_whitespace(c: char) -> bool {
        matches!(
            c,
            ' ' | '\t'
                | '\n'
                | '\r'
                | '\x0B'
                | '\x0C'
                | '\u{00A0}'
                | '\u{FEFF}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{1680}'
                | '\u{2000}'
                | '\u{2001}'
                | '\u{2002}'
                | '\u{2003}'
                | '\u{2004}'
                | '\u{2005}'
                | '\u{2006}'
                | '\u{2007}'
                | '\u{2008}'
                | '\u{2009}'
                | '\u{200A}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
        )
    }

    s.trim_matches(is_js_whitespace)
}

use crate::ast::{ArrowFunctionBody, BlockStatement, Expression, FunctionParam};
use crate::error::JsError;
use crate::gc::{Gc, GcPtr, Guard, Heap, Reset, Traceable};

/// Trait for types that have cheap (O(1), reference-counted) clones.
///
/// This trait makes it explicit when a clone is cheap (just incrementing a reference count)
/// vs when it might be expensive (copying data). Types implementing this trait should have
/// O(1) clone operations, typically because they use `Rc` or similar reference counting.
///
/// # Examples
/// - `JsObjectRef` (Rc<RefCell<JsObject>>) - cheap clone
/// - `JsString` (Rc<str>) - cheap clone
/// - `Environment` (contains Rc) - cheap clone
///
/// Regular `.clone()` should still work but requires a comment explaining why the clone
/// is necessary when the type doesn't implement `CheapClone`.
pub trait CheapClone: Clone {
    /// Create a cheap (reference-counted) clone of this value.
    ///
    /// This is semantically identical to `clone()` but makes it explicit that
    /// the operation is O(1) and only increments a reference count.
    fn cheap_clone(&self) -> Self {
        self.clone()
    }
}

// Implement CheapClone for Rc-based types (Rc<RefCell<T>> is covered by this)
impl<T: ?Sized> CheapClone for Rc<T> {}
impl<T: CheapClone> CheapClone for Option<T> {}

/// A JavaScript value
///
/// Size-optimized: JsSymbol is boxed since symbols are rare, allowing JsValue
/// to be 16 bytes instead of 32 bytes.
#[derive(Clone, Default)]
pub enum JsValue {
    #[default]
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(JsString),
    Symbol(Box<JsSymbol>),
    Object(JsObjectRef),
}

/// A JsValue bundled with a Guard that keeps it alive.
///
/// IMPORTANT: Access the value through destructuring ONLY to ensure the guard
/// stays alive for the correct duration. See CLAUDE.md for rules.
///
/// The fields are public to allow struct destructuring pattern, which is the
/// ONLY approved way to access the contents.
pub struct Guarded {
    pub value: JsValue,
    pub guard: Option<Guard<JsObject>>,
}

impl std::fmt::Debug for Guarded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Guarded")
            .field("value", &self.value)
            .field("guard", &self.guard.as_ref().map(|_| "<guard>"))
            .finish()
    }
}

impl Guarded {
    /// Create a guarded value with a guard.
    /// If the value is a primitive, the guard is dropped and `guard` will be `None`.
    /// NOTE: You must call `value.guard_by(&guard)` before this if the value is an object.
    pub fn with_guard(value: JsValue, guard: Guard<JsObject>) -> Self {
        if matches!(value, JsValue::Object(_)) {
            Self {
                value,
                guard: Some(guard),
            }
        } else {
            // Primitives don't need guarding - drop the guard
            drop(guard);
            Self { value, guard: None }
        }
    }

    /// Create an unguarded value (for primitives or already-owned objects)
    pub fn unguarded(value: JsValue) -> Self {
        Self { value, guard: None }
    }

    /// Create a guarded value for returning from the VM.
    /// Creates a new guard from the heap if the value is an object.
    /// For primitives, returns an unguarded value.
    pub fn from_value(value: JsValue, heap: &Heap<JsObject>) -> Self {
        if let JsValue::Object(obj) = &value {
            let guard = heap.create_guard();
            guard.guard(obj.cheap_clone());
            Self {
                value,
                guard: Some(guard),
            }
        } else {
            Self { value, guard: None }
        }
    }

    /// Return a new value with the same guard (for derived values)
    ///
    /// Use this when you derive a value from a guarded input and want to
    /// propagate the guard to keep the original object alive.
    pub fn with_value(self, value: JsValue) -> Self {
        Self {
            value,
            guard: self.guard,
        }
    }
}

impl JsValue {
    /// Check if this value is null or undefined
    pub fn is_null_or_undefined(&self) -> bool {
        matches!(self, JsValue::Null | JsValue::Undefined)
    }

    /// Check if this value is callable (a function)
    pub fn is_callable(&self) -> bool {
        match self {
            JsValue::Object(obj) => {
                matches!(obj.borrow().exotic, ExoticObject::Function(_))
            }
            _ => false,
        }
    }

    /// Check if this is a string value
    pub fn is_string(&self) -> bool {
        matches!(self, JsValue::String(_))
    }

    /// If this value is an object, add it to the guard.
    /// This keeps the object alive as long as the guard exists.
    pub fn guard_by(&self, guard: &Guard<JsObject>) {
        if let JsValue::Object(obj) = self {
            guard.guard(obj.cheap_clone());
        }
    }

    /// Get the typeof result for this value
    // FIXME: use intern strings
    pub fn type_of(&self) -> &'static str {
        match self {
            JsValue::Undefined => "undefined",
            JsValue::Null => "object", // Historical quirk
            JsValue::Boolean(_) => "boolean",
            JsValue::Number(_) => "number",
            JsValue::String(_) => "string",
            JsValue::Symbol(_) => "symbol",
            JsValue::Object(obj) => {
                if obj.borrow().is_callable() {
                    "function"
                } else {
                    "object"
                }
            }
        }
    }

    /// Convert to boolean (ToBoolean)
    pub fn to_boolean(&self) -> bool {
        match self {
            JsValue::Undefined | JsValue::Null => false,
            JsValue::Boolean(b) => *b,
            JsValue::Number(n) => *n != 0.0 && !n.is_nan(),
            JsValue::String(s) => !s.is_empty(),
            JsValue::Symbol(_) => true, // Symbols are always truthy
            JsValue::Object(_) => true,
        }
    }

    /// Convert to number (ToNumber)
    pub fn to_number(&self) -> f64 {
        match self {
            JsValue::Undefined => f64::NAN,
            JsValue::Null => 0.0,
            JsValue::Boolean(true) => 1.0,
            JsValue::Boolean(false) => 0.0,
            JsValue::Number(n) => *n,
            JsValue::String(s) => string_to_number(s.as_str()),
            JsValue::Symbol(_) => f64::NAN, // Cannot convert Symbol to number
            JsValue::Object(obj) => {
                // Handle wrapper objects with primitives
                let borrowed = obj.borrow();
                match &borrowed.exotic {
                    ExoticObject::Number(n) => *n,
                    ExoticObject::Boolean(b) => {
                        if *b {
                            1.0
                        } else {
                            0.0
                        }
                    }
                    ExoticObject::StringObj(s) => string_to_number(s.as_str()),
                    _ => f64::NAN, // Other objects would need ToPrimitive
                }
            }
        }
    }

    /// Convert to string (ToString)
    // FIXME: use interned strings
    pub fn to_js_string(&self) -> JsString {
        match self {
            JsValue::Undefined => JsString::from("undefined"),
            JsValue::Null => JsString::from("null"),
            JsValue::Boolean(true) => JsString::from("true"),
            JsValue::Boolean(false) => JsString::from("false"),
            JsValue::Number(n) => JsString::from(number_to_string(*n)),
            JsValue::String(s) => s.clone(),
            JsValue::Symbol(s) => {
                // Symbol.prototype.toString returns "Symbol(description)"
                match &s.description {
                    Some(desc) => JsString::from(format!("Symbol({})", desc.as_str())),
                    None => JsString::from("Symbol()"),
                }
            }
            JsValue::Object(obj) => {
                // Check for wrapper objects that have primitive values
                let borrowed = obj.borrow();
                match &borrowed.exotic {
                    ExoticObject::Number(n) => {
                        // Number wrapper - convert the primitive to string
                        JsValue::Number(*n).to_js_string()
                    }
                    ExoticObject::StringObj(s) => {
                        // String wrapper - return the wrapped string
                        s.clone()
                    }
                    ExoticObject::Boolean(b) => {
                        // Boolean wrapper - convert the primitive to string
                        if *b {
                            JsString::from("true")
                        } else {
                            JsString::from("false")
                        }
                    }
                    ExoticObject::Array { elements } => {
                        // Array.prototype.toString joins elements with comma
                        let strings: Vec<String> = elements
                            .iter()
                            .map(|v| {
                                // null and undefined become empty strings in join
                                match v {
                                    JsValue::Null | JsValue::Undefined => String::new(),
                                    _ => v.to_js_string().to_string(),
                                }
                            })
                            .collect();
                        JsString::from(strings.join(","))
                    }
                    _ => JsString::from("[object Object]"),
                }
            }
        }
    }

    /// Strict equality (===)
    pub fn strict_equals(&self, other: &JsValue) -> bool {
        match (self, other) {
            (JsValue::Undefined, JsValue::Undefined) => true,
            (JsValue::Null, JsValue::Null) => true,
            (JsValue::Boolean(a), JsValue::Boolean(b)) => a == b,
            (JsValue::Number(a), JsValue::Number(b)) => {
                // NaN !== NaN
                if a.is_nan() || b.is_nan() {
                    false
                } else {
                    a == b
                }
            }
            (JsValue::String(a), JsValue::String(b)) => a == b,
            (JsValue::Symbol(a), JsValue::Symbol(b)) => a == b, // Symbols compare by id
            (JsValue::Object(a), JsValue::Object(b)) => Gc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl fmt::Debug for JsValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsValue::Undefined => write!(f, "undefined"),
            JsValue::Null => write!(f, "null"),
            JsValue::Boolean(b) => write!(f, "{}", b),
            JsValue::Number(n) => write!(f, "{}", n),
            JsValue::String(s) => write!(f, "\"{}\"", s.as_ref()),
            JsValue::Symbol(s) => match &s.description {
                Some(desc) => write!(f, "Symbol({})", desc.as_str()),
                None => write!(f, "Symbol()"),
            },
            JsValue::Object(obj) => {
                let obj = obj.borrow();
                match &obj.exotic {
                    ExoticObject::Ordinary => {
                        // Check if this is an Error object (has name and message properties)
                        let name_key = PropertyKey::from("name");
                        let message_key = PropertyKey::from("message");

                        if let (Some(name_prop), Some(message_prop)) = (
                            obj.properties.get(&name_key),
                            obj.properties.get(&message_key),
                        ) {
                            let name = name_prop.value.to_js_string();
                            let msg = message_prop.value.to_js_string();
                            write!(f, "{}: {}", name.as_ref(), msg.as_ref())
                        } else {
                            write!(f, "{{...}}")
                        }
                    }
                    ExoticObject::Array { .. } => write!(f, "[...]"),
                    ExoticObject::Function(func) => {
                        let name = func.name().unwrap_or("anonymous");
                        write!(f, "[Function: {}]", name)
                    }
                    ExoticObject::Map { entries } => write!(f, "Map({})", entries.len()),
                    ExoticObject::Set { entries } => write!(f, "Set({})", entries.len()),
                    ExoticObject::Date { timestamp } => write!(f, "Date({})", timestamp),
                    ExoticObject::RegExp { pattern, flags } => write!(f, "/{}/{}", pattern, flags),
                    ExoticObject::Generator(_) => write!(f, "[object Generator]"),
                    ExoticObject::BytecodeGenerator(_) => write!(f, "[object Generator]"),
                    ExoticObject::Promise(state) => {
                        let status = match state.borrow().status {
                            PromiseStatus::Pending => "pending",
                            PromiseStatus::Fulfilled => "fulfilled",
                            PromiseStatus::Rejected => "rejected",
                        };
                        write!(f, "Promise {{{}}}", status)
                    }
                    ExoticObject::Environment(env_data) => {
                        write!(f, "[Environment {} bindings]", env_data.bindings.len())
                    }
                    ExoticObject::Enum(data) => {
                        write!(f, "enum {}", data.name)
                    }
                    ExoticObject::Proxy(proxy_data) => {
                        if proxy_data.revoked {
                            write!(f, "[Proxy (revoked)]")
                        } else {
                            write!(f, "[Proxy]")
                        }
                    }
                    ExoticObject::Boolean(b) => write!(f, "[Boolean: {}]", b),
                    ExoticObject::Number(n) => write!(f, "[Number: {}]", n),
                    ExoticObject::StringObj(s) => write!(f, "[String: \"{}\"]", s),
                    ExoticObject::RawJSON(raw) => write!(f, "[RawJSON: {}]", raw),
                }
            }
        }
    }
}

impl PartialEq for JsValue {
    fn eq(&self, other: &Self) -> bool {
        self.strict_equals(other)
    }
}

// Conversions from Rust types

impl From<bool> for JsValue {
    fn from(b: bool) -> Self {
        JsValue::Boolean(b)
    }
}

impl From<f64> for JsValue {
    fn from(n: f64) -> Self {
        JsValue::Number(n)
    }
}

impl From<i32> for JsValue {
    fn from(n: i32) -> Self {
        JsValue::Number(n as f64)
    }
}

impl From<&str> for JsValue {
    fn from(s: &str) -> Self {
        JsValue::String(JsString::from(s))
    }
}

impl From<String> for JsValue {
    fn from(s: String) -> Self {
        JsValue::String(JsString::from(s))
    }
}

impl From<JsString> for JsValue {
    fn from(s: JsString) -> Self {
        JsValue::String(s)
    }
}

/// Reference-counted string for efficient string handling
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct JsString(Rc<str>);

/// A key for variable lookups that uses pointer-based hashing.
///
/// This wrapper around JsString uses the pointer address for hashing and equality,
/// which is O(1) instead of O(n) for content-based hashing. This is safe because
/// all variable names are interned through StringDict, so identical names share
/// the same Rc allocation.
///
/// WARNING: Only use this for interned strings! Using non-interned strings will
/// cause incorrect lookups (two equal strings might not be found).
#[derive(Clone)]
pub struct VarKey(pub JsString);

impl std::hash::Hash for VarKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Hash the pointer address, not the content
        // Use the data pointer from the fat pointer (Rc<str> is a fat pointer)
        let ptr = Rc::as_ptr(&self.0 .0) as *const () as usize;
        ptr.hash(state);
    }
}

impl PartialEq for VarKey {
    fn eq(&self, other: &Self) -> bool {
        // Compare pointer addresses, not content
        Rc::ptr_eq(&self.0 .0, &other.0 .0)
    }
}

impl Eq for VarKey {}

impl std::fmt::Debug for VarKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VarKey({:?})", self.0)
    }
}

impl std::fmt::Display for VarKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<JsString> for VarKey {
    fn from(s: JsString) -> Self {
        VarKey(s)
    }
}

/// Unique identifier for a private field/method.
///
/// Private fields in JavaScript are "branded" - two classes can both have
/// a field named `#x`, but they are different fields. Each class gets a unique
/// ClassBrandId, and a PrivateFieldKey combines this with the field name.
pub type ClassBrandId = u32;

/// Key for looking up private fields in an object.
///
/// This combines a class brand (unique per class definition) with the field name.
/// The field_name includes the # prefix (e.g., "#count").
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PrivateFieldKey {
    /// Unique identifier for the class that defined this private field
    pub class_brand: ClassBrandId,
    /// The private field name (including # prefix)
    pub field_name: JsString,
}

impl PrivateFieldKey {
    pub fn new(class_brand: ClassBrandId, field_name: JsString) -> Self {
        Self {
            class_brand,
            field_name,
        }
    }
}

// JsString wraps Rc<str>, so clone is cheap (just reference count increment)
impl CheapClone for JsString {}

impl JsString {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn parse<F: std::str::FromStr>(&self) -> Result<F, F::Err> {
        self.0.parse()
    }
}

impl AsRef<str> for JsString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::borrow::Borrow<str> for JsString {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl PartialEq<str> for JsString {
    fn eq(&self, other: &str) -> bool {
        self.0.as_ref() == other
    }
}

impl PartialEq<&str> for JsString {
    fn eq(&self, other: &&str) -> bool {
        self.0.as_ref() == *other
    }
}

impl From<&str> for JsString {
    fn from(s: &str) -> Self {
        JsString(s.into())
    }
}

impl From<String> for JsString {
    fn from(s: String) -> Self {
        JsString(s.into())
    }
}

impl fmt::Debug for JsString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}\"", self.0)
    }
}

impl fmt::Display for JsString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::ops::Add<&str> for JsString {
    type Output = JsString;

    fn add(self, other: &str) -> JsString {
        let mut s = String::from(&*self.0);
        s.push_str(other);
        JsString::from(s)
    }
}

impl std::ops::Add<&JsString> for JsString {
    type Output = JsString;

    fn add(self, other: &JsString) -> JsString {
        let mut s = String::from(&*self.0);
        s.push_str(&other.0);
        JsString::from(s)
    }
}

/// JavaScript Symbol primitive
/// Symbols are unique identifiers, optionally with a description
#[derive(Clone, Debug)]
pub struct JsSymbol {
    /// Unique identifier for this symbol
    id: u64,
    /// Optional description (from Symbol('description'))
    pub description: Option<JsString>,
}

impl JsSymbol {
    /// Create a new unique symbol with an optional description
    pub fn new(id: u64, description: Option<JsString>) -> Self {
        Self { id, description }
    }

    /// Get the symbol's unique ID
    pub fn id(&self) -> u64 {
        self.id
    }
}

impl PartialEq for JsSymbol {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for JsSymbol {}

impl std::hash::Hash for JsSymbol {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

/// Reference to a heap-allocated object (GC-managed)
///
/// This is now `Gc<JsObject>` which gives us `Ref<'_, JsObject>` directly
/// from `borrow()` calls, matching the old `Rc<RefCell<JsObject>>` API.
pub type JsObjectRef = Gc<JsObject>;

// Implement CheapClone for Gc<T> (clone is just Copy)
impl<T: Default + Reset + Traceable> CheapClone for Gc<T> {}

/// Reset implementation for JsObject - used by new GC for object pooling.
///
/// When an object is collected, it can be reset and reused instead of
/// being dropped. This is more efficient than allocating new objects.
impl Reset for JsObject {
    fn reset(&mut self) {
        self.prototype = None;
        self.extensible = true;
        self.frozen = false;
        self.sealed = false;
        self.null_prototype = false;
        // clear() preserves capacity, avoiding reallocation for reused objects
        self.properties.clear();
        self.exotic = ExoticObject::Ordinary;
        self.private_fields = None;
    }
}

impl Traceable for JsObject {
    fn trace<F: FnMut(GcPtr<Self>)>(&self, mut visitor: F) {
        // Trace prototype - use copy_ref to avoid incrementing ref_count during tracing
        if let Some(proto) = &self.prototype {
            visitor(proto.copy_ref());
        }

        // Trace properties (both data values and accessor functions)
        for prop in self.properties.values() {
            if let JsValue::Object(obj) = &prop.value {
                visitor(obj.copy_ref());
            }
            // Trace accessor getter/setter functions
            if let Some(accessor) = &prop.accessor {
                if let Some(getter) = &accessor.getter {
                    visitor(getter.copy_ref());
                }
                if let Some(setter) = &accessor.setter {
                    visitor(setter.copy_ref());
                }
            }
        }

        // Trace exotic object references
        match &self.exotic {
            ExoticObject::Function(func) => {
                match func {
                    JsFunction::Bound(bound) => {
                        visitor(bound.target.copy_ref());
                        if let JsValue::Object(obj) = &bound.this_arg {
                            visitor(obj.copy_ref());
                        }
                        for arg in &bound.bound_args {
                            if let JsValue::Object(obj) = arg {
                                visitor(obj.copy_ref());
                            }
                        }
                    }
                    JsFunction::PromiseResolve(promise) | JsFunction::PromiseReject(promise) => {
                        visitor(promise.copy_ref());
                    }
                    JsFunction::PromiseAllFulfill { state, .. } => {
                        // Trace the result promise and all results
                        visitor(state.result_promise.copy_ref());
                        for result in state.results.borrow().iter() {
                            if let JsValue::Object(obj) = result {
                                visitor(obj.copy_ref());
                            }
                        }
                    }
                    JsFunction::PromiseAllReject(state) => {
                        // Trace the result promise and all results
                        visitor(state.result_promise.copy_ref());
                        for result in state.results.borrow().iter() {
                            if let JsValue::Object(obj) = result {
                                visitor(obj.copy_ref());
                            }
                        }
                    }
                    JsFunction::Bytecode(bc)
                    | JsFunction::BytecodeGenerator(bc)
                    | JsFunction::BytecodeAsync(bc)
                    | JsFunction::BytecodeAsyncGenerator(bc) => {
                        // Trace the closure environment
                        visitor(bc.closure.copy_ref());
                        // Trace captured this (for arrow functions)
                        if let Some(this) = &bc.captured_this {
                            if let JsValue::Object(obj) = this.as_ref() {
                                visitor(obj.copy_ref());
                            }
                        }
                    }
                    JsFunction::ModuleExportGetter { module_env, .. } => {
                        // Trace the module environment for live bindings
                        visitor(module_env.copy_ref());
                    }
                    JsFunction::ModuleReExportGetter { source_module, .. } => {
                        // Trace the source module for re-export live bindings
                        visitor(source_module.copy_ref());
                    }
                    JsFunction::ProxyRevoke(proxy) => {
                        // Trace the associated proxy object
                        visitor(proxy.copy_ref());
                    }
                    JsFunction::Native(_)
                    | JsFunction::AccessorGetter
                    | JsFunction::AccessorSetter => {}
                }
            }
            ExoticObject::Map { entries } => {
                for (k, v) in entries {
                    if let JsValue::Object(obj) = k {
                        visitor(obj.copy_ref());
                    }
                    if let JsValue::Object(obj) = v {
                        visitor(obj.copy_ref());
                    }
                }
            }
            ExoticObject::Set { entries } => {
                for entry in entries {
                    if let JsValue::Object(obj) = entry {
                        visitor(obj.copy_ref());
                    }
                }
            }
            ExoticObject::Promise(state) => {
                let state = state.borrow();
                if let Some(JsValue::Object(obj)) = &state.result {
                    visitor(obj.copy_ref());
                }
                for handler in &state.handlers {
                    if let Some(JsValue::Object(obj)) = &handler.on_fulfilled {
                        visitor(obj.copy_ref());
                    }
                    if let Some(JsValue::Object(obj)) = &handler.on_rejected {
                        visitor(obj.copy_ref());
                    }
                    visitor(handler.result_promise.copy_ref());
                }
            }
            ExoticObject::Generator(state) => {
                let state = state.borrow();
                // Trace closure environment
                visitor(state.closure.copy_ref());
                // Trace arguments that might be objects
                for arg in &state.args {
                    if let JsValue::Object(obj) = arg {
                        visitor(obj.copy_ref());
                    }
                }
                // Trace sent value if it's an object
                if let JsValue::Object(obj) = &state.sent_value {
                    visitor(obj.copy_ref());
                }
            }
            ExoticObject::BytecodeGenerator(state) => {
                let state = state.borrow();
                // Trace closure environment
                visitor(state.closure.copy_ref());
                // Trace arguments
                for arg in &state.args {
                    if let JsValue::Object(obj) = arg {
                        visitor(obj.copy_ref());
                    }
                }
                // Trace this_value if it's an object
                if let JsValue::Object(obj) = &state.this_value {
                    visitor(obj.copy_ref());
                }
                // Trace saved register values
                for reg in &state.saved_registers {
                    if let JsValue::Object(obj) = reg {
                        visitor(obj.copy_ref());
                    }
                }
                // Trace sent value if it's an object
                if let JsValue::Object(obj) = &state.sent_value {
                    visitor(obj.copy_ref());
                }
                // Trace function environment
                if let Some(env) = &state.func_env {
                    visitor(env.copy_ref());
                }
                // Trace current environment at yield point
                if let Some(env) = &state.current_env {
                    visitor(env.copy_ref());
                }
                // Trace delegated iterator for yield*
                if let Some((iter_obj, next_method)) = &state.delegated_iterator {
                    visitor(iter_obj.copy_ref());
                    if let JsValue::Object(obj) = next_method {
                        visitor(obj.copy_ref());
                    }
                }
                // Trace throw value for generator.throw()
                if let Some(JsValue::Object(obj)) = &state.throw_value {
                    visitor(obj.copy_ref());
                }
            }
            ExoticObject::Environment(env_data) => {
                // Trace all bindings in the environment
                for binding in env_data.bindings.values() {
                    if let JsValue::Object(obj) = &binding.value {
                        visitor(obj.copy_ref());
                    }
                }
                // Trace outer environment if any
                if let Some(outer) = &env_data.outer {
                    visitor(outer.copy_ref());
                }
            }
            ExoticObject::Array { elements } => {
                // Trace all array elements that are objects
                for elem in elements {
                    if let JsValue::Object(obj) = elem {
                        visitor(obj.copy_ref());
                    }
                }
            }
            ExoticObject::Ordinary
            | ExoticObject::Date { .. }
            | ExoticObject::RegExp { .. }
            | ExoticObject::Enum(_)
            | ExoticObject::Boolean(_)
            | ExoticObject::Number(_)
            | ExoticObject::StringObj(_)
            | ExoticObject::RawJSON(_) => {
                // These exotic types don't contain object references that need tracing
            }
            ExoticObject::Proxy(proxy_data) => {
                // Trace target and handler objects
                visitor(proxy_data.target.copy_ref());
                visitor(proxy_data.handler.copy_ref());
            }
        }

        // Trace private fields (may contain object references)
        if let Some(private_fields) = &self.private_fields {
            for value in private_fields.values() {
                if let JsValue::Object(obj) = value {
                    visitor(obj.copy_ref());
                }
            }
        }
    }
}

/// A JavaScript object
#[derive(Debug)]
pub struct JsObject {
    /// Prototype link
    pub prototype: Option<JsObjectRef>,
    /// Whether the object can have properties added
    pub extensible: bool,
    /// Whether the object is frozen (no modifications allowed)
    pub frozen: bool,
    /// Whether the object is sealed (no new properties, but existing can be modified)
    pub sealed: bool,
    /// Whether this object was explicitly created with null prototype (Object.create(null))
    pub null_prototype: bool,
    /// Object properties (optimized for small objects)
    pub properties: PropertyStorage,
    /// Exotic object behavior
    pub exotic: ExoticObject,
    /// Private fields storage (only used by instances of classes with private members)
    /// Key is (ClassBrandId, field_name), value is the private field/method value
    pub private_fields: Option<FxHashMap<PrivateFieldKey, JsValue>>,
}

impl JsObject {
    /// Create a new ordinary object
    pub fn new() -> Self {
        Self {
            prototype: None,
            extensible: true,
            frozen: false,
            sealed: false,
            null_prototype: false,
            properties: PropertyStorage::new(),
            exotic: ExoticObject::Ordinary,
            private_fields: None,
        }
    }

    /// Create a new ordinary object with pre-allocated property capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            prototype: None,
            extensible: true,
            frozen: false,
            sealed: false,
            null_prototype: false,
            properties: PropertyStorage::with_capacity(capacity),
            exotic: ExoticObject::Ordinary,
            private_fields: None,
        }
    }

    /// Create a new ordinary object with a prototype
    pub fn with_prototype(prototype: JsObjectRef) -> Self {
        Self {
            prototype: Some(prototype),
            extensible: true,
            frozen: false,
            sealed: false,
            null_prototype: false,
            properties: PropertyStorage::new(),
            exotic: ExoticObject::Ordinary,
            private_fields: None,
        }
    }

    /// Get a private field value
    pub fn get_private_field(&self, key: &PrivateFieldKey) -> Option<&JsValue> {
        self.private_fields.as_ref().and_then(|pf| pf.get(key))
    }

    /// Set a private field value
    pub fn set_private_field(&mut self, key: PrivateFieldKey, value: JsValue) {
        self.private_fields
            .get_or_insert_with(FxHashMap::default)
            .insert(key, value);
    }

    /// Check if an object has a specific private field
    pub fn has_private_field(&self, key: &PrivateFieldKey) -> bool {
        self.private_fields
            .as_ref()
            .is_some_and(|pf| pf.contains_key(key))
    }

    /// Check if this object is callable
    pub fn is_callable(&self) -> bool {
        match &self.exotic {
            ExoticObject::Function(_) => true,
            ExoticObject::Proxy(data) => {
                // A proxy is callable if its target is callable
                !data.revoked && data.target.borrow().is_callable()
            }
            _ => false,
        }
    }

    /// Get an own property
    pub fn get_own_property(&self, key: &PropertyKey) -> Option<&Property> {
        self.properties.get(key)
    }

    /// Get a property, searching the prototype chain
    // FIXME: return reference
    pub fn get_property(&self, key: &PropertyKey) -> Option<JsValue> {
        // For arrays, handle index access and length from elements Vec
        if let ExoticObject::Array { ref elements } = self.exotic {
            match key {
                PropertyKey::Index(idx) => {
                    return elements.get(*idx as usize).cloned();
                }
                PropertyKey::String(s) if s.as_str() == "length" => {
                    return Some(JsValue::Number(elements.len() as f64));
                }
                _ => {}
            }
        }

        // For functions, handle name and length properties
        if let ExoticObject::Function(ref func) = self.exotic {
            if let PropertyKey::String(s) = key {
                match s.as_str() {
                    "name" => {
                        // First check if there's an own property (for SetFunctionName override)
                        if let Some(prop) = self.properties.get(key) {
                            return Some(prop.value.clone());
                        }
                        let name = match func {
                            JsFunction::Native(f) => Some(f.name.clone()),
                            JsFunction::Bytecode(bc)
                            | JsFunction::BytecodeGenerator(bc)
                            | JsFunction::BytecodeAsync(bc)
                            | JsFunction::BytecodeAsyncGenerator(bc) => bc
                                .chunk
                                .function_info
                                .as_ref()
                                .and_then(|info| info.name.clone()),
                            JsFunction::Bound(b) => {
                                // Bound functions have name "bound <target name>"
                                if let ExoticObject::Function(target_func) =
                                    &b.target.borrow().exotic
                                {
                                    match target_func {
                                        JsFunction::Native(f) => {
                                            Some(JsString::from(format!("bound {}", f.name)))
                                        }
                                        JsFunction::Bytecode(bc)
                                        | JsFunction::BytecodeGenerator(bc)
                                        | JsFunction::BytecodeAsync(bc)
                                        | JsFunction::BytecodeAsyncGenerator(bc) => {
                                            bc.chunk.function_info.as_ref().and_then(|info| {
                                                info.name
                                                    .as_ref()
                                                    .map(|n| JsString::from(format!("bound {}", n)))
                                            })
                                        }
                                        _ => Some(JsString::from("bound ")),
                                    }
                                } else {
                                    Some(JsString::from("bound "))
                                }
                            }
                            _ => None,
                        };
                        return Some(match name {
                            Some(n) => JsValue::String(n),
                            None => JsValue::String(JsString::from("")),
                        });
                    }
                    "length" => {
                        let arity = match func {
                            JsFunction::Native(f) => f.arity,
                            JsFunction::Bytecode(bc)
                            | JsFunction::BytecodeGenerator(bc)
                            | JsFunction::BytecodeAsync(bc)
                            | JsFunction::BytecodeAsyncGenerator(bc) => bc
                                .chunk
                                .function_info
                                .as_ref()
                                .map(|info| info.param_count)
                                .unwrap_or(0),
                            JsFunction::Bound(b) => {
                                // Bound functions have length = target.length - bound_args.length (min 0)
                                if let ExoticObject::Function(target_func) =
                                    &b.target.borrow().exotic
                                {
                                    let target_length = match target_func {
                                        JsFunction::Native(f) => f.arity,
                                        JsFunction::Bytecode(bc)
                                        | JsFunction::BytecodeGenerator(bc)
                                        | JsFunction::BytecodeAsync(bc)
                                        | JsFunction::BytecodeAsyncGenerator(bc) => bc
                                            .chunk
                                            .function_info
                                            .as_ref()
                                            .map(|info| info.param_count)
                                            .unwrap_or(0),
                                        _ => 0,
                                    };
                                    target_length.saturating_sub(b.bound_args.len())
                                } else {
                                    0
                                }
                            }
                            _ => 0,
                        };
                        return Some(JsValue::Number(arity as f64));
                    }
                    _ => {}
                }
            }
        }

        // For enums, handle member lookups from EnumData
        if let ExoticObject::Enum(ref data) = self.exotic {
            match key {
                PropertyKey::String(s) => {
                    // Forward mapping: member name -> value
                    if let Some(val) = data.get_by_name(s.as_str()) {
                        return Some(val);
                    }
                    // Also check if this is a numeric string for reverse mapping
                    if let Ok(n) = s.as_str().parse::<f64>() {
                        if let Some(name) = data.get_by_value(n) {
                            return Some(JsValue::String(name));
                        }
                    }
                }
                PropertyKey::Index(idx) => {
                    // Reverse mapping: numeric index -> member name
                    if let Some(name) = data.get_by_value(*idx as f64) {
                        return Some(JsValue::String(name));
                    }
                }
                PropertyKey::Symbol(_) => {}
            }
        }

        if let Some(prop) = self.properties.get(key) {
            return Some(prop.value.clone());
        }

        if let Some(ref proto) = self.prototype {
            return proto.borrow().get_property(key);
        }

        None
    }

    /// Get a property descriptor, searching the prototype chain
    /// Returns (property, found_in_prototype)
    pub fn get_property_descriptor(&self, key: &PropertyKey) -> Option<(Property, bool)> {
        // For arrays, handle index access and length from elements Vec
        if let ExoticObject::Array { ref elements } = self.exotic {
            match key {
                PropertyKey::Index(idx) => {
                    if let Some(val) = elements.get(*idx as usize) {
                        return Some((Property::data(val.clone()), false));
                    }
                    // Index out of bounds - return None (falls through to prototype)
                }
                PropertyKey::String(s) if s.as_str() == "length" => {
                    return Some((
                        Property::data(JsValue::Number(elements.len() as f64)),
                        false,
                    ));
                }
                _ => {}
            }
        }

        // For Maps, compute size from entries
        if let ExoticObject::Map { ref entries } = self.exotic {
            if let PropertyKey::String(s) = key {
                if s.as_str() == "size" {
                    return Some((Property::data(JsValue::Number(entries.len() as f64)), false));
                }
            }
        }

        // For Sets, compute size from entries
        if let ExoticObject::Set { ref entries } = self.exotic {
            if let PropertyKey::String(s) = key {
                if s.as_str() == "size" {
                    return Some((Property::data(JsValue::Number(entries.len() as f64)), false));
                }
            }
        }

        // For functions, handle name and length properties
        if let ExoticObject::Function(ref func) = self.exotic {
            if let PropertyKey::String(s) = key {
                match s.as_str() {
                    "name" => {
                        let name = match func {
                            JsFunction::Native(f) => Some(f.name.clone()),
                            JsFunction::Bytecode(bc)
                            | JsFunction::BytecodeGenerator(bc)
                            | JsFunction::BytecodeAsync(bc)
                            | JsFunction::BytecodeAsyncGenerator(bc) => bc
                                .chunk
                                .function_info
                                .as_ref()
                                .and_then(|info| info.name.clone()),
                            JsFunction::Bound(b) => {
                                // Bound functions have name "bound <target name>"
                                if let ExoticObject::Function(target_func) =
                                    &b.target.borrow().exotic
                                {
                                    match target_func {
                                        JsFunction::Native(f) => {
                                            Some(JsString::from(format!("bound {}", f.name)))
                                        }
                                        JsFunction::Bytecode(bc)
                                        | JsFunction::BytecodeGenerator(bc)
                                        | JsFunction::BytecodeAsync(bc)
                                        | JsFunction::BytecodeAsyncGenerator(bc) => {
                                            bc.chunk.function_info.as_ref().and_then(|info| {
                                                info.name
                                                    .as_ref()
                                                    .map(|n| JsString::from(format!("bound {}", n)))
                                            })
                                        }
                                        _ => Some(JsString::from("bound ")),
                                    }
                                } else {
                                    Some(JsString::from("bound "))
                                }
                            }
                            _ => None,
                        };
                        // Per ES spec, function name is: writable: false, enumerable: false, configurable: true
                        return Some((
                            Property::with_attributes(
                                match name {
                                    Some(n) => JsValue::String(n),
                                    None => JsValue::String(JsString::from("")),
                                },
                                false, // writable
                                false, // enumerable
                                true,  // configurable
                            ),
                            false,
                        ));
                    }
                    "length" => {
                        let arity = match func {
                            JsFunction::Native(f) => f.arity,
                            JsFunction::Bytecode(bc)
                            | JsFunction::BytecodeGenerator(bc)
                            | JsFunction::BytecodeAsync(bc)
                            | JsFunction::BytecodeAsyncGenerator(bc) => bc
                                .chunk
                                .function_info
                                .as_ref()
                                .map(|info| info.param_count)
                                .unwrap_or(0),
                            JsFunction::Bound(b) => {
                                // Bound functions have length = target.length - bound_args.length (min 0)
                                if let ExoticObject::Function(target_func) =
                                    &b.target.borrow().exotic
                                {
                                    let target_length = match target_func {
                                        JsFunction::Native(f) => f.arity,
                                        JsFunction::Bytecode(bc)
                                        | JsFunction::BytecodeGenerator(bc)
                                        | JsFunction::BytecodeAsync(bc)
                                        | JsFunction::BytecodeAsyncGenerator(bc) => bc
                                            .chunk
                                            .function_info
                                            .as_ref()
                                            .map(|info| info.param_count)
                                            .unwrap_or(0),
                                        _ => 0,
                                    };
                                    target_length.saturating_sub(b.bound_args.len())
                                } else {
                                    0
                                }
                            }
                            _ => 0,
                        };
                        // Per ES spec, function length is: writable: false, enumerable: false, configurable: true
                        return Some((
                            Property::with_attributes(
                                JsValue::Number(arity as f64),
                                false, // writable
                                false, // enumerable
                                true,  // configurable
                            ),
                            false,
                        ));
                    }
                    _ => {}
                }
            }
        }

        // For enums, handle member lookups from EnumData
        if let ExoticObject::Enum(ref data) = self.exotic {
            match key {
                PropertyKey::String(s) => {
                    // Forward mapping: member name -> value
                    if let Some(val) = data.get_by_name(s.as_str()) {
                        return Some((Property::data(val), false));
                    }
                    // Also check if this is a numeric string for reverse mapping
                    if let Ok(n) = s.as_str().parse::<f64>() {
                        if let Some(name) = data.get_by_value(n) {
                            return Some((Property::data(JsValue::String(name)), false));
                        }
                    }
                }
                PropertyKey::Index(idx) => {
                    // Reverse mapping: numeric index -> member name
                    if let Some(name) = data.get_by_value(*idx as f64) {
                        return Some((Property::data(JsValue::String(name)), false));
                    }
                }
                PropertyKey::Symbol(_) => {}
            }
        }

        if let Some(prop) = self.properties.get(key) {
            return Some((prop.clone(), false));
        }

        if let Some(ref proto) = self.prototype {
            if let Some((prop, _)) = proto.borrow().get_property_descriptor(key) {
                return Some((prop, true));
            }
        }

        None
    }

    /// Set a property
    pub fn set_property(&mut self, key: PropertyKey, value: JsValue) {
        // Frozen objects cannot be modified at all
        if self.frozen {
            return;
        }

        // For arrays, handle index access via elements Vec
        if let ExoticObject::Array { ref mut elements } = self.exotic {
            if let PropertyKey::Index(idx) = key {
                let idx = idx as usize;
                // Extend array with undefined if needed (dense array)
                if idx >= elements.len() {
                    elements.resize(idx + 1, JsValue::Undefined);
                }
                // Safe: we just resized to ensure idx is in bounds
                if let Some(slot) = elements.get_mut(idx) {
                    *slot = value;
                }
                return;
            }
            // Setting length truncates or extends the array
            if let PropertyKey::String(ref s) = key {
                if s.as_str() == "length" {
                    if let JsValue::Number(n) = value {
                        let new_len = n as usize;
                        elements.resize(new_len, JsValue::Undefined);
                    }
                    return;
                }
            }
        }

        // For enums, handle member access via EnumData
        if let ExoticObject::Enum(ref mut data) = self.exotic {
            if let PropertyKey::String(ref s) = key {
                // Update existing member or add new one
                if data.set_by_name(s.as_str(), value.clone()) {
                    // Also update reverse mapping if value is numeric
                    if let JsValue::Number(n) = &value {
                        // Find and update the reverse mapping entry
                        let reverse_key = if n.fract() == 0.0 && *n >= 0.0 && *n <= u32::MAX as f64
                        {
                            PropertyKey::Index(*n as u32)
                        } else {
                            PropertyKey::String(JsString::from(n.to_string()))
                        };
                        self.properties.insert(
                            reverse_key,
                            Property::data(JsValue::String(s.cheap_clone())),
                        );
                    }
                    return;
                }
                // If not an existing member, allow adding new properties
            }
        }

        if let Some(prop) = self.properties.get_mut(&key) {
            // Only set if writable
            if prop.writable() {
                prop.value = value;
            }
        } else if self.extensible && !self.sealed {
            // Sealed objects cannot have new properties added
            self.properties.insert(key, Property::data(value));
        }
    }

    /// Define a property with attributes
    pub fn define_property(&mut self, key: PropertyKey, prop: Property) {
        self.properties.insert(key, prop);
    }

    /// Check if object has own property
    pub fn has_own_property(&self, key: &PropertyKey) -> bool {
        self.properties.contains_key(key)
    }

    /// Get own property keys
    pub fn own_keys(&self) -> Vec<PropertyKey> {
        self.properties.keys().cloned().collect()
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Array-specific methods for efficient element access
    // ═══════════════════════════════════════════════════════════════════════════

    /// Get array length if this is an array, None otherwise
    #[inline]
    pub fn array_length(&self) -> Option<u32> {
        if let ExoticObject::Array { ref elements } = self.exotic {
            Some(elements.len() as u32)
        } else {
            None
        }
    }

    /// Get array elements slice if this is an array
    #[inline]
    pub fn array_elements(&self) -> Option<&[JsValue]> {
        if let ExoticObject::Array { ref elements } = self.exotic {
            Some(elements)
        } else {
            None
        }
    }

    /// Get mutable array elements if this is an array
    #[inline]
    pub fn array_elements_mut(&mut self) -> Option<&mut Vec<JsValue>> {
        if let ExoticObject::Array { ref mut elements } = self.exotic {
            Some(elements)
        } else {
            None
        }
    }

    /// Check if this is an array
    #[inline]
    pub fn is_array(&self) -> bool {
        matches!(self.exotic, ExoticObject::Array { .. })
    }
}

impl Default for JsObject {
    fn default() -> Self {
        Self::new()
    }
}

/// Property key (string, index, or symbol)
///
/// Size-optimized: JsSymbol is boxed since symbol keys are rare.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PropertyKey {
    String(JsString),
    Index(u32),
    Symbol(Box<JsSymbol>),
}

impl PropertyKey {
    pub fn from_value(value: &JsValue) -> Self {
        match value {
            JsValue::Number(n) => {
                let idx = *n as u32;
                if idx as f64 == *n && *n >= 0.0 {
                    PropertyKey::Index(idx)
                } else {
                    PropertyKey::String(value.to_js_string())
                }
            }
            JsValue::String(s) => {
                // Check if it's a valid array index
                if let Ok(idx) = s.parse::<u32>() {
                    if idx.to_string() == s.as_str() {
                        return PropertyKey::Index(idx);
                    }
                }
                PropertyKey::String(s.cheap_clone())
            }
            JsValue::Symbol(s) => PropertyKey::Symbol(s.clone()),
            _ => PropertyKey::String(value.to_js_string()),
        }
    }

    /// Check if this is a symbol key
    pub fn is_symbol(&self) -> bool {
        matches!(self, PropertyKey::Symbol(_))
    }

    /// Check if this key equals a string literal (avoids allocation)
    #[inline]
    pub fn eq_str(&self, s: &str) -> bool {
        match self {
            PropertyKey::String(js_str) => js_str.as_str() == s,
            PropertyKey::Index(_) | PropertyKey::Symbol(_) => false,
        }
    }
}

// FIXME: remove it as it overused, we should  explicitly create PropertyKey from interned strings
impl From<&str> for PropertyKey {
    #[inline]
    fn from(s: &str) -> Self {
        // Fast path: check first char is a digit before parsing
        if let Some(first) = s.bytes().next() {
            if first.is_ascii_digit() {
                if let Ok(idx) = s.parse::<u32>() {
                    // Verify it's canonical (no leading zeros except "0")
                    if idx.to_string() == s {
                        return PropertyKey::Index(idx);
                    }
                }
            }
        }
        PropertyKey::String(JsString::from(s))
    }
}

// FIXME: remove it as it overused, we should  explicitly create PropertyKey from interned strings
impl From<String> for PropertyKey {
    fn from(s: String) -> Self {
        PropertyKey::from(s.as_str())
    }
}

// FIXME: remove it as it overused, we should  explicitly create PropertyKey from interned strings
impl From<JsString> for PropertyKey {
    #[inline]
    fn from(s: JsString) -> Self {
        // Fast path: check first char is a digit before parsing
        if let Some(first) = s.as_str().bytes().next() {
            if first.is_ascii_digit() {
                if let Ok(idx) = s.parse::<u32>() {
                    // Verify it's canonical (no leading zeros except "0")
                    if idx.to_string() == s.as_str() {
                        return PropertyKey::Index(idx);
                    }
                }
            }
        }
        PropertyKey::String(s)
    }
}

// FIXME: remove it as it overused
impl From<u32> for PropertyKey {
    fn from(idx: u32) -> Self {
        PropertyKey::Index(idx)
    }
}

impl fmt::Display for PropertyKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PropertyKey::String(s) => write!(f, "{}", s),
            PropertyKey::Index(i) => write!(f, "{}", i),
            PropertyKey::Symbol(s) => match &s.description {
                Some(desc) => write!(f, "Symbol({})", desc.as_str()),
                None => write!(f, "Symbol()"),
            },
        }
    }
}

/// Property attribute flags (packed into a single byte)
mod property_flags {
    pub const WRITABLE: u8 = 0b001;
    pub const ENUMERABLE: u8 = 0b010;
    pub const CONFIGURABLE: u8 = 0b100;
    pub const ALL: u8 = WRITABLE | ENUMERABLE | CONFIGURABLE;
}

/// Accessor functions (getter and/or setter) - boxed to save space for data properties
#[derive(Debug, Clone)]
pub struct Accessor {
    pub getter: Option<JsObjectRef>,
    pub setter: Option<JsObjectRef>,
}

/// A property descriptor - optimized for size.
/// Most properties are simple data properties, so we optimize for that case.
#[derive(Debug, Clone)]
pub struct Property {
    pub value: JsValue,
    /// Packed flags: bit 0 = writable, bit 1 = enumerable, bit 2 = configurable
    flags: u8,
    /// Accessor functions (boxed, rarely used) - None for data properties
    accessor: Option<Box<Accessor>>,
}

impl Property {
    /// Create a data property with default attributes (writable, enumerable, configurable)
    #[inline]
    pub fn data(value: JsValue) -> Self {
        Self {
            value,
            flags: property_flags::ALL,
            accessor: None,
        }
    }

    /// Create a read-only data property (enumerable, configurable, but not writable)
    #[inline]
    pub fn data_readonly(value: JsValue) -> Self {
        Self {
            value,
            flags: property_flags::ENUMERABLE | property_flags::CONFIGURABLE,
            accessor: None,
        }
    }

    /// Create an accessor property with getter and/or setter
    pub fn accessor(getter: Option<JsObjectRef>, setter: Option<JsObjectRef>) -> Self {
        Self {
            value: JsValue::Undefined,
            flags: property_flags::ENUMERABLE | property_flags::CONFIGURABLE,
            accessor: Some(Box::new(Accessor { getter, setter })),
        }
    }

    /// Check if this is an accessor property (has getter or setter)
    #[inline]
    pub fn is_accessor(&self) -> bool {
        self.accessor.is_some()
    }

    /// Create a property with custom attributes
    #[inline]
    pub fn with_attributes(
        value: JsValue,
        writable: bool,
        enumerable: bool,
        configurable: bool,
    ) -> Self {
        let mut flags = 0;
        if writable {
            flags |= property_flags::WRITABLE;
        }
        if enumerable {
            flags |= property_flags::ENUMERABLE;
        }
        if configurable {
            flags |= property_flags::CONFIGURABLE;
        }
        Self {
            value,
            flags,
            accessor: None,
        }
    }

    // Attribute getters
    #[inline]
    pub fn writable(&self) -> bool {
        (self.flags & property_flags::WRITABLE) != 0
    }

    #[inline]
    pub fn enumerable(&self) -> bool {
        (self.flags & property_flags::ENUMERABLE) != 0
    }

    #[inline]
    pub fn configurable(&self) -> bool {
        (self.flags & property_flags::CONFIGURABLE) != 0
    }

    // Attribute setters
    #[inline]
    pub fn set_writable(&mut self, writable: bool) {
        if writable {
            self.flags |= property_flags::WRITABLE;
        } else {
            self.flags &= !property_flags::WRITABLE;
        }
    }

    #[inline]
    pub fn set_enumerable(&mut self, enumerable: bool) {
        if enumerable {
            self.flags |= property_flags::ENUMERABLE;
        } else {
            self.flags &= !property_flags::ENUMERABLE;
        }
    }

    #[inline]
    pub fn set_configurable(&mut self, configurable: bool) {
        if configurable {
            self.flags |= property_flags::CONFIGURABLE;
        } else {
            self.flags &= !property_flags::CONFIGURABLE;
        }
    }

    /// Get the getter function (if this is an accessor property)
    #[inline]
    pub fn getter(&self) -> Option<&JsObjectRef> {
        self.accessor.as_ref().and_then(|a| a.getter.as_ref())
    }

    /// Get the setter function (if this is an accessor property)
    #[inline]
    pub fn setter(&self) -> Option<&JsObjectRef> {
        self.accessor.as_ref().and_then(|a| a.setter.as_ref())
    }

    /// Set the getter function
    pub fn set_getter(&mut self, getter: Option<JsObjectRef>) {
        if let Some(ref mut acc) = self.accessor {
            acc.getter = getter;
        } else if getter.is_some() {
            self.accessor = Some(Box::new(Accessor {
                getter,
                setter: None,
            }));
        }
    }

    /// Set the setter function
    pub fn set_setter(&mut self, setter: Option<JsObjectRef>) {
        if let Some(ref mut acc) = self.accessor {
            acc.setter = setter;
        } else if setter.is_some() {
            self.accessor = Some(Box::new(Accessor {
                getter: None,
                setter,
            }));
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Property Storage - optimized for small objects
// ═══════════════════════════════════════════════════════════════════════════════

/// Maximum number of properties stored inline before switching to a HashMap.
/// 2 properties covers most small objects like `{ a, b }` or `{ x: 1, y: 2 }`.
const INLINE_PROPERTY_CAPACITY: usize = 2;

/// Optimized property storage that uses inline storage for small objects.
///
/// Most JavaScript objects have only a few properties. By storing up to 2 properties
/// inline (without heap allocation), we avoid the overhead of a HashMap for common cases.
/// When the object grows beyond 2 properties, we transparently switch to a HashMap.
#[derive(Debug)]
pub enum PropertyStorage {
    /// Inline storage for small objects (≤2 properties).
    /// Uses a fixed-size array with a length counter.
    Inline {
        len: u8,
        entries: [(PropertyKey, Property); INLINE_PROPERTY_CAPACITY],
    },
    /// HashMap storage for larger objects.
    Map(FxHashMap<PropertyKey, Property>),
}

impl Default for PropertyStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl PropertyStorage {
    /// Create empty inline storage.
    #[inline]
    pub fn new() -> Self {
        PropertyStorage::Inline {
            len: 0,
            entries: std::array::from_fn(|_| {
                (PropertyKey::Index(0), Property::data(JsValue::Undefined))
            }),
        }
    }

    /// Create storage with pre-allocated capacity.
    /// If capacity > INLINE_PROPERTY_CAPACITY, creates a HashMap.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        if capacity <= INLINE_PROPERTY_CAPACITY {
            Self::new()
        } else {
            PropertyStorage::Map(FxHashMap::with_capacity_and_hasher(
                capacity,
                Default::default(),
            ))
        }
    }

    /// Reserve capacity. Only meaningful for Map variant.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        if let PropertyStorage::Map(map) = self {
            map.reserve(additional);
        }
        // For Inline, we'll convert to Map when needed during insert
    }

    /// Get a property by key.
    #[inline]
    pub fn get(&self, key: &PropertyKey) -> Option<&Property> {
        match self {
            PropertyStorage::Inline { len, entries } => {
                for entry in entries.get(..(*len as usize)).unwrap_or_default() {
                    if &entry.0 == key {
                        return Some(&entry.1);
                    }
                }
                None
            }
            PropertyStorage::Map(map) => map.get(key),
        }
    }

    /// Get a mutable reference to a property by key.
    #[inline]
    pub fn get_mut(&mut self, key: &PropertyKey) -> Option<&mut Property> {
        match self {
            PropertyStorage::Inline { len, entries } => {
                for entry in entries.get_mut(..(*len as usize)).unwrap_or_default() {
                    if &entry.0 == key {
                        return Some(&mut entry.1);
                    }
                }
                None
            }
            PropertyStorage::Map(map) => map.get_mut(key),
        }
    }

    /// Insert or update a property. Returns the old value if the key existed.
    pub fn insert(&mut self, key: PropertyKey, value: Property) -> Option<Property> {
        match self {
            PropertyStorage::Inline { len, entries } => {
                let current_len = *len as usize;

                // Check if key already exists
                for entry in entries.get_mut(..current_len).unwrap_or_default() {
                    if entry.0 == key {
                        let old = std::mem::replace(&mut entry.1, value);
                        return Some(old);
                    }
                }

                // Key doesn't exist - try to add inline
                if let Some(slot) = entries.get_mut(current_len) {
                    *slot = (key, value);
                    *len += 1;
                    return None;
                }

                // Need to convert to Map (current_len == INLINE_PROPERTY_CAPACITY)
                let mut map = FxHashMap::with_capacity_and_hasher(
                    INLINE_PROPERTY_CAPACITY + 1,
                    Default::default(),
                );
                for entry in entries.iter_mut() {
                    let (k, v) = std::mem::replace(
                        entry,
                        (PropertyKey::Index(0), Property::data(JsValue::Undefined)),
                    );
                    map.insert(k, v);
                }
                map.insert(key, value);
                *self = PropertyStorage::Map(map);
                None
            }
            PropertyStorage::Map(map) => map.insert(key, value),
        }
    }

    /// Check if a key exists.
    #[inline]
    pub fn contains_key(&self, key: &PropertyKey) -> bool {
        match self {
            PropertyStorage::Inline { len, entries } => {
                for entry in entries.get(..(*len as usize)).unwrap_or_default() {
                    if &entry.0 == key {
                        return true;
                    }
                }
                false
            }
            PropertyStorage::Map(map) => map.contains_key(key),
        }
    }

    /// Remove a property by key. Returns the removed value if it existed.
    pub fn remove(&mut self, key: &PropertyKey) -> Option<Property> {
        match self {
            PropertyStorage::Inline { len, entries } => {
                let current_len = *len as usize;
                let mut found_idx = None;
                for (i, entry) in entries
                    .get(..current_len)
                    .unwrap_or_default()
                    .iter()
                    .enumerate()
                {
                    if &entry.0 == key {
                        found_idx = Some(i);
                        break;
                    }
                }
                if let Some(i) = found_idx {
                    // Swap with last element and decrement len
                    let removed = if let Some(entry) = entries.get_mut(i) {
                        std::mem::replace(
                            entry,
                            (PropertyKey::Index(0), Property::data(JsValue::Undefined)),
                        )
                    } else {
                        return None;
                    };
                    if i < current_len - 1 {
                        entries.swap(i, current_len - 1);
                    }
                    *len -= 1;
                    Some(removed.1)
                } else {
                    None
                }
            }
            PropertyStorage::Map(map) => map.remove(key),
        }
    }

    /// Clear all properties.
    #[inline]
    pub fn clear(&mut self) {
        match self {
            PropertyStorage::Inline { len, entries } => {
                // Reset entries to avoid holding references
                for entry in entries.get_mut(..(*len as usize)).unwrap_or_default() {
                    *entry = (PropertyKey::Index(0), Property::data(JsValue::Undefined));
                }
                *len = 0;
            }
            PropertyStorage::Map(map) => map.clear(),
        }
    }

    /// Get the number of properties.
    #[inline]
    pub fn len(&self) -> usize {
        match self {
            PropertyStorage::Inline { len, .. } => *len as usize,
            PropertyStorage::Map(map) => map.len(),
        }
    }

    /// Check if empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Iterate over all (key, value) pairs.
    pub fn iter(&self) -> PropertyStorageIter<'_> {
        match self {
            PropertyStorage::Inline { len, entries } => PropertyStorageIter::Inline {
                entries,
                index: 0,
                len: *len as usize,
            },
            PropertyStorage::Map(map) => PropertyStorageIter::Map(map.iter()),
        }
    }

    /// Iterate over all (key, value) pairs mutably.
    pub fn iter_mut(&mut self) -> PropertyStorageIterMut<'_> {
        match self {
            PropertyStorage::Inline { len, entries } => {
                let len = *len as usize;
                PropertyStorageIterMut::Inline {
                    entries: entries.get_mut(..len).unwrap_or_default(),
                }
            }
            PropertyStorage::Map(map) => PropertyStorageIterMut::Map(map.iter_mut()),
        }
    }

    /// Iterate over all keys.
    pub fn keys(&self) -> impl Iterator<Item = &PropertyKey> {
        self.iter().map(|(k, _)| k)
    }

    /// Iterate over all values.
    pub fn values(&self) -> impl Iterator<Item = &Property> {
        self.iter().map(|(_, v)| v)
    }
}

/// Iterator over PropertyStorage entries.
pub enum PropertyStorageIter<'a> {
    Inline {
        entries: &'a [(PropertyKey, Property); INLINE_PROPERTY_CAPACITY],
        index: usize,
        len: usize,
    },
    Map(std::collections::hash_map::Iter<'a, PropertyKey, Property>),
}

impl<'a> Iterator for PropertyStorageIter<'a> {
    type Item = (&'a PropertyKey, &'a Property);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            PropertyStorageIter::Inline {
                entries,
                index,
                len,
            } => {
                if *index < *len {
                    let i = *index;
                    *index += 1;
                    entries.get(i).map(|e| (&e.0, &e.1))
                } else {
                    None
                }
            }
            PropertyStorageIter::Map(iter) => iter.next(),
        }
    }
}

/// Mutable iterator over PropertyStorage entries.
pub enum PropertyStorageIterMut<'a> {
    Inline {
        entries: &'a mut [(PropertyKey, Property)],
    },
    Map(std::collections::hash_map::IterMut<'a, PropertyKey, Property>),
}

impl<'a> Iterator for PropertyStorageIterMut<'a> {
    type Item = (&'a PropertyKey, &'a mut Property);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            PropertyStorageIterMut::Inline { entries } => {
                // Take the slice and split off the first element
                if entries.is_empty() {
                    None
                } else {
                    // Split the slice: take first element, keep the rest
                    let (first, rest) = std::mem::take(entries).split_at_mut(1);
                    *entries = rest;
                    first.first_mut().map(|e| (&e.0, &mut e.1))
                }
            }
            PropertyStorageIterMut::Map(iter) => iter.next(),
        }
    }
}

/// Environment data stored in Environment exotic objects.
///
/// This is used to store variable bindings in the GC-managed object graph,
/// allowing the GC to trace and collect environments that form cycles.
#[derive(Debug)]
pub struct EnvironmentData {
    /// Variable bindings in this scope
    /// Uses VarKey for O(1) pointer-based lookups (all var names are interned)
    pub bindings: FxHashMap<VarKey, Binding>,
    /// Parent environment (if any) - now a GC reference
    pub outer: Option<JsObjectRef>,
}

impl EnvironmentData {
    /// Create a new environment with no outer scope (for global environment)
    pub fn new() -> Self {
        Self {
            bindings: FxHashMap::default(),
            outer: None,
        }
    }

    /// Create a new environment with the given outer environment as parent
    pub fn with_outer(outer: Option<JsObjectRef>) -> Self {
        Self {
            bindings: FxHashMap::default(),
            outer,
        }
    }

    /// Create a new environment with the given outer environment and pre-allocated capacity
    /// for bindings. This reduces HashMap resizing during function execution.
    pub fn with_outer_and_capacity(outer: Option<JsObjectRef>, capacity: usize) -> Self {
        Self {
            bindings: FxHashMap::with_capacity_and_hasher(capacity, Default::default()),
            outer,
        }
    }
}

impl Default for EnvironmentData {
    fn default() -> Self {
        Self::new()
    }
}

/// Exotic object behavior
#[derive(Debug)]
pub enum ExoticObject {
    /// Ordinary object
    Ordinary,
    /// Array exotic object - stores elements directly for O(1) indexed access
    Array { elements: Vec<JsValue> },
    /// Boolean wrapper object - stores primitive boolean value
    Boolean(bool),
    /// Number wrapper object - stores primitive number value
    Number(f64),
    /// String wrapper object - stores primitive string value
    StringObj(JsString),
    /// Function exotic object
    Function(JsFunction),
    /// Map exotic object - stores key-value pairs preserving insertion order
    // FIXME: use a proper hash map
    Map { entries: Vec<(JsValue, JsValue)> },
    /// Set exotic object - stores unique values preserving insertion order
    // FIXME: use a proper set
    Set { entries: Vec<JsValue> },
    /// Date exotic object - stores timestamp in milliseconds since Unix epoch
    Date { timestamp: f64 },
    /// RegExp exotic object - stores pattern and flags
    // FIXME: use JsStrings
    RegExp { pattern: String, flags: String },
    /// Generator exotic object - stores generator state (AST-based)
    Generator(Rc<RefCell<GeneratorState>>),
    /// Bytecode generator exotic object - stores bytecode generator state
    BytecodeGenerator(Rc<RefCell<BytecodeGeneratorState>>),
    /// Promise exotic object - stores promise state
    Promise(Rc<RefCell<PromiseState>>),
    /// Environment exotic object - stores variable bindings
    Environment(EnvironmentData),
    /// Enum exotic object - stores enum metadata
    Enum(EnumData),
    /// Proxy exotic object - wraps target with handler traps
    Proxy(ProxyData),
    /// Raw JSON exotic object - stores a JSON string for literal insertion in JSON.stringify
    RawJSON(JsString),
}

/// Proxy internal state
///
/// Stores the target object and handler object for the proxy.
/// If revoked is true, all operations on the proxy will throw TypeError.
#[derive(Debug, Clone)]
pub struct ProxyData {
    /// The wrapped target object
    pub target: JsObjectRef,
    /// The handler object containing traps
    pub handler: JsObjectRef,
    /// Whether this proxy has been revoked
    pub revoked: bool,
}

/// Enum member - stores name and value
#[derive(Debug, Clone)]
pub struct EnumMember {
    /// Member name (e.g., "Up", "Down")
    pub name: JsString,
    /// Member value (number or string)
    pub value: JsValue,
}

/// Enum internal state
///
/// Stores enum members directly for efficient access.
/// Forward mappings (name → value) and reverse mappings (numeric value → name)
/// are computed from the members list.
#[derive(Debug, Clone)]
pub struct EnumData {
    /// Enum name (for debugging/toString)
    pub name: JsString,
    /// Whether this is a const enum
    pub const_: bool,
    /// Enum members in declaration order
    pub members: Vec<EnumMember>,
}

impl EnumData {
    /// Get value by member name (forward mapping)
    pub fn get_by_name(&self, name: &str) -> Option<JsValue> {
        self.members
            .iter()
            .find(|m| m.name.as_str() == name)
            .map(|m| m.value.clone())
    }

    /// Get member name by numeric value (reverse mapping)
    /// Only works for numeric values, returns None for string values
    pub fn get_by_value(&self, value: f64) -> Option<JsString> {
        self.members.iter().find_map(|m| {
            if let JsValue::Number(n) = &m.value {
                if *n == value {
                    return Some(m.name.cheap_clone());
                }
            }
            None
        })
    }

    /// Get all property keys (member names + reverse mapping keys for numeric values)
    pub fn keys(&self) -> Vec<PropertyKey> {
        let mut keys = Vec::with_capacity(self.members.len() * 2);

        for member in &self.members {
            // Forward mapping key (member name)
            keys.push(PropertyKey::String(member.name.cheap_clone()));

            // Reverse mapping key for numeric values
            if let JsValue::Number(_) = &member.value {
                keys.push(PropertyKey::from_value(&member.value));
            }
        }

        keys
    }

    /// Get all values (member values + reverse mapping values)
    pub fn values(&self) -> Vec<JsValue> {
        let mut values = Vec::with_capacity(self.members.len() * 2);

        for member in &self.members {
            // Forward mapping value
            values.push(member.value.clone());

            // Reverse mapping value for numeric values (the member name)
            if let JsValue::Number(_) = &member.value {
                values.push(JsValue::String(member.name.cheap_clone()));
            }
        }

        values
    }

    /// Get all entries as (key_string, value) pairs for Object.entries
    pub fn entries(&self) -> Vec<(String, JsValue)> {
        let mut entries = Vec::with_capacity(self.members.len() * 2);

        for member in &self.members {
            // Forward mapping entry (member name -> value)
            entries.push((member.name.to_string(), member.value.clone()));

            // Reverse mapping entry for numeric values (value string -> name)
            if let JsValue::Number(n) = &member.value {
                entries.push((n.to_string(), JsValue::String(member.name.cheap_clone())));
            }
        }

        entries
    }

    /// Check if the enum has a property with the given key
    pub fn has_property(&self, key: &PropertyKey) -> bool {
        match key {
            PropertyKey::String(s) => {
                // Check forward mapping
                if self.members.iter().any(|m| m.name.as_str() == s.as_str()) {
                    return true;
                }
                // Check reverse mapping for numeric string keys
                if let Ok(n) = s.as_str().parse::<f64>() {
                    return self.get_by_value(n).is_some();
                }
                false
            }
            PropertyKey::Index(idx) => self.get_by_value(*idx as f64).is_some(),
            PropertyKey::Symbol(_) => false,
        }
    }

    /// Set a member value by name (for mutability support)
    /// Returns true if the member was found and updated
    pub fn set_by_name(&mut self, name: &str, value: JsValue) -> bool {
        if let Some(member) = self.members.iter_mut().find(|m| m.name.as_str() == name) {
            member.value = value;
            true
        } else {
            false
        }
    }
}

/// Promise internal state
#[derive(Debug, Clone)]
pub struct PromiseState {
    /// Current state of the promise
    pub status: PromiseStatus,
    /// Resolved value or rejection reason
    pub result: Option<JsValue>,
    /// Handlers to call when promise settles
    pub handlers: Vec<PromiseHandler>,
}

/// Promise status
#[derive(Debug, Clone, PartialEq)]
pub enum PromiseStatus {
    /// Promise is pending
    Pending,
    /// Promise is fulfilled
    Fulfilled,
    /// Promise is rejected
    Rejected,
}

/// Handler attached via .then()/.catch()
#[derive(Debug, Clone)]
pub struct PromiseHandler {
    /// Callback for fulfilled state
    pub on_fulfilled: Option<JsValue>,
    /// Callback for rejected state
    pub on_rejected: Option<JsValue>,
    /// The promise returned by .then()/.catch()
    pub result_promise: JsObjectRef,
}

/// Saved frame state for generators
/// This stores the frames and values that need to be restored when resuming
#[derive(Debug, Clone)]
pub struct SavedFrameState {
    /// Serialized frame data - stored as a clonable representation
    pub frame_data: Vec<u8>,
    /// Number of frames
    pub frame_count: usize,
}

/// Generator state for suspended generators
///
/// Note: This struct intentionally does NOT derive Clone because it may hold
/// a saved ExecutionState which contains Guards that cannot be cloned.
/// The struct is always wrapped in Rc<RefCell<>> for shared access.
pub struct GeneratorState {
    /// The generator function's body (Rc for cheap cloning)
    pub body: Rc<BlockStatement>,
    /// Parameters of the generator function (Rc for cheap cloning)
    pub params: Rc<[FunctionParam]>,
    /// Arguments passed to the generator
    pub args: Vec<JsValue>,
    /// The captured closure environment (GC-managed)
    pub closure: JsObjectRef,
    /// Current execution status
    pub status: GeneratorStatus,
    /// Value passed in via next(value)
    pub sent_value: JsValue,
    /// Function name for debugging
    pub name: Option<JsString>,
    /// Unique ID for this generator (used to look up saved execution state)
    pub id: u64,
    /// Whether this generator has started execution
    pub started: bool,
}

impl std::fmt::Debug for GeneratorState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GeneratorState")
            .field("status", &self.status)
            .field("id", &self.id)
            .field("started", &self.started)
            .finish()
    }
}

/// Status of generator execution
#[derive(Debug, Clone, PartialEq)]
pub enum GeneratorStatus {
    /// Not yet started
    Suspended,
    /// Completed (returned or exhausted)
    Completed,
}

/// Generator state for bytecode-based generators
///
/// This stores the bytecode chunk and VM state needed to resume execution.
pub struct BytecodeGeneratorState {
    /// The bytecode chunk for the generator function
    pub chunk: Rc<crate::compiler::BytecodeChunk>,
    /// The captured closure environment
    pub closure: JsObjectRef,
    /// Arguments passed to the generator function
    pub args: Vec<JsValue>,
    /// The `this` value passed when the generator function was called
    pub this_value: JsValue,
    /// Current execution status
    pub status: GeneratorStatus,
    /// Value passed in via next(value)
    pub sent_value: JsValue,
    /// Unique ID for this generator
    pub id: u64,
    /// Whether this generator has started execution
    pub started: bool,
    /// Saved instruction pointer (for resumption)
    pub saved_ip: usize,
    /// Saved register values (for resumption)
    pub saved_registers: Vec<JsValue>,
    /// Saved call stack (for resumption)
    pub saved_call_stack: Vec<crate::interpreter::bytecode_vm::CallFrame>,
    /// Saved try stack (for resumption)
    pub saved_try_stack: Vec<crate::interpreter::bytecode_vm::TryHandler>,
    /// Register to store the result of yield
    pub yield_result_register: Option<u8>,
    /// The function environment (created on first call, reused on subsequent calls)
    pub func_env: Option<JsObjectRef>,
    /// The current environment at yield time (may be nested within func_env)
    pub current_env: Option<JsObjectRef>,
    /// Delegated iterator for yield* (iterator object and its next method)
    pub delegated_iterator: Option<(JsObjectRef, JsValue)>,
    /// Whether this is an async generator (next() returns Promise)
    pub is_async: bool,
    /// Exception to throw when resuming (for generator.throw())
    pub throw_value: Option<JsValue>,
}

impl std::fmt::Debug for BytecodeGeneratorState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BytecodeGeneratorState")
            .field("status", &self.status)
            .field("id", &self.id)
            .field("started", &self.started)
            .field("saved_ip", &self.saved_ip)
            .finish()
    }
}

/// Function representation
#[derive(Debug, Clone)]
pub enum JsFunction {
    /// Bytecode function (compiled from source)
    Bytecode(BytecodeFunction),
    /// Bytecode generator function (creates a generator when called)
    BytecodeGenerator(BytecodeFunction),
    /// Bytecode async function (returns Promise when called)
    BytecodeAsync(BytecodeFunction),
    /// Bytecode async generator function (creates async generator when called)
    BytecodeAsyncGenerator(BytecodeFunction),
    /// Native Rust function
    Native(NativeFunction),
    /// Bound function (created by Function.prototype.bind)
    Bound(Box<BoundFunctionData>),
    /// Promise resolve function (has internal [[Promise]] slot)
    PromiseResolve(JsObjectRef),
    /// Promise reject function (has internal [[Promise]] slot)
    PromiseReject(JsObjectRef),
    /// Promise.all fulfill handler (tracks shared state for aggregation)
    /// Contains (shared_state, index) - index is which promise slot to fill
    PromiseAllFulfill {
        state: std::rc::Rc<PromiseAllSharedState>,
        index: usize,
    },
    /// Promise.all reject handler (rejects on first failure)
    PromiseAllReject(std::rc::Rc<PromiseAllSharedState>),
    /// Auto-accessor getter (metadata stored in object properties)
    AccessorGetter,
    /// Auto-accessor setter (metadata stored in object properties)
    AccessorSetter,
    /// Module export getter for live bindings
    /// Contains (module_environment, binding_name)
    ModuleExportGetter {
        module_env: JsObjectRef,
        binding_name: JsString,
    },
    /// Re-export getter for live bindings through re-exports
    /// Delegates to another module's namespace object property
    ModuleReExportGetter {
        source_module: JsObjectRef,
        source_key: PropertyKey,
    },
    /// Proxy revoke function - revokes the associated proxy
    /// Contains the proxy object reference
    ProxyRevoke(JsObjectRef),
}

/// Shared state for Promise.all tracking
/// This is shared across all handlers via Rc
#[derive(Debug)]
pub struct PromiseAllSharedState {
    /// Number of promises still pending
    pub remaining: std::cell::Cell<usize>,
    /// Results array (indexed by original position)
    pub results: std::cell::RefCell<Vec<JsValue>>,
    /// The result promise to fulfill when all complete
    pub result_promise: JsObjectRef,
    /// Whether any promise has already rejected
    pub rejected: std::cell::Cell<bool>,
}

/// Data for a bound function
#[derive(Debug, Clone)]
pub struct BoundFunctionData {
    /// The target function to call
    pub target: JsObjectRef,
    /// The bound this value
    pub this_arg: JsValue,
    /// Pre-filled arguments
    pub bound_args: Vec<JsValue>,
}

impl JsFunction {
    pub fn name(&self) -> Option<&str> {
        match self {
            JsFunction::Bytecode(f)
            | JsFunction::BytecodeGenerator(f)
            | JsFunction::BytecodeAsync(f)
            | JsFunction::BytecodeAsyncGenerator(f) => f
                .chunk
                .function_info
                .as_ref()
                .and_then(|info| info.name.as_ref())
                .map(|s| s.as_str()),
            JsFunction::Native(f) => Some(f.name.as_ref()),
            JsFunction::Bound(_) => Some("bound"),
            JsFunction::PromiseResolve(_) => Some("resolve"),
            JsFunction::PromiseReject(_) => Some("reject"),
            JsFunction::PromiseAllFulfill { .. } => Some("promiseAllFulfill"),
            JsFunction::PromiseAllReject(_) => Some("promiseAllReject"),
            JsFunction::AccessorGetter => Some("get"),
            JsFunction::AccessorSetter => Some("set"),
            JsFunction::ModuleExportGetter { .. } => Some("get"),
            JsFunction::ModuleReExportGetter { .. } => Some("get"),
            JsFunction::ProxyRevoke(_) => Some("revoke"),
        }
    }
}

/// Bytecode-compiled function
#[derive(Debug, Clone)]
pub struct BytecodeFunction {
    /// The compiled bytecode chunk
    pub chunk: Rc<crate::compiler::BytecodeChunk>,
    /// The captured closure environment (GC-managed)
    pub closure: JsObjectRef,
    /// Captured `this` value for arrow functions (None for regular functions)
    pub captured_this: Option<Box<JsValue>>,
}

/// Function body (block or expression for arrow functions)
// FIXME: do not need to wrap in rc
#[derive(Debug, Clone)]
pub enum FunctionBody {
    Block(Rc<BlockStatement>),
    Expression(Rc<Expression>),
}

impl From<Rc<ArrowFunctionBody>> for FunctionBody {
    fn from(body: Rc<ArrowFunctionBody>) -> Self {
        match body.as_ref() {
            ArrowFunctionBody::Block(block) => FunctionBody::Block(block.clone()),
            ArrowFunctionBody::Expression(expr) => FunctionBody::Expression(expr.clone()),
        }
    }
}

/// Native function signature type
/// Returns Guarded to keep newly created objects alive until ownership is transferred.
pub type NativeFn =
    fn(&mut crate::interpreter::Interpreter, JsValue, &[JsValue]) -> Result<Guarded, JsError>;

/// Native function wrapper
#[derive(Clone)]
pub struct NativeFunction {
    pub name: JsString,
    pub func: NativeFn,
    pub arity: usize,
}

impl fmt::Debug for NativeFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeFunction")
            .field("name", &self.name)
            .field("arity", &self.arity)
            .finish()
    }
}

/// Variable binding
#[derive(Debug, Clone)]
pub struct Binding {
    pub value: JsValue,
    pub mutable: bool,
    pub initialized: bool,
    /// For import bindings: reference to module object and property key for live bindings.
    /// When set, `value` is ignored and the actual value is read from the module object.
    pub import_binding: Option<ImportBinding>,
}

/// Reference to a module export for live bindings
#[derive(Debug, Clone)]
pub struct ImportBinding {
    pub module_obj: JsObjectRef,
    pub property_key: PropertyKey,
}

/// Represents a module export entry - either a direct export from this module's
/// environment, or a re-export from another module.
#[derive(Debug, Clone)]
pub enum ModuleExport {
    /// Direct export: the binding exists in this module's environment
    /// The value is stored but the actual live value comes from the environment
    Direct { name: JsString, value: JsValue },
    /// Re-export: delegates to another module's export
    /// Contains the source module namespace object and the property key
    ReExport {
        source_module: JsObjectRef,
        source_key: PropertyKey,
    },
}

// ═══════════════════════════════════════════════════════════════════════════════
// Environment GC Integration
// ═══════════════════════════════════════════════════════════════════════════════

/// Environment reference - a GC-managed environment object.
///
/// This is an alias for `JsObjectRef` where the object has `ExoticObject::Environment`.
/// Using this type makes it clear when a reference is expected to be an environment.
pub type EnvRef = JsObjectRef;

/// A guarded environment that keeps itself alive via its guard.
///
/// This bundles an environment reference with the guard that keeps it rooted.
/// Used for `self.env` in the interpreter to ensure environments aren't collected
/// while they're the current execution environment.
pub struct GuardedEnv {
    /// The environment object
    pub env: EnvRef,
    /// Guard keeping this environment alive (None for root_guard-allocated envs)
    pub guard: Option<Guard<JsObject>>,
}

impl GuardedEnv {
    /// Create a guarded environment with an explicit guard
    pub fn with_guard(env: EnvRef, guard: Guard<JsObject>) -> Self {
        Self {
            env,
            guard: Some(guard),
        }
    }

    /// Create an unguarded environment (for envs already rooted via root_guard)
    pub fn unguarded(env: EnvRef) -> Self {
        Self { env, guard: None }
    }

    /// Get the environment reference
    pub fn get(&self) -> &EnvRef {
        &self.env
    }

    /// Clone the environment reference (for passing to outer, etc.)
    pub fn clone_ref(&self) -> EnvRef {
        self.env.clone()
    }
}

/// Create a new environment object with a temporary guard.
///
/// The environment is created with an optional outer environment reference.
/// The outer environment holds a reference to the new env via EnvironmentData::outer,
/// which increments ref_count via clone.
/// Returns the environment object. Caller is responsible for ownership transfer.
pub fn create_environment_with_guard(guard: &Guard<JsObject>, outer: Option<EnvRef>) -> EnvRef {
    let env = guard.alloc();
    {
        let mut env_ref = env.borrow_mut();
        env_ref.null_prototype = true;
        // The outer clone (if any) in EnvironmentData automatically increments ref_count
        env_ref.exotic = ExoticObject::Environment(EnvironmentData::with_outer(outer));
    }
    env
}

/// Create a new environment object with its own temporary guard.
///
/// This is used for per-iteration loop environments that should NOT be added to root_guard.
/// The guard is returned so it can be kept alive until the environment is safely stored
/// (e.g., in self.env), after which the guard can be dropped.
///
/// Returns (environment, guard) - caller must keep guard alive until env is owned elsewhere.
pub fn create_environment_unrooted(
    heap: &Heap<JsObject>,
    outer: Option<EnvRef>,
) -> (EnvRef, Guard<JsObject>) {
    let guard = heap.create_guard();
    let env = guard.alloc();
    {
        let mut env_ref = env.borrow_mut();
        env_ref.null_prototype = true;
        env_ref.exotic = ExoticObject::Environment(EnvironmentData::with_outer(outer));
    }
    (env, guard)
}

/// Create a new environment object with pre-allocated capacity for bindings.
///
/// Like `create_environment_unrooted`, but pre-sizes the bindings HashMap to avoid
/// resizing during function execution. Use when the number of bindings is known
/// (e.g., from FunctionInfo::binding_count).
///
/// Returns (environment, guard) - caller must keep guard alive until env is owned elsewhere.
pub fn create_environment_unrooted_with_capacity(
    heap: &Heap<JsObject>,
    outer: Option<EnvRef>,
    capacity: usize,
) -> (EnvRef, Guard<JsObject>) {
    let guard = heap.create_guard();
    let env = guard.alloc();
    {
        let mut env_ref = env.borrow_mut();
        env_ref.null_prototype = true;
        env_ref.exotic =
            ExoticObject::Environment(EnvironmentData::with_outer_and_capacity(outer, capacity));
    }
    (env, guard)
}

/// Create a new guarded environment.
///
/// This creates an environment with its own guard that keeps it alive.
/// Used for creating environments that will be stored in `self.env`.
pub fn create_guarded_env(heap: &Heap<JsObject>, outer: Option<EnvRef>) -> GuardedEnv {
    let (env, guard) = create_environment_unrooted(heap, outer);
    GuardedEnv::with_guard(env, guard)
}

impl JsObject {
    /// Get environment data if this is an environment object
    pub fn as_environment(&self) -> Option<&EnvironmentData> {
        match &self.exotic {
            ExoticObject::Environment(data) => Some(data),
            _ => None,
        }
    }

    /// Get mutable environment data if this is an environment object
    pub fn as_environment_mut(&mut self) -> Option<&mut EnvironmentData> {
        match &mut self.exotic {
            ExoticObject::Environment(data) => Some(data),
            _ => None,
        }
    }

    /// Check if this object is an environment
    pub fn is_environment(&self) -> bool {
        matches!(self.exotic, ExoticObject::Environment(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_boolean() {
        assert!(!JsValue::Undefined.to_boolean());
        assert!(!JsValue::Null.to_boolean());
        assert!(!JsValue::Boolean(false).to_boolean());
        assert!(JsValue::Boolean(true).to_boolean());
        assert!(!JsValue::Number(0.0).to_boolean());
        assert!(JsValue::Number(1.0).to_boolean());
        assert!(!JsValue::Number(f64::NAN).to_boolean());
        assert!(!JsValue::String(JsString::from("")).to_boolean());
        assert!(JsValue::String(JsString::from("hello")).to_boolean());
    }

    #[test]
    fn test_to_number() {
        assert!(JsValue::Undefined.to_number().is_nan());
        assert_eq!(JsValue::Null.to_number(), 0.0);
        assert_eq!(JsValue::Boolean(true).to_number(), 1.0);
        assert_eq!(JsValue::Boolean(false).to_number(), 0.0);
        assert_eq!(JsValue::Number(42.0).to_number(), 42.0);
        assert_eq!(JsValue::String(JsString::from("42")).to_number(), 42.0);
        assert!(JsValue::String(JsString::from("hello"))
            .to_number()
            .is_nan());
    }

    #[test]
    fn test_strict_equals() {
        assert!(JsValue::Undefined.strict_equals(&JsValue::Undefined));
        assert!(JsValue::Null.strict_equals(&JsValue::Null));
        assert!(!JsValue::Undefined.strict_equals(&JsValue::Null));
        assert!(JsValue::Number(1.0).strict_equals(&JsValue::Number(1.0)));
        assert!(!JsValue::Number(f64::NAN).strict_equals(&JsValue::Number(f64::NAN)));
    }

    #[test]
    fn test_to_js_string_number_canonical() {
        // JavaScript uses canonical string representations for numbers
        // Very small numbers use exponential notation
        assert_eq!(
            JsValue::Number(0.0000001).to_js_string().as_str(),
            "1e-7",
            "0.0000001 should be '1e-7'"
        );

        // Normal decimals
        assert_eq!(
            JsValue::Number(0.1).to_js_string().as_str(),
            "0.1",
            "0.1 should be '0.1'"
        );

        // Very large numbers use exponential notation
        assert_eq!(
            JsValue::Number(1e21).to_js_string().as_str(),
            "1e+21",
            "1e21 should be '1e+21'"
        );
    }

    #[test]
    fn test_property_key_from_decimal() {
        // PropertyKey::from_value should use canonical string representation
        let key = PropertyKey::from_value(&JsValue::Number(0.1));
        match key {
            PropertyKey::String(s) => assert_eq!(s.as_str(), "0.1"),
            PropertyKey::Index(_) => panic!("0.1 should not be an index"),
            PropertyKey::Symbol(_) => panic!("0.1 should not be a symbol"),
        }
    }

    #[test]
    fn test_rust_parse_leading_decimal() {
        // Check what Rust does with ".1"
        let parsed: f64 = ".1".parse().unwrap();
        println!("Parsed .1 as: {}", parsed);
        assert_eq!(parsed, 0.1);
    }

    #[test]
    fn test_property_key_from_value_decimal_debug() {
        let n = 0.1_f64;
        println!("n = {}", n);
        let key = PropertyKey::from_value(&JsValue::Number(n));
        println!("PropertyKey = {:?}", key);
        println!("PropertyKey display = {}", key);
        match key {
            PropertyKey::String(s) => {
                println!("It's a String: '{}'", s);
                assert_eq!(s.as_str(), "0.1");
            }
            PropertyKey::Index(i) => {
                println!("It's an Index: {}", i);
                panic!("0.1 should not be an index!");
            }
            PropertyKey::Symbol(_) => panic!("0.1 should not be a symbol"),
        }
    }
}
