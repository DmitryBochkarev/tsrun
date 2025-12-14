//! JavaScript value representation
//!
//! The core JsValue type and related structures for representing JavaScript values at runtime.

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use rustc_hash::FxHashMap;

use crate::ast::{ArrowFunctionBody, BlockStatement, FunctionParam};
use crate::gc::{Gc, Guard, Heap, Reset};
use crate::lexer::Span;
use crate::string_dict::StringDict;

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

/// A JavaScript value
#[derive(Clone, Default)]
pub enum JsValue {
    #[default]
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(JsString),
    Symbol(JsSymbol),
    Object(JsObjectRef),
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

    /// Get the typeof result for this value
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
            JsValue::String(s) => s.parse::<f64>().unwrap_or(f64::NAN),
            JsValue::Symbol(_) => f64::NAN, // Cannot convert Symbol to number
            JsValue::Object(_) => {
                // Would need ToPrimitive then ToNumber
                f64::NAN
            }
        }
    }

    /// Convert to string (ToString)
    pub fn to_js_string(&self) -> JsString {
        match self {
            JsValue::Undefined => JsString::from("undefined"),
            JsValue::Null => JsString::from("null"),
            JsValue::Boolean(true) => JsString::from("true"),
            JsValue::Boolean(false) => JsString::from("false"),
            JsValue::Number(n) => {
                if n.is_nan() {
                    JsString::from("NaN")
                } else if n.is_infinite() {
                    if *n > 0.0 {
                        JsString::from("Infinity")
                    } else {
                        JsString::from("-Infinity")
                    }
                } else if *n == 0.0 {
                    JsString::from("0")
                } else {
                    JsString::from(n.to_string())
                }
            }
            JsValue::String(s) => s.clone(),
            JsValue::Symbol(s) => {
                // Symbol.prototype.toString returns "Symbol(description)"
                match &s.description {
                    Some(desc) => JsString::from(format!("Symbol({})", desc)),
                    None => JsString::from("Symbol()"),
                }
            }
            JsValue::Object(_) => JsString::from("[object Object]"),
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
                Some(desc) => write!(f, "Symbol({})", desc),
                None => write!(f, "Symbol()"),
            },
            JsValue::Object(obj) => {
                let obj = obj.borrow();
                match &obj.exotic {
                    ExoticObject::Ordinary => write!(f, "{{...}}"),
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
    pub description: Option<String>,
}

impl JsSymbol {
    /// Create a new unique symbol with an optional description
    pub fn new(id: u64, description: Option<String>) -> Self {
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
impl<T> CheapClone for Gc<T> {}

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
        self.properties.clear();
        self.exotic = ExoticObject::Ordinary;
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
    /// Object properties
    pub properties: FxHashMap<PropertyKey, Property>,
    /// Exotic object behavior
    pub exotic: ExoticObject,
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
            properties: FxHashMap::default(),
            exotic: ExoticObject::Ordinary,
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
            properties: FxHashMap::with_capacity_and_hasher(capacity, Default::default()),
            exotic: ExoticObject::Ordinary,
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
            properties: FxHashMap::default(),
            exotic: ExoticObject::Ordinary,
        }
    }

    /// Check if this object is callable
    pub fn is_callable(&self) -> bool {
        matches!(self.exotic, ExoticObject::Function(_))
    }

    /// Get an own property
    pub fn get_own_property(&self, key: &PropertyKey) -> Option<&Property> {
        self.properties.get(key)
    }

    /// Get a property, searching the prototype chain
    pub fn get_property(&self, key: &PropertyKey) -> Option<JsValue> {
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
        if let Some(prop) = self.properties.get_mut(&key) {
            // Only set if writable
            if prop.writable {
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
}

impl Default for JsObject {
    fn default() -> Self {
        Self::new()
    }
}

/// Property key (string, index, or symbol)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PropertyKey {
    String(JsString),
    Index(u32),
    Symbol(JsSymbol),
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
                PropertyKey::String(s.clone())
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

impl From<String> for PropertyKey {
    fn from(s: String) -> Self {
        PropertyKey::from(s.as_str())
    }
}

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
                Some(desc) => write!(f, "Symbol({})", desc),
                None => write!(f, "Symbol()"),
            },
        }
    }
}

/// Object property descriptor
#[derive(Debug, Clone)]
pub struct Property {
    pub value: JsValue,
    pub writable: bool,
    pub enumerable: bool,
    pub configurable: bool,
    /// Getter function (for accessor properties)
    pub getter: Option<JsObjectRef>,
    /// Setter function (for accessor properties)
    pub setter: Option<JsObjectRef>,
}

