//! Symbol built-in object implementation

use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};

use rustc_hash::FxHashMap;

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{Guarded, JsString, JsSymbol, JsValue};

/// Global symbol ID counter for generating unique symbol IDs
static SYMBOL_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generate a unique symbol ID
pub fn next_symbol_id() -> u64 {
    SYMBOL_ID_COUNTER.fetch_add(1, Ordering::SeqCst)
}

// Symbol registry - maps string keys to symbols for Symbol.for()
// This is a thread-local registry to avoid requiring synchronization
thread_local! {
    static SYMBOL_REGISTRY: RefCell<FxHashMap<String, JsSymbol>> = RefCell::new(FxHashMap::default());
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

/// Initialize Symbol.prototype with toString and valueOf methods
pub fn init_symbol_prototype(interp: &mut Interpreter) {
    let proto = interp.symbol_prototype.clone();

    // Symbol.prototype.toString()
    interp.register_method(&proto, "toString", symbol_to_string, 0);

    // Symbol.prototype.valueOf()
    interp.register_method(&proto, "valueOf", symbol_value_of, 0);

    // Symbol.prototype.description (getter)
    // Note: In full JS this is an accessor property. For simplicity we implement
    // description access directly in member expression evaluation.
}

/// Initialize Symbol constructor and register it globally
pub fn init_symbol(interp: &mut Interpreter) {
    init_symbol_prototype(interp);

    let well_known = get_well_known_symbols();

    // Create the Symbol function (not a constructor - can't be called with new)
    let symbol_fn = interp.create_native_function("Symbol", symbol_call, 0);
    interp.root_guard.guard(symbol_fn.clone());

    // Symbol.for(key) and Symbol.keyFor(sym)
    interp.register_method(&symbol_fn, "for", symbol_for, 1);
    interp.register_method(&symbol_fn, "keyFor", symbol_key_for, 1);

    // Well-known symbols
    let iterator_key = interp.key("iterator");
    let to_string_tag_key = interp.key("toStringTag");
    let has_instance_key = interp.key("hasInstance");
    let is_concat_spreadable_key = interp.key("isConcatSpreadable");
    let species_key = interp.key("species");
    let to_primitive_key = interp.key("toPrimitive");
    let unscopables_key = interp.key("unscopables");
    let match_key = interp.key("match");
    let replace_key = interp.key("replace");
    let search_key = interp.key("search");
    let split_key = interp.key("split");
    let async_iterator_key = interp.key("asyncIterator");

    {
        let mut sym = symbol_fn.borrow_mut();

        sym.set_property(
            iterator_key,
            JsValue::Symbol(Box::new(JsSymbol::new(
                well_known.iterator,
                Some("Symbol.iterator".to_string()),
            ))),
        );
        sym.set_property(
            to_string_tag_key,
            JsValue::Symbol(Box::new(JsSymbol::new(
                well_known.to_string_tag,
                Some("Symbol.toStringTag".to_string()),
            ))),
        );
        sym.set_property(
            has_instance_key,
            JsValue::Symbol(Box::new(JsSymbol::new(
                well_known.has_instance,
                Some("Symbol.hasInstance".to_string()),
            ))),
        );
        sym.set_property(
            is_concat_spreadable_key,
            JsValue::Symbol(Box::new(JsSymbol::new(
                well_known.is_concat_spreadable,
                Some("Symbol.isConcatSpreadable".to_string()),
            ))),
        );
        sym.set_property(
            species_key,
            JsValue::Symbol(Box::new(JsSymbol::new(
                well_known.species,
                Some("Symbol.species".to_string()),
            ))),
        );
        sym.set_property(
            to_primitive_key,
            JsValue::Symbol(Box::new(JsSymbol::new(
                well_known.to_primitive,
                Some("Symbol.toPrimitive".to_string()),
            ))),
        );
        sym.set_property(
            unscopables_key,
            JsValue::Symbol(Box::new(JsSymbol::new(
                well_known.unscopables,
                Some("Symbol.unscopables".to_string()),
            ))),
        );
        sym.set_property(
            match_key,
            JsValue::Symbol(Box::new(JsSymbol::new(
                well_known.match_symbol,
                Some("Symbol.match".to_string()),
            ))),
        );
        sym.set_property(
            replace_key,
            JsValue::Symbol(Box::new(JsSymbol::new(
                well_known.replace,
                Some("Symbol.replace".to_string()),
            ))),
        );
        sym.set_property(
            search_key,
            JsValue::Symbol(Box::new(JsSymbol::new(
                well_known.search,
                Some("Symbol.search".to_string()),
            ))),
        );
        sym.set_property(
            split_key,
            JsValue::Symbol(Box::new(JsSymbol::new(
                well_known.split,
                Some("Symbol.split".to_string()),
            ))),
        );
        sym.set_property(
            async_iterator_key,
            JsValue::Symbol(Box::new(JsSymbol::new(
                well_known.async_iterator,
                Some("Symbol.asyncIterator".to_string()),
            ))),
        );
    }

    // Register globally
    let symbol_key = interp.key("Symbol");
    interp
        .global
        .borrow_mut()
        .set_property(symbol_key, JsValue::Object(symbol_fn));
}

/// Symbol() - create a new unique symbol
fn symbol_call(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let description = match args.first() {
        None | Some(JsValue::Undefined) => None,
        Some(other) => Some(other.to_js_string().to_string()),
    };

    let id = next_symbol_id();
    Ok(Guarded::unguarded(JsValue::Symbol(Box::new(JsSymbol::new(
        id,
        description,
    )))))
}

/// Symbol.for(key) - get or create a symbol in the global registry
fn symbol_for(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let key = args
        .first()
        .map(|v| v.to_js_string().to_string())
        .unwrap_or_else(|| "undefined".to_string());

    SYMBOL_REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        if let Some(sym) = registry.get(&key) {
            return Ok(Guarded::unguarded(JsValue::Symbol(Box::new(sym.clone()))));
        }

        let id = next_symbol_id();
        let sym = JsSymbol::new(id, Some(key.clone()));
        registry.insert(key, sym.clone());
        Ok(Guarded::unguarded(JsValue::Symbol(Box::new(sym))))
    })
}

/// Symbol.keyFor(sym) - get the key for a registered symbol
fn symbol_key_for(
    _interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
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
                return Ok(Guarded::unguarded(JsValue::String(JsString::from(
                    key.as_str(),
                ))));
            }
        }
        Ok(Guarded::unguarded(JsValue::Undefined))
    })
}

/// Symbol.prototype.toString()
fn symbol_to_string(
    _interp: &mut Interpreter,
    this: JsValue,
    _args: &[JsValue],
) -> Result<Guarded, JsError> {
    match this {
        JsValue::Symbol(s) => {
            let result = match &s.description {
                Some(desc) => format!("Symbol({})", desc),
                None => "Symbol()".to_string(),
            };
            Ok(Guarded::unguarded(JsValue::String(JsString::from(result))))
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
) -> Result<Guarded, JsError> {
    match this {
        JsValue::Symbol(_) => Ok(Guarded::unguarded(this)),
        _ => Err(JsError::type_error(
            "Symbol.prototype.valueOf requires that 'this' be a Symbol",
        )),
    }
}
