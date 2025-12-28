//! Symbol built-in object implementation

use crate::error::JsError;
use crate::interpreter::Interpreter;
use crate::value::{CheapClone, Guarded, JsString, JsSymbol, JsValue, PropertyKey};

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
        Self::new(&mut 1)
    }
}

impl WellKnownSymbols {
    /// Create well-known symbols using the provided counter.
    /// The counter is incremented for each symbol allocated.
    pub fn new(next_id: &mut u64) -> Self {
        let mut alloc = || {
            let id = *next_id;
            *next_id += 1;
            id
        };

        Self {
            iterator: alloc(),
            to_string_tag: alloc(),
            has_instance: alloc(),
            is_concat_spreadable: alloc(),
            species: alloc(),
            to_primitive: alloc(),
            unscopables: alloc(),
            match_symbol: alloc(),
            replace: alloc(),
            search: alloc(),
            split: alloc(),
            async_iterator: alloc(),
        }
    }
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

    let well_known = interp.well_known_symbols;

    // Create the Symbol function (not a constructor - can't be called with new)
    let symbol_fn = interp.create_native_function("Symbol", symbol_call, 0);
    interp.root_guard.guard(symbol_fn.clone());

    // Symbol.for(key) and Symbol.keyFor(sym)
    interp.register_method(&symbol_fn, "for", symbol_for, 1);
    interp.register_method(&symbol_fn, "keyFor", symbol_key_for, 1);

    // Well-known symbols
    let iterator_key = PropertyKey::String(interp.intern("iterator"));
    let to_string_tag_key = PropertyKey::String(interp.intern("toStringTag"));
    let has_instance_key = PropertyKey::String(interp.intern("hasInstance"));
    let is_concat_spreadable_key = PropertyKey::String(interp.intern("isConcatSpreadable"));
    let species_key = PropertyKey::String(interp.intern("species"));
    let to_primitive_key = PropertyKey::String(interp.intern("toPrimitive"));
    let unscopables_key = PropertyKey::String(interp.intern("unscopables"));
    let match_key = PropertyKey::String(interp.intern("match"));
    let replace_key = PropertyKey::String(interp.intern("replace"));
    let search_key = PropertyKey::String(interp.intern("search"));
    let split_key = PropertyKey::String(interp.intern("split"));
    let async_iterator_key = PropertyKey::String(interp.intern("asyncIterator"));

    // Intern well-known symbol descriptions
    let sym_iterator = interp.intern("Symbol.iterator");
    let sym_to_string_tag = interp.intern("Symbol.toStringTag");
    let sym_has_instance = interp.intern("Symbol.hasInstance");
    let sym_is_concat_spreadable = interp.intern("Symbol.isConcatSpreadable");
    let sym_species = interp.intern("Symbol.species");
    let sym_to_primitive = interp.intern("Symbol.toPrimitive");
    let sym_unscopables = interp.intern("Symbol.unscopables");
    let sym_match = interp.intern("Symbol.match");
    let sym_replace = interp.intern("Symbol.replace");
    let sym_search = interp.intern("Symbol.search");
    let sym_split = interp.intern("Symbol.split");
    let sym_async_iterator = interp.intern("Symbol.asyncIterator");

    {
        let mut sym = symbol_fn.borrow_mut();

        sym.set_property(
            iterator_key,
            JsValue::Symbol(Box::new(JsSymbol::new(
                well_known.iterator,
                Some(sym_iterator),
            ))),
        );
        sym.set_property(
            to_string_tag_key,
            JsValue::Symbol(Box::new(JsSymbol::new(
                well_known.to_string_tag,
                Some(sym_to_string_tag),
            ))),
        );
        sym.set_property(
            has_instance_key,
            JsValue::Symbol(Box::new(JsSymbol::new(
                well_known.has_instance,
                Some(sym_has_instance),
            ))),
        );
        sym.set_property(
            is_concat_spreadable_key,
            JsValue::Symbol(Box::new(JsSymbol::new(
                well_known.is_concat_spreadable,
                Some(sym_is_concat_spreadable),
            ))),
        );
        sym.set_property(
            species_key,
            JsValue::Symbol(Box::new(JsSymbol::new(
                well_known.species,
                Some(sym_species),
            ))),
        );
        sym.set_property(
            to_primitive_key,
            JsValue::Symbol(Box::new(JsSymbol::new(
                well_known.to_primitive,
                Some(sym_to_primitive),
            ))),
        );
        sym.set_property(
            unscopables_key,
            JsValue::Symbol(Box::new(JsSymbol::new(
                well_known.unscopables,
                Some(sym_unscopables),
            ))),
        );
        sym.set_property(
            match_key,
            JsValue::Symbol(Box::new(JsSymbol::new(
                well_known.match_symbol,
                Some(sym_match),
            ))),
        );
        sym.set_property(
            replace_key,
            JsValue::Symbol(Box::new(JsSymbol::new(
                well_known.replace,
                Some(sym_replace),
            ))),
        );
        sym.set_property(
            search_key,
            JsValue::Symbol(Box::new(JsSymbol::new(well_known.search, Some(sym_search)))),
        );
        sym.set_property(
            split_key,
            JsValue::Symbol(Box::new(JsSymbol::new(well_known.split, Some(sym_split)))),
        );
        sym.set_property(
            async_iterator_key,
            JsValue::Symbol(Box::new(JsSymbol::new(
                well_known.async_iterator,
                Some(sym_async_iterator),
            ))),
        );
    }

    // Set Symbol.prototype.constructor = Symbol
    let constructor_key = PropertyKey::String(interp.intern("constructor"));
    interp
        .symbol_prototype
        .borrow_mut()
        .set_property(constructor_key, JsValue::Object(symbol_fn.clone()));

    // Register globally
    let symbol_key = PropertyKey::String(interp.intern("Symbol"));
    interp
        .global
        .borrow_mut()
        .set_property(symbol_key, JsValue::Object(symbol_fn));
}

/// Symbol() - create a new unique symbol
fn symbol_call(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let description = match args.first() {
        None | Some(JsValue::Undefined) => None,
        Some(other) => Some(interp.to_js_string(other)),
    };

    let id = interp.next_symbol_id();
    Ok(Guarded::unguarded(JsValue::Symbol(Box::new(
        JsSymbol::new(id, description),
    ))))
}

/// Symbol.for(key) - get or create a symbol in the global registry
fn symbol_for(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let key = match args.first() {
        Some(v) => interp.to_js_string(v),
        None => interp.intern("undefined"),
    };

    // Check if symbol already exists in registry
    if let Some(sym) = interp.symbol_registry_get(&key) {
        return Ok(Guarded::unguarded(JsValue::Symbol(Box::new(sym))));
    }

    // Create new symbol and register it
    let id = interp.next_symbol_id();
    let sym = JsSymbol::new(id, Some(key.cheap_clone()));
    interp.symbol_registry_insert(key, sym.clone());
    Ok(Guarded::unguarded(JsValue::Symbol(Box::new(sym))))
}

/// Symbol.keyFor(sym) - get the key for a registered symbol
fn symbol_key_for(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let sym = match args.first() {
        Some(JsValue::Symbol(s)) => s,
        _ => {
            return Err(JsError::type_error(
                "Symbol.keyFor requires a symbol argument",
            ));
        }
    };

    match interp.symbol_registry_key_for(sym.id()) {
        Some(key) => Ok(Guarded::unguarded(JsValue::String(key))),
        None => Ok(Guarded::unguarded(JsValue::Undefined)),
    }
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
                Some(desc) => format!("Symbol({})", desc.as_str()),
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
