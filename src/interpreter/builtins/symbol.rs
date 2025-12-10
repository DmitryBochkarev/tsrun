//! Symbol built-in object implementation

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{
    create_function, create_object, register_method, JsFunction, JsObjectRef, JsString, JsSymbol,
    JsValue, NativeFunction, PropertyKey,
};

/// Global symbol ID counter for generating unique symbol IDs
static SYMBOL_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generate a unique symbol ID
pub fn next_symbol_id() -> u64 {
    SYMBOL_ID_COUNTER.fetch_add(1, Ordering::SeqCst)
}

// Symbol registry - maps string keys to symbols for Symbol.for()
// This is a thread-local registry to avoid requiring synchronization
thread_local! {
    static SYMBOL_REGISTRY: RefCell<HashMap<String, JsSymbol>> = RefCell::new(HashMap::new());
}

/// Well-known symbol IDs (reserved during initialization)
#[derive(Clone, Copy)]
pub struct WellKnownSymbols {
    pub iterator: u64,
    pub to_string_tag: u64,
    pub has_instance: u64,
    pub is_concat_spreadable: u64,
    pub species: u64,
    pub to_primitive: u64,
    pub unscopables: u64,
    pub match_symbol: u64,
    pub replace: u64,
    pub search: u64,
    pub split: u64,
    pub async_iterator: u64,
}

impl Default for WellKnownSymbols {
    fn default() -> Self {
        Self::new()
    }
}

impl WellKnownSymbols {
    pub fn new() -> Self {
        Self {
            iterator: next_symbol_id(),
            to_string_tag: next_symbol_id(),
            has_instance: next_symbol_id(),
            is_concat_spreadable: next_symbol_id(),
            species: next_symbol_id(),
            to_primitive: next_symbol_id(),
            unscopables: next_symbol_id(),
            match_symbol: next_symbol_id(),
            replace: next_symbol_id(),
            search: next_symbol_id(),
            split: next_symbol_id(),
            async_iterator: next_symbol_id(),
        }
    }
}

// Global well-known symbols singleton
thread_local! {
    static WELL_KNOWN_SYMBOLS: WellKnownSymbols = WellKnownSymbols::new();
}

pub fn get_well_known_symbols() -> WellKnownSymbols {
    WELL_KNOWN_SYMBOLS.with(|s| *s)
}

/// Create the Symbol constructor function object
pub fn create_symbol_constructor(
    _symbol_prototype: &JsObjectRef,
    well_known: &WellKnownSymbols,
) -> JsObjectRef {
    // Create the Symbol function (not a constructor - can't be called with new)
    let symbol_fn = create_function(JsFunction::Native(NativeFunction {
        name: "Symbol".to_string(),
        func: symbol_call,
        arity: 0,
    }));

    let mut sym = symbol_fn.borrow_mut();

    // Symbol.for(key) and Symbol.keyFor(sym)
    register_method(&mut sym, "for", symbol_for, 1);
    register_method(&mut sym, "keyFor", symbol_key_for, 1);

    // Well-known symbols
    sym.set_property(
        PropertyKey::from("iterator"),
        JsValue::Symbol(JsSymbol::new(
            well_known.iterator,
            Some("Symbol.iterator".to_string()),
        )),
    );
    sym.set_property(
        PropertyKey::from("toStringTag"),
        JsValue::Symbol(JsSymbol::new(
            well_known.to_string_tag,
            Some("Symbol.toStringTag".to_string()),
        )),
    );
    sym.set_property(
        PropertyKey::from("hasInstance"),
        JsValue::Symbol(JsSymbol::new(
            well_known.has_instance,
            Some("Symbol.hasInstance".to_string()),
        )),
    );
    sym.set_property(
        PropertyKey::from("isConcatSpreadable"),
        JsValue::Symbol(JsSymbol::new(
            well_known.is_concat_spreadable,
            Some("Symbol.isConcatSpreadable".to_string()),
        )),
    );
    sym.set_property(
        PropertyKey::from("species"),
        JsValue::Symbol(JsSymbol::new(
            well_known.species,
            Some("Symbol.species".to_string()),
        )),
    );
    sym.set_property(
        PropertyKey::from("toPrimitive"),
        JsValue::Symbol(JsSymbol::new(
            well_known.to_primitive,
            Some("Symbol.toPrimitive".to_string()),
        )),
    );
    sym.set_property(
        PropertyKey::from("unscopables"),
        JsValue::Symbol(JsSymbol::new(
            well_known.unscopables,
            Some("Symbol.unscopables".to_string()),
        )),
    );
    sym.set_property(
        PropertyKey::from("match"),
        JsValue::Symbol(JsSymbol::new(
            well_known.match_symbol,
            Some("Symbol.match".to_string()),
        )),
    );
    sym.set_property(
        PropertyKey::from("replace"),
        JsValue::Symbol(JsSymbol::new(
            well_known.replace,
            Some("Symbol.replace".to_string()),
        )),
    );
    sym.set_property(
        PropertyKey::from("search"),
        JsValue::Symbol(JsSymbol::new(
            well_known.search,
            Some("Symbol.search".to_string()),
        )),
    );
    sym.set_property(
        PropertyKey::from("split"),
        JsValue::Symbol(JsSymbol::new(
            well_known.split,
            Some("Symbol.split".to_string()),
        )),
    );
    sym.set_property(
        PropertyKey::from("asyncIterator"),
        JsValue::Symbol(JsSymbol::new(
            well_known.async_iterator,
            Some("Symbol.asyncIterator".to_string()),
        )),
    );

    drop(sym);
    symbol_fn
}

