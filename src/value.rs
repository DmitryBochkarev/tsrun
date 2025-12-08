//! JavaScript value representation
//!
//! The core JsValue type and related structures for representing JavaScript values at runtime.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use crate::ast::{ArrowFunctionBody, BlockStatement, FunctionParam};
use crate::error::JsError;
use crate::lexer::Span;

/// A JavaScript value
#[derive(Clone)]
pub enum JsValue {
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
            (JsValue::Object(a), JsValue::Object(b)) => Rc::ptr_eq(a, b),
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

impl Default for JsValue {
    fn default() -> Self {
        JsValue::Undefined
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

/// Reference to a heap-allocated object
pub type JsObjectRef = Rc<RefCell<JsObject>>;

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
    pub properties: HashMap<PropertyKey, Property>,
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
            properties: HashMap::new(),
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
            properties: HashMap::new(),
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
}

impl From<&str> for PropertyKey {
    fn from(s: &str) -> Self {
        // Check if it's a valid array index
        if let Ok(idx) = s.parse::<u32>() {
            if idx.to_string() == s {
                return PropertyKey::Index(idx);
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
    pub fn with_attributes(value: JsValue, writable: bool, enumerable: bool, configurable: bool) -> Self {
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
}

/// Generator state for suspended generators
#[derive(Debug, Clone)]
pub struct GeneratorState {
    /// The generator function's body
    pub body: BlockStatement,
    /// Parameters of the generator function
    pub params: Vec<FunctionParam>,
    /// Arguments passed to the generator
    pub args: Vec<JsValue>,
    /// The captured closure environment
    pub closure: Environment,
    /// Current execution state
    pub state: GeneratorStatus,
    /// Current statement index
    pub stmt_index: usize,
    /// Value passed in via next(value)
    pub sent_value: JsValue,
    /// Function name for debugging
    pub name: Option<String>,
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
            JsFunction::Interpreted(f) => f.name.as_deref(),
            JsFunction::Native(f) => Some(&f.name),
            JsFunction::Bound(_) => Some("bound"),
        }
    }
}

/// User-defined function
#[derive(Debug, Clone)]
pub struct InterpretedFunction {
    pub name: Option<String>,
    pub params: Vec<FunctionParam>,
    pub body: FunctionBody,
    pub closure: Environment,
    pub source_location: Span,
    /// Whether this is a generator function (function*)
    pub generator: bool,
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

/// Native function signature
pub type NativeFn = fn(&mut crate::interpreter::Interpreter, JsValue, Vec<JsValue>) -> Result<JsValue, JsError>;

/// Native function wrapper
#[derive(Clone)]
pub struct NativeFunction {
    pub name: String,
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

/// Execution environment for variable bindings
#[derive(Debug, Clone)]
pub struct Environment {
    pub bindings: Rc<RefCell<HashMap<String, Binding>>>,
    pub outer: Option<Box<Environment>>,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            bindings: Rc::new(RefCell::new(HashMap::new())),
            outer: None,
        }
    }

    pub fn with_outer(outer: Environment) -> Self {
        Self {
            bindings: Rc::new(RefCell::new(HashMap::new())),
            outer: Some(Box::new(outer)),
        }
    }

    /// Define a new binding
    pub fn define(&mut self, name: String, value: JsValue, mutable: bool) {
        self.bindings.borrow_mut().insert(
            name,
            Binding {
                value,
                mutable,
                initialized: true,
            },
        );
    }

    /// Define an uninitialized binding (for TDZ - let/const before declaration)
    pub fn define_uninitialized(&mut self, name: String, mutable: bool) {
        self.bindings.borrow_mut().insert(
            name,
            Binding {
                value: JsValue::Undefined,
                mutable,
                initialized: false,
            },
        );
    }

    /// Initialize a previously uninitialized binding (for TDZ)
    pub fn initialize(&self, name: &str, value: JsValue) -> Result<(), JsError> {
        if let Some(binding) = self.bindings.borrow_mut().get_mut(name) {
            binding.value = value;
            binding.initialized = true;
            return Ok(());
        }

        if let Some(ref outer) = self.outer {
            return outer.initialize(name, value);
        }

        Err(JsError::reference_error(name))
    }

    /// Get a binding value (returns Err for uninitialized TDZ bindings)
    pub fn get(&self, name: &str) -> Result<JsValue, JsError> {
        if let Some(binding) = self.bindings.borrow().get(name) {
            if !binding.initialized {
                return Err(JsError::reference_error_with_message(
                    name,
                    "Cannot access before initialization",
                ));
            }
            return Ok(binding.value.clone());
        }

        if let Some(ref outer) = self.outer {
            return outer.get(name);
        }

        Err(JsError::reference_error(name))
    }

    /// Set a binding value
    pub fn set(&self, name: &str, value: JsValue) -> Result<(), JsError> {
        if let Some(binding) = self.bindings.borrow_mut().get_mut(name) {
            if !binding.initialized {
                return Err(JsError::reference_error_with_message(
                    name,
                    "Cannot access before initialization",
                ));
            }
            if !binding.mutable {
                return Err(JsError::type_error(format!(
                    "Assignment to constant variable '{}'",
                    name
                )));
            }
            binding.value = value;
            return Ok(());
        }

        if let Some(ref outer) = self.outer {
            return outer.set(name, value);
        }

        Err(JsError::reference_error(name))
    }

    /// Check if a binding exists
    pub fn has(&self, name: &str) -> bool {
        if self.bindings.borrow().contains_key(name) {
            return true;
        }

        if let Some(ref outer) = self.outer {
            return outer.has(name);
        }

        false
    }

    /// Check if a binding exists in this exact scope (not outer)
    pub fn has_own(&self, name: &str) -> bool {
        self.bindings.borrow().contains_key(name)
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

/// Variable binding
#[derive(Debug, Clone)]
pub struct Binding {
    pub value: JsValue,
    pub mutable: bool,
    pub initialized: bool,
}

// Helper functions for creating objects

/// Create a new ordinary object
pub fn create_object() -> JsObjectRef {
    Rc::new(RefCell::new(JsObject::new()))
}

/// Create a new array object
pub fn create_array(elements: Vec<JsValue>) -> JsObjectRef {
    let len = elements.len() as u32;
    let mut obj = JsObject {
        prototype: None, // Should be Array.prototype
        extensible: true,
        frozen: false,
        sealed: false,
        null_prototype: false,
        properties: HashMap::new(),
        exotic: ExoticObject::Array { length: len },
    };

    for (i, elem) in elements.into_iter().enumerate() {
        obj.properties.insert(
            PropertyKey::Index(i as u32),
            Property::data(elem),
        );
    }

    obj.properties.insert(
        PropertyKey::from("length"),
        Property::with_attributes(JsValue::Number(len as f64), true, false, false),
    );

    Rc::new(RefCell::new(obj))
}

/// Create a function object
pub fn create_function(func: JsFunction) -> JsObjectRef {
    let mut obj = JsObject {
        prototype: None, // Should be Function.prototype
        extensible: true,
        frozen: false,
        sealed: false,
        null_prototype: false,
        properties: HashMap::new(),
        exotic: ExoticObject::Function(func),
    };

    // Add length and name properties
    obj.properties.insert(
        PropertyKey::from("length"),
        Property::with_attributes(JsValue::Number(0.0), false, false, true),
    );

    Rc::new(RefCell::new(obj))
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
        assert!(JsValue::String(JsString::from("hello")).to_number().is_nan());
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