impl Property {
    pub fn data(value: JsValue) -> Self {
        Self {
            value,
            writable: true,
            enumerable: true,
            configurable: true,
            getter: None,
            setter: None,
        }
    }

    pub fn data_readonly(value: JsValue) -> Self {
        Self {
            value,
            writable: false,
            enumerable: true,
            configurable: true,
            getter: None,
            setter: None,
        }
    }

    /// Create an accessor property with getter and/or setter
    pub fn accessor(getter: Option<JsObjectRef>, setter: Option<JsObjectRef>) -> Self {
        Self {
            value: JsValue::Undefined,
            writable: false,
            enumerable: true,
            configurable: true,
            getter,
            setter,
        }
    }

    /// Check if this is an accessor property (has getter or setter)
    pub fn is_accessor(&self) -> bool {
        self.getter.is_some() || self.setter.is_some()
    }

    /// Create a property with custom attributes
    pub fn with_attributes(
        value: JsValue,
        writable: bool,
        enumerable: bool,
        configurable: bool,
    ) -> Self {
        Self {
            value,
            writable,
            enumerable,
            configurable,
            getter: None,
            setter: None,
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
    pub bindings: FxHashMap<JsString, Binding>,
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
    /// Array exotic object
    Array { length: u32 },
    /// Function exotic object
    Function(JsFunction),
    /// Map exotic object - stores key-value pairs preserving insertion order
    Map { entries: Vec<(JsValue, JsValue)> },
    /// Set exotic object - stores unique values preserving insertion order
    Set { entries: Vec<JsValue> },
    /// Date exotic object - stores timestamp in milliseconds since Unix epoch
    Date { timestamp: f64 },
    /// RegExp exotic object - stores pattern and flags
    RegExp { pattern: String, flags: String },
    /// Generator exotic object - stores generator state
    Generator(Rc<RefCell<GeneratorState>>),
    /// Promise exotic object - stores promise state
    Promise(Rc<RefCell<PromiseState>>),
    /// Environment exotic object - stores variable bindings
    Environment(EnvironmentData),
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

/// Generator state for suspended generators
#[derive(Debug, Clone)]
pub struct GeneratorState {
    /// The generator function's body (Rc for cheap cloning)
    pub body: Rc<BlockStatement>,
    /// Parameters of the generator function (Rc for cheap cloning)
    pub params: Rc<[FunctionParam]>,
    /// Arguments passed to the generator
    pub args: Vec<JsValue>,
    /// The captured closure environment (GC-managed)
    pub closure: JsObjectRef,
    /// Current execution state
    pub state: GeneratorStatus,
    /// Current statement index
    pub stmt_index: usize,
    /// Value passed in via next(value)
    pub sent_value: JsValue,
    /// Function name for debugging
    pub name: Option<JsString>,
}

/// Status of generator execution
#[derive(Debug, Clone, PartialEq)]
pub enum GeneratorStatus {
    /// Not yet started
    Suspended,
    /// Completed (returned or exhausted)
    Completed,
}

/// Function representation
#[derive(Debug, Clone)]
pub enum JsFunction {
    /// User-defined function
    Interpreted(InterpretedFunction),
    /// Native Rust function
    Native(NativeFunction),
    /// Bound function (created by Function.prototype.bind)
    Bound(Box<BoundFunctionData>),
    /// Promise resolve function (has internal [[Promise]] slot)
    PromiseResolve(JsObjectRef),
    /// Promise reject function (has internal [[Promise]] slot)
    PromiseReject(JsObjectRef),
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
            JsFunction::Interpreted(f) => f.name.as_ref().map(|s| s.as_str()),
            JsFunction::Native(f) => Some(f.name.as_ref()),
            JsFunction::Bound(_) => Some("bound"),
            JsFunction::PromiseResolve(_) => Some("resolve"),
            JsFunction::PromiseReject(_) => Some("reject"),
        }
    }
}

/// User-defined function
#[derive(Debug, Clone)]
pub struct InterpretedFunction {
    pub name: Option<JsString>,
    /// Function parameters wrapped in Rc for cheap cloning
    pub params: Rc<[FunctionParam]>,
    /// Function body wrapped in Rc for cheap cloning
    pub body: Rc<FunctionBody>,
    /// The captured closure environment (GC-managed)
    pub closure: JsObjectRef,
    pub source_location: Span,
    /// Whether this is a generator function (function*)
    pub generator: bool,
    /// Whether this is an async function
    pub async_: bool,
}

/// Function body (block or expression for arrow functions)
#[derive(Debug, Clone)]
pub enum FunctionBody {
    Block(BlockStatement),
    Expression(Box<crate::ast::Expression>),
}

impl From<ArrowFunctionBody> for FunctionBody {
    fn from(body: ArrowFunctionBody) -> Self {
        match body {
            ArrowFunctionBody::Block(block) => FunctionBody::Block(block),
            ArrowFunctionBody::Expression(expr) => FunctionBody::Expression(expr),
        }
    }
}

// TODO: Re-enable after GC migration
// /// Native function signature
// pub type NativeFn =
//     fn(&mut crate::interpreter::Interpreter, JsValue, &[JsValue]) -> Result<JsValue, JsError>;
pub type NativeFn = fn();

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
}