/// Create Symbol.prototype
pub fn create_symbol_prototype() -> JsObjectRef {
    let proto = create_object();
    let mut p = proto.borrow_mut();

    // Symbol.prototype.toString()
    let to_string_fn = create_function(JsFunction::Native(NativeFunction {
        name: "toString".to_string(),
        func: symbol_to_string,
        arity: 0,
    }));
    p.set_property(PropertyKey::from("toString"), JsValue::Object(to_string_fn));

    // Symbol.prototype.valueOf()
    let value_of_fn = create_function(JsFunction::Native(NativeFunction {
        name: "valueOf".to_string(),
        func: symbol_value_of,
        arity: 0,
    }));
    p.set_property(PropertyKey::from("valueOf"), JsValue::Object(value_of_fn));

    // Symbol.prototype.description (getter)
    // Note: In full JS this is an accessor property. For simplicity we implement
    // description access directly in member expression evaluation.

    drop(p);
    proto
}

/// Symbol() - create a new unique symbol
fn symbol_call(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let description = match args.first() {
        None | Some(JsValue::Undefined) => None,
        Some(other) => Some(other.to_js_string().to_string()),
    };

    let id = next_symbol_id();
    Ok(JsValue::Symbol(JsSymbol::new(id, description)))
}

/// Symbol.for(key) - get or create a symbol in the global registry
fn symbol_for(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let key = args
        .first()
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_else(|| "undefined".to_string());

    SYMBOL_REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        if let Some(sym) = registry.get(&key) {
            return Ok(JsValue::Symbol(sym.clone()));
        }

        let id = next_symbol_id();
        let sym = JsSymbol::new(id, Some(key.clone()));
        registry.insert(key, sym.clone());
        Ok(JsValue::Symbol(sym))
    })
}

/// Symbol.keyFor(sym) - get the key for a registered symbol
fn symbol_key_for(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, JsError> {
    let sym = match args.first() {
        Some(JsValue::Symbol(s)) => s,
        _ => {
            return Err(JsError::type_error(
                "Symbol.keyFor requires a symbol argument",
            ))
        }
    };

    SYMBOL_REGISTRY.with(|registry| {
        let registry = registry.borrow();
        for (key, registered_sym) in registry.iter() {
            if registered_sym.id() == sym.id() {
                return Ok(JsValue::String(JsString::from(key.as_str())));
            }
        }
        Ok(JsValue::Undefined)
    })
}

/// Symbol.prototype.toString()
fn symbol_to_string(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    match this {
        JsValue::Symbol(s) => {
            let result = match &s.description {
                Some(desc) => format!("Symbol({})", desc),
                None => "Symbol()".to_string(),
            };
            Ok(JsValue::String(JsString::from(result)))
        }
        _ => Err(JsError::type_error(
            "Symbol.prototype.toString requires that 'this' be a Symbol",
        )),
    }
}

/// Symbol.prototype.valueOf()
fn symbol_value_of(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, JsError> {
    match this {
        JsValue::Symbol(_) => Ok(this),
        _ => Err(JsError::type_error(
            "Symbol.prototype.valueOf requires that 'this' be a Symbol",
        )),
    }
}