// ═══════════════════════════════════════════════════════════════════════════════
// Helper functions for creating GC-managed objects
//
// These functions use the temporary guard pattern:
// - Allocate with a temporary guard
// - Return both the object and the guard
// - Caller transfers ownership, then drops the guard
// ═══════════════════════════════════════════════════════════════════════════════

/// Create a new ordinary object with a temporary guard.
///
/// Returns `(object, temp_guard)`. The caller should:
/// 1. Establish ownership: `parent.own(&object, &heap)`
/// 2. Drop or let the temp_guard go out of scope
pub fn create_object_with_guard(guard: &Guard<JsObject>) -> JsObjectRef {
    // Object is default-initialized by Reset trait
    guard.alloc()
}

/// Create a new array object with a temporary guard.
///
/// Returns the array object. Caller is responsible for ownership transfer.
pub fn create_array_with_guard(
    guard: &Guard<JsObject>,
    dict: &mut StringDict,
    elements: Vec<JsValue>,
) -> JsObjectRef {
    let len = elements.len() as u32;
    let arr = guard.alloc();
    {
        let mut arr_ref = arr.borrow_mut();
        arr_ref.exotic = ExoticObject::Array { length: len };

        for (i, elem) in elements.into_iter().enumerate() {
            arr_ref
                .properties
                .insert(PropertyKey::Index(i as u32), Property::data(elem));
        }

        let length_key = PropertyKey::String(dict.get_or_insert("length"));
        arr_ref.properties.insert(
            length_key,
            Property::with_attributes(JsValue::Number(len as f64), true, false, false),
        );
    }
    arr
}

/// Create a function object with a temporary guard.
///
/// Returns the function object. Caller is responsible for ownership transfer.
pub fn create_function_with_guard(
    guard: &Guard<JsObject>,
    dict: &mut StringDict,
    func: JsFunction,
) -> JsObjectRef {
    let f = guard.alloc();
    {
        let mut f_ref = f.borrow_mut();
        f_ref.exotic = ExoticObject::Function(func);

        // Add length property
        let length_key = PropertyKey::String(dict.get_or_insert("length"));
        f_ref.properties.insert(
            length_key,
            Property::with_attributes(JsValue::Number(0.0), false, false, true),
        );
    }
    f
}

/// Register a native method on a prototype object.
///
/// Uses the given guard for allocation. The function object is owned by the
/// prototype through the property assignment.
pub fn register_method_with_guard(
    guard: &Guard<JsObject>,
    heap: &Heap<JsObject>,
    dict: &mut StringDict,
    obj: &JsObjectRef,
    name: &str,
    func: NativeFn,
    arity: usize,
) {
    let interned_name = dict.get_or_insert(name);
    let f = create_function_with_guard(
        guard,
        dict,
        JsFunction::Native(NativeFunction {
            name: interned_name.cheap_clone(),
            func,
            arity,
        }),
    );
    // Prototype owns the function
    obj.own(&f, heap);
    let key = PropertyKey::String(interned_name);
    obj.borrow_mut().set_property(key, JsValue::Object(f));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Environment GC Integration
// ═══════════════════════════════════════════════════════════════════════════════

/// Environment reference - a GC-managed environment object.
///
/// This is an alias for `JsObjectRef` where the object has `ExoticObject::Environment`.
/// Using this type makes it clear when a reference is expected to be an environment.
pub type EnvRef = JsObjectRef;

/// Create a new environment object with a temporary guard.
///
/// The environment is created with an optional outer environment reference.
/// Returns the environment object. Caller is responsible for ownership transfer.
pub fn create_environment_with_guard(
    guard: &Guard<JsObject>,
    heap: &Heap<JsObject>,
    outer: Option<EnvRef>,
) -> EnvRef {
    let env = guard.alloc();
    {
        let mut env_ref = env.borrow_mut();
        env_ref.null_prototype = true;
        env_ref.exotic = ExoticObject::Environment(EnvironmentData::with_outer(outer));
    }
    // If there's an outer environment, it owns this one
    if let Some(ref outer_env) = outer {
        outer_env.own(&env, heap);
    }
    env
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
}
