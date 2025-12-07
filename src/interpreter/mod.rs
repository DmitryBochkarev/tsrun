//! Interpreter for executing TypeScript AST

// Builtin function implementations (split into separate files)
pub mod builtins;

// Import all builtin functions
use builtins::*;

use crate::ast::{
    Argument, ArrayElement, AssignmentExpression, AssignmentOp, AssignmentTarget, BinaryExpression,
    BinaryOp, BlockStatement, CallExpression, ClassDeclaration, ConditionalExpression,
    EnumDeclaration, Expression, ForInOfLeft, ForInStatement, ForInit, ForOfStatement,
    ForStatement, FunctionDeclaration, LiteralValue, LogicalExpression, LogicalOp,
    MemberExpression, MemberProperty, NewExpression, ObjectPatternProperty, ObjectProperty,
    ObjectPropertyKey, Pattern, Program, Statement, UnaryExpression, UnaryOp, UpdateExpression,
    UpdateOp, VariableDeclaration, VariableKind,
};
use crate::error::JsError;
use crate::value::{
    create_array, create_function, create_object, Environment, ExoticObject, FunctionBody,
    InterpretedFunction, JsFunction, JsObjectRef, JsString, JsValue, NativeFunction, PropertyKey,
};

/// Completion record for control flow
#[derive(Debug)]
pub enum Completion {
    Normal(JsValue),
    Return(JsValue),
    Break(Option<String>),
    Continue(Option<String>),
}

/// The interpreter state
pub struct Interpreter {
    /// Global object
    pub global: JsObjectRef,
    /// Current environment
    pub env: Environment,
    /// Object.prototype for all objects
    pub object_prototype: JsObjectRef,
    /// Array.prototype for all array instances
    pub array_prototype: JsObjectRef,
    /// String.prototype for string methods
    pub string_prototype: JsObjectRef,
    /// Number.prototype for number methods
    pub number_prototype: JsObjectRef,
    /// Function.prototype for function methods (call, apply, bind)
    pub function_prototype: JsObjectRef,
    /// Map.prototype for map methods
    pub map_prototype: JsObjectRef,
    /// Set.prototype for set methods
    pub set_prototype: JsObjectRef,
    /// Date.prototype for date methods
    pub date_prototype: JsObjectRef,
    /// RegExp.prototype for regex methods
    pub regexp_prototype: JsObjectRef,
}

impl Interpreter {
    /// Create a new interpreter with global environment
    pub fn new() -> Self {
        let global = create_object();
        let mut env = Environment::new();

        // Add some basic global values
        env.define("undefined".to_string(), JsValue::Undefined, false);
        env.define("NaN".to_string(), JsValue::Number(f64::NAN), false);
        env.define("Infinity".to_string(), JsValue::Number(f64::INFINITY), false);

        // Add console object with methods
        let console = create_object();
        {
            let mut con = console.borrow_mut();

            let log_fn = create_function(JsFunction::Native(NativeFunction {
                name: "log".to_string(),
                func: console_log,
                arity: 0,
            }));
            con.set_property(PropertyKey::from("log"), JsValue::Object(log_fn));

            let error_fn = create_function(JsFunction::Native(NativeFunction {
                name: "error".to_string(),
                func: console_error,
                arity: 0,
            }));
            con.set_property(PropertyKey::from("error"), JsValue::Object(error_fn));

            let warn_fn = create_function(JsFunction::Native(NativeFunction {
                name: "warn".to_string(),
                func: console_warn,
                arity: 0,
            }));
            con.set_property(PropertyKey::from("warn"), JsValue::Object(warn_fn));

            let info_fn = create_function(JsFunction::Native(NativeFunction {
                name: "info".to_string(),
                func: console_info,
                arity: 0,
            }));
            con.set_property(PropertyKey::from("info"), JsValue::Object(info_fn));

            let debug_fn = create_function(JsFunction::Native(NativeFunction {
                name: "debug".to_string(),
                func: console_debug,
                arity: 0,
            }));
            con.set_property(PropertyKey::from("debug"), JsValue::Object(debug_fn));
        }
        env.define("console".to_string(), JsValue::Object(console), false);

        // Add JSON object
        let json = create_object();
        {
            let stringify_fn = create_function(JsFunction::Native(NativeFunction {
                name: "stringify".to_string(),
                func: json_stringify,
                arity: 1,
            }));
            json.borrow_mut().set_property(PropertyKey::from("stringify"), JsValue::Object(stringify_fn));

            let parse_fn = create_function(JsFunction::Native(NativeFunction {
                name: "parse".to_string(),
                func: json_parse,
                arity: 1,
            }));
            json.borrow_mut().set_property(PropertyKey::from("parse"), JsValue::Object(parse_fn));
        }
        env.define("JSON".to_string(), JsValue::Object(json), false);

        // Add Object global
        let object_constructor = create_function(JsFunction::Native(NativeFunction {
            name: "Object".to_string(),
            func: object_constructor,
            arity: 1,
        }));
        {
            let mut obj = object_constructor.borrow_mut();

            let keys_fn = create_function(JsFunction::Native(NativeFunction {
                name: "keys".to_string(),
                func: object_keys,
                arity: 1,
            }));
            obj.set_property(PropertyKey::from("keys"), JsValue::Object(keys_fn));

            let values_fn = create_function(JsFunction::Native(NativeFunction {
                name: "values".to_string(),
                func: object_values,
                arity: 1,
            }));
            obj.set_property(PropertyKey::from("values"), JsValue::Object(values_fn));

            let entries_fn = create_function(JsFunction::Native(NativeFunction {
                name: "entries".to_string(),
                func: object_entries,
                arity: 1,
            }));
            obj.set_property(PropertyKey::from("entries"), JsValue::Object(entries_fn));

            let assign_fn = create_function(JsFunction::Native(NativeFunction {
                name: "assign".to_string(),
                func: object_assign,
                arity: 2,
            }));
            obj.set_property(PropertyKey::from("assign"), JsValue::Object(assign_fn));

            let fromentries_fn = create_function(JsFunction::Native(NativeFunction {
                name: "fromEntries".to_string(),
                func: object_from_entries,
                arity: 1,
            }));
            obj.set_property(PropertyKey::from("fromEntries"), JsValue::Object(fromentries_fn));

            let hasown_fn = create_function(JsFunction::Native(NativeFunction {
                name: "hasOwn".to_string(),
                func: object_has_own,
                arity: 2,
            }));
            obj.set_property(PropertyKey::from("hasOwn"), JsValue::Object(hasown_fn));

            let create_fn = create_function(JsFunction::Native(NativeFunction {
                name: "create".to_string(),
                func: object_create,
                arity: 1,
            }));
            obj.set_property(PropertyKey::from("create"), JsValue::Object(create_fn));

            let freeze_fn = create_function(JsFunction::Native(NativeFunction {
                name: "freeze".to_string(),
                func: object_freeze,
                arity: 1,
            }));
            obj.set_property(PropertyKey::from("freeze"), JsValue::Object(freeze_fn));

            let isfrozen_fn = create_function(JsFunction::Native(NativeFunction {
                name: "isFrozen".to_string(),
                func: object_is_frozen,
                arity: 1,
            }));
            obj.set_property(PropertyKey::from("isFrozen"), JsValue::Object(isfrozen_fn));

            let seal_fn = create_function(JsFunction::Native(NativeFunction {
                name: "seal".to_string(),
                func: object_seal,
                arity: 1,
            }));
            obj.set_property(PropertyKey::from("seal"), JsValue::Object(seal_fn));

            let issealed_fn = create_function(JsFunction::Native(NativeFunction {
                name: "isSealed".to_string(),
                func: object_is_sealed,
                arity: 1,
            }));
            obj.set_property(PropertyKey::from("isSealed"), JsValue::Object(issealed_fn));
        }
        // Create Object.prototype
        let object_prototype = create_object();
        {
            let mut proto = object_prototype.borrow_mut();

            let hasownprop_fn = create_function(JsFunction::Native(NativeFunction {
                name: "hasOwnProperty".to_string(),
                func: object_has_own_property,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("hasOwnProperty"), JsValue::Object(hasownprop_fn));

            let tostring_fn = create_function(JsFunction::Native(NativeFunction {
                name: "toString".to_string(),
                func: object_to_string,
                arity: 0,
            }));
            proto.set_property(PropertyKey::from("toString"), JsValue::Object(tostring_fn));

            let valueof_fn = create_function(JsFunction::Native(NativeFunction {
                name: "valueOf".to_string(),
                func: object_value_of,
                arity: 0,
            }));
            proto.set_property(PropertyKey::from("valueOf"), JsValue::Object(valueof_fn));
        }
        env.define("Object".to_string(), JsValue::Object(object_constructor), false);

        // Create Array.prototype with methods
        let array_prototype = create_object();
        {
            let mut proto = array_prototype.borrow_mut();

            // Array.prototype.push
            let push_fn = create_function(JsFunction::Native(NativeFunction {
                name: "push".to_string(),
                func: array_push,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("push"), JsValue::Object(push_fn));

            // Array.prototype.pop
            let pop_fn = create_function(JsFunction::Native(NativeFunction {
                name: "pop".to_string(),
                func: array_pop,
                arity: 0,
            }));
            proto.set_property(PropertyKey::from("pop"), JsValue::Object(pop_fn));

            // Array.prototype.map
            let map_fn = create_function(JsFunction::Native(NativeFunction {
                name: "map".to_string(),
                func: array_map,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("map"), JsValue::Object(map_fn));

            // Array.prototype.filter
            let filter_fn = create_function(JsFunction::Native(NativeFunction {
                name: "filter".to_string(),
                func: array_filter,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("filter"), JsValue::Object(filter_fn));

            // Array.prototype.forEach
            let foreach_fn = create_function(JsFunction::Native(NativeFunction {
                name: "forEach".to_string(),
                func: array_foreach,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("forEach"), JsValue::Object(foreach_fn));

            // Array.prototype.reduce
            let reduce_fn = create_function(JsFunction::Native(NativeFunction {
                name: "reduce".to_string(),
                func: array_reduce,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("reduce"), JsValue::Object(reduce_fn));

            // Array.prototype.find
            let find_fn = create_function(JsFunction::Native(NativeFunction {
                name: "find".to_string(),
                func: array_find,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("find"), JsValue::Object(find_fn));

            // Array.prototype.findIndex
            let findindex_fn = create_function(JsFunction::Native(NativeFunction {
                name: "findIndex".to_string(),
                func: array_find_index,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("findIndex"), JsValue::Object(findindex_fn));

            // Array.prototype.indexOf
            let indexof_fn = create_function(JsFunction::Native(NativeFunction {
                name: "indexOf".to_string(),
                func: array_index_of,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("indexOf"), JsValue::Object(indexof_fn));

            // Array.prototype.includes
            let includes_fn = create_function(JsFunction::Native(NativeFunction {
                name: "includes".to_string(),
                func: array_includes,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("includes"), JsValue::Object(includes_fn));

            // Array.prototype.slice
            let slice_fn = create_function(JsFunction::Native(NativeFunction {
                name: "slice".to_string(),
                func: array_slice,
                arity: 2,
            }));
            proto.set_property(PropertyKey::from("slice"), JsValue::Object(slice_fn));

            // Array.prototype.concat
            let concat_fn = create_function(JsFunction::Native(NativeFunction {
                name: "concat".to_string(),
                func: array_concat,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("concat"), JsValue::Object(concat_fn));

            // Array.prototype.join
            let join_fn = create_function(JsFunction::Native(NativeFunction {
                name: "join".to_string(),
                func: array_join,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("join"), JsValue::Object(join_fn));

            // Array.prototype.every
            let every_fn = create_function(JsFunction::Native(NativeFunction {
                name: "every".to_string(),
                func: array_every,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("every"), JsValue::Object(every_fn));

            // Array.prototype.some
            let some_fn = create_function(JsFunction::Native(NativeFunction {
                name: "some".to_string(),
                func: array_some,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("some"), JsValue::Object(some_fn));

            // Array.prototype.shift
            let shift_fn = create_function(JsFunction::Native(NativeFunction {
                name: "shift".to_string(),
                func: array_shift,
                arity: 0,
            }));
            proto.set_property(PropertyKey::from("shift"), JsValue::Object(shift_fn));

            // Array.prototype.unshift
            let unshift_fn = create_function(JsFunction::Native(NativeFunction {
                name: "unshift".to_string(),
                func: array_unshift,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("unshift"), JsValue::Object(unshift_fn));

            // Array.prototype.reverse
            let reverse_fn = create_function(JsFunction::Native(NativeFunction {
                name: "reverse".to_string(),
                func: array_reverse,
                arity: 0,
            }));
            proto.set_property(PropertyKey::from("reverse"), JsValue::Object(reverse_fn));

            // Array.prototype.sort
            let sort_fn = create_function(JsFunction::Native(NativeFunction {
                name: "sort".to_string(),
                func: array_sort,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("sort"), JsValue::Object(sort_fn));

            // Array.prototype.fill
            let fill_fn = create_function(JsFunction::Native(NativeFunction {
                name: "fill".to_string(),
                func: array_fill,
                arity: 3,
            }));
            proto.set_property(PropertyKey::from("fill"), JsValue::Object(fill_fn));

            // Array.prototype.copyWithin
            let copywithin_fn = create_function(JsFunction::Native(NativeFunction {
                name: "copyWithin".to_string(),
                func: array_copy_within,
                arity: 3,
            }));
            proto.set_property(PropertyKey::from("copyWithin"), JsValue::Object(copywithin_fn));

            // Array.prototype.splice
            let splice_fn = create_function(JsFunction::Native(NativeFunction {
                name: "splice".to_string(),
                func: array_splice,
                arity: 2,
            }));
            proto.set_property(PropertyKey::from("splice"), JsValue::Object(splice_fn));

            // Array.prototype.at
            let at_fn = create_function(JsFunction::Native(NativeFunction {
                name: "at".to_string(),
                func: array_at,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("at"), JsValue::Object(at_fn));

            // Array.prototype.lastIndexOf
            let lastindexof_fn = create_function(JsFunction::Native(NativeFunction {
                name: "lastIndexOf".to_string(),
                func: array_last_index_of,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("lastIndexOf"), JsValue::Object(lastindexof_fn));

            // Array.prototype.reduceRight
            let reduceright_fn = create_function(JsFunction::Native(NativeFunction {
                name: "reduceRight".to_string(),
                func: array_reduce_right,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("reduceRight"), JsValue::Object(reduceright_fn));

            // Array.prototype.flat
            let flat_fn = create_function(JsFunction::Native(NativeFunction {
                name: "flat".to_string(),
                func: array_flat,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("flat"), JsValue::Object(flat_fn));

            // Array.prototype.flatMap
            let flatmap_fn = create_function(JsFunction::Native(NativeFunction {
                name: "flatMap".to_string(),
                func: array_flat_map,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("flatMap"), JsValue::Object(flatmap_fn));

            // Array.prototype.findLast
            let findlast_fn = create_function(JsFunction::Native(NativeFunction {
                name: "findLast".to_string(),
                func: array_find_last,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("findLast"), JsValue::Object(findlast_fn));

            // Array.prototype.findLastIndex
            let findlastindex_fn = create_function(JsFunction::Native(NativeFunction {
                name: "findLastIndex".to_string(),
                func: array_find_last_index,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("findLastIndex"), JsValue::Object(findlastindex_fn));

            // Array.prototype.toReversed
            let toreversed_fn = create_function(JsFunction::Native(NativeFunction {
                name: "toReversed".to_string(),
                func: array_to_reversed,
                arity: 0,
            }));
            proto.set_property(PropertyKey::from("toReversed"), JsValue::Object(toreversed_fn));

            // Array.prototype.toSorted
            let tosorted_fn = create_function(JsFunction::Native(NativeFunction {
                name: "toSorted".to_string(),
                func: array_to_sorted,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("toSorted"), JsValue::Object(tosorted_fn));

            // Array.prototype.toSpliced
            let tospliced_fn = create_function(JsFunction::Native(NativeFunction {
                name: "toSpliced".to_string(),
                func: array_to_spliced,
                arity: 2,
            }));
            proto.set_property(PropertyKey::from("toSpliced"), JsValue::Object(tospliced_fn));

            // Array.prototype.with
            let with_fn = create_function(JsFunction::Native(NativeFunction {
                name: "with".to_string(),
                func: array_with,
                arity: 2,
            }));
            proto.set_property(PropertyKey::from("with"), JsValue::Object(with_fn));
        }

        // Add Array global
        let array_constructor = create_function(JsFunction::Native(NativeFunction {
            name: "Array".to_string(),
            func: array_constructor_fn,
            arity: 0,
        }));
        {
            let mut arr = array_constructor.borrow_mut();

            let is_array_fn = create_function(JsFunction::Native(NativeFunction {
                name: "isArray".to_string(),
                func: array_is_array,
                arity: 1,
            }));
            arr.set_property(PropertyKey::from("isArray"), JsValue::Object(is_array_fn));

            let of_fn = create_function(JsFunction::Native(NativeFunction {
                name: "of".to_string(),
                func: array_of,
                arity: 0,
            }));
            arr.set_property(PropertyKey::from("of"), JsValue::Object(of_fn));

            let from_fn = create_function(JsFunction::Native(NativeFunction {
                name: "from".to_string(),
                func: array_from,
                arity: 1,
            }));
            arr.set_property(PropertyKey::from("from"), JsValue::Object(from_fn));

            // Set Array.prototype
            arr.set_property(PropertyKey::from("prototype"), JsValue::Object(array_prototype.clone()));
        }
        env.define("Array".to_string(), JsValue::Object(array_constructor), false);

        // Create String.prototype with methods
        let string_prototype = create_object();
        {
            let mut proto = string_prototype.borrow_mut();

            // String.prototype.charAt
            let charat_fn = create_function(JsFunction::Native(NativeFunction {
                name: "charAt".to_string(),
                func: string_char_at,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("charAt"), JsValue::Object(charat_fn));

            // String.prototype.indexOf
            let indexof_fn = create_function(JsFunction::Native(NativeFunction {
                name: "indexOf".to_string(),
                func: string_index_of,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("indexOf"), JsValue::Object(indexof_fn));

            // String.prototype.lastIndexOf
            let lastindexof_fn = create_function(JsFunction::Native(NativeFunction {
                name: "lastIndexOf".to_string(),
                func: string_last_index_of,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("lastIndexOf"), JsValue::Object(lastindexof_fn));

            // String.prototype.at
            let at_fn = create_function(JsFunction::Native(NativeFunction {
                name: "at".to_string(),
                func: string_at,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("at"), JsValue::Object(at_fn));

            // String.prototype.includes
            let includes_fn = create_function(JsFunction::Native(NativeFunction {
                name: "includes".to_string(),
                func: string_includes,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("includes"), JsValue::Object(includes_fn));

            // String.prototype.startsWith
            let startswith_fn = create_function(JsFunction::Native(NativeFunction {
                name: "startsWith".to_string(),
                func: string_starts_with,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("startsWith"), JsValue::Object(startswith_fn));

            // String.prototype.endsWith
            let endswith_fn = create_function(JsFunction::Native(NativeFunction {
                name: "endsWith".to_string(),
                func: string_ends_with,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("endsWith"), JsValue::Object(endswith_fn));

            // String.prototype.slice
            let slice_fn = create_function(JsFunction::Native(NativeFunction {
                name: "slice".to_string(),
                func: string_slice,
                arity: 2,
            }));
            proto.set_property(PropertyKey::from("slice"), JsValue::Object(slice_fn));

            // String.prototype.substring
            let substring_fn = create_function(JsFunction::Native(NativeFunction {
                name: "substring".to_string(),
                func: string_substring,
                arity: 2,
            }));
            proto.set_property(PropertyKey::from("substring"), JsValue::Object(substring_fn));

            // String.prototype.toLowerCase
            let tolower_fn = create_function(JsFunction::Native(NativeFunction {
                name: "toLowerCase".to_string(),
                func: string_to_lower_case,
                arity: 0,
            }));
            proto.set_property(PropertyKey::from("toLowerCase"), JsValue::Object(tolower_fn));

            // String.prototype.toUpperCase
            let toupper_fn = create_function(JsFunction::Native(NativeFunction {
                name: "toUpperCase".to_string(),
                func: string_to_upper_case,
                arity: 0,
            }));
            proto.set_property(PropertyKey::from("toUpperCase"), JsValue::Object(toupper_fn));

            // String.prototype.trim
            let trim_fn = create_function(JsFunction::Native(NativeFunction {
                name: "trim".to_string(),
                func: string_trim,
                arity: 0,
            }));
            proto.set_property(PropertyKey::from("trim"), JsValue::Object(trim_fn));

            // String.prototype.trimStart
            let trimstart_fn = create_function(JsFunction::Native(NativeFunction {
                name: "trimStart".to_string(),
                func: string_trim_start,
                arity: 0,
            }));
            proto.set_property(PropertyKey::from("trimStart"), JsValue::Object(trimstart_fn));

            // String.prototype.trimEnd
            let trimend_fn = create_function(JsFunction::Native(NativeFunction {
                name: "trimEnd".to_string(),
                func: string_trim_end,
                arity: 0,
            }));
            proto.set_property(PropertyKey::from("trimEnd"), JsValue::Object(trimend_fn));

            // String.prototype.split
            let split_fn = create_function(JsFunction::Native(NativeFunction {
                name: "split".to_string(),
                func: string_split,
                arity: 2,
            }));
            proto.set_property(PropertyKey::from("split"), JsValue::Object(split_fn));

            // String.prototype.repeat
            let repeat_fn = create_function(JsFunction::Native(NativeFunction {
                name: "repeat".to_string(),
                func: string_repeat,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("repeat"), JsValue::Object(repeat_fn));

            // String.prototype.replace
            let replace_fn = create_function(JsFunction::Native(NativeFunction {
                name: "replace".to_string(),
                func: string_replace,
                arity: 2,
            }));
            proto.set_property(PropertyKey::from("replace"), JsValue::Object(replace_fn));

            // String.prototype.replaceAll
            let replaceall_fn = create_function(JsFunction::Native(NativeFunction {
                name: "replaceAll".to_string(),
                func: string_replace_all,
                arity: 2,
            }));
            proto.set_property(PropertyKey::from("replaceAll"), JsValue::Object(replaceall_fn));

            // String.prototype.padStart
            let padstart_fn = create_function(JsFunction::Native(NativeFunction {
                name: "padStart".to_string(),
                func: string_pad_start,
                arity: 2,
            }));
            proto.set_property(PropertyKey::from("padStart"), JsValue::Object(padstart_fn));

            // String.prototype.padEnd
            let padend_fn = create_function(JsFunction::Native(NativeFunction {
                name: "padEnd".to_string(),
                func: string_pad_end,
                arity: 2,
            }));
            proto.set_property(PropertyKey::from("padEnd"), JsValue::Object(padend_fn));

            // String.prototype.concat
            let concat_fn = create_function(JsFunction::Native(NativeFunction {
                name: "concat".to_string(),
                func: string_concat,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("concat"), JsValue::Object(concat_fn));

            // String.prototype.charCodeAt
            let charcodeat_fn = create_function(JsFunction::Native(NativeFunction {
                name: "charCodeAt".to_string(),
                func: string_char_code_at,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("charCodeAt"), JsValue::Object(charcodeat_fn));
        }

        // Add String global object
        let string_obj = create_object();
        {
            let mut str = string_obj.borrow_mut();

            let fromcharcode_fn = create_function(JsFunction::Native(NativeFunction {
                name: "fromCharCode".to_string(),
                func: string_from_char_code,
                arity: 1,
            }));
            str.set_property(PropertyKey::from("fromCharCode"), JsValue::Object(fromcharcode_fn));

            str.set_property(PropertyKey::from("prototype"), JsValue::Object(string_prototype.clone()));
        }
        env.define("String".to_string(), JsValue::Object(string_obj), false);

        // Create Math object with methods and constants
        let math_object = create_object();
        {
            let mut math = math_object.borrow_mut();

            // Constants
            math.set_property(PropertyKey::from("PI"), JsValue::Number(std::f64::consts::PI));
            math.set_property(PropertyKey::from("E"), JsValue::Number(std::f64::consts::E));
            math.set_property(PropertyKey::from("LN2"), JsValue::Number(std::f64::consts::LN_2));
            math.set_property(PropertyKey::from("LN10"), JsValue::Number(std::f64::consts::LN_10));
            math.set_property(PropertyKey::from("LOG2E"), JsValue::Number(std::f64::consts::LOG2_E));
            math.set_property(PropertyKey::from("LOG10E"), JsValue::Number(std::f64::consts::LOG10_E));
            math.set_property(PropertyKey::from("SQRT2"), JsValue::Number(std::f64::consts::SQRT_2));
            math.set_property(PropertyKey::from("SQRT1_2"), JsValue::Number(std::f64::consts::FRAC_1_SQRT_2));

            // Methods
            let abs_fn = create_function(JsFunction::Native(NativeFunction {
                name: "abs".to_string(),
                func: math_abs,
                arity: 1,
            }));
            math.set_property(PropertyKey::from("abs"), JsValue::Object(abs_fn));

            let floor_fn = create_function(JsFunction::Native(NativeFunction {
                name: "floor".to_string(),
                func: math_floor,
                arity: 1,
            }));
            math.set_property(PropertyKey::from("floor"), JsValue::Object(floor_fn));

            let ceil_fn = create_function(JsFunction::Native(NativeFunction {
                name: "ceil".to_string(),
                func: math_ceil,
                arity: 1,
            }));
            math.set_property(PropertyKey::from("ceil"), JsValue::Object(ceil_fn));

            let round_fn = create_function(JsFunction::Native(NativeFunction {
                name: "round".to_string(),
                func: math_round,
                arity: 1,
            }));
            math.set_property(PropertyKey::from("round"), JsValue::Object(round_fn));

            let trunc_fn = create_function(JsFunction::Native(NativeFunction {
                name: "trunc".to_string(),
                func: math_trunc,
                arity: 1,
            }));
            math.set_property(PropertyKey::from("trunc"), JsValue::Object(trunc_fn));

            let sign_fn = create_function(JsFunction::Native(NativeFunction {
                name: "sign".to_string(),
                func: math_sign,
                arity: 1,
            }));
            math.set_property(PropertyKey::from("sign"), JsValue::Object(sign_fn));

            let min_fn = create_function(JsFunction::Native(NativeFunction {
                name: "min".to_string(),
                func: math_min,
                arity: 2,
            }));
            math.set_property(PropertyKey::from("min"), JsValue::Object(min_fn));

            let max_fn = create_function(JsFunction::Native(NativeFunction {
                name: "max".to_string(),
                func: math_max,
                arity: 2,
            }));
            math.set_property(PropertyKey::from("max"), JsValue::Object(max_fn));

            let pow_fn = create_function(JsFunction::Native(NativeFunction {
                name: "pow".to_string(),
                func: math_pow,
                arity: 2,
            }));
            math.set_property(PropertyKey::from("pow"), JsValue::Object(pow_fn));

            let sqrt_fn = create_function(JsFunction::Native(NativeFunction {
                name: "sqrt".to_string(),
                func: math_sqrt,
                arity: 1,
            }));
            math.set_property(PropertyKey::from("sqrt"), JsValue::Object(sqrt_fn));

            let log_fn = create_function(JsFunction::Native(NativeFunction {
                name: "log".to_string(),
                func: math_log,
                arity: 1,
            }));
            math.set_property(PropertyKey::from("log"), JsValue::Object(log_fn));

            let exp_fn = create_function(JsFunction::Native(NativeFunction {
                name: "exp".to_string(),
                func: math_exp,
                arity: 1,
            }));
            math.set_property(PropertyKey::from("exp"), JsValue::Object(exp_fn));

            let random_fn = create_function(JsFunction::Native(NativeFunction {
                name: "random".to_string(),
                func: math_random,
                arity: 0,
            }));
            math.set_property(PropertyKey::from("random"), JsValue::Object(random_fn));

            let sin_fn = create_function(JsFunction::Native(NativeFunction {
                name: "sin".to_string(),
                func: math_sin,
                arity: 1,
            }));
            math.set_property(PropertyKey::from("sin"), JsValue::Object(sin_fn));

            let cos_fn = create_function(JsFunction::Native(NativeFunction {
                name: "cos".to_string(),
                func: math_cos,
                arity: 1,
            }));
            math.set_property(PropertyKey::from("cos"), JsValue::Object(cos_fn));

            let tan_fn = create_function(JsFunction::Native(NativeFunction {
                name: "tan".to_string(),
                func: math_tan,
                arity: 1,
            }));
            math.set_property(PropertyKey::from("tan"), JsValue::Object(tan_fn));

            let asin_fn = create_function(JsFunction::Native(NativeFunction {
                name: "asin".to_string(),
                func: math_asin,
                arity: 1,
            }));
            math.set_property(PropertyKey::from("asin"), JsValue::Object(asin_fn));

            let acos_fn = create_function(JsFunction::Native(NativeFunction {
                name: "acos".to_string(),
                func: math_acos,
                arity: 1,
            }));
            math.set_property(PropertyKey::from("acos"), JsValue::Object(acos_fn));

            let atan_fn = create_function(JsFunction::Native(NativeFunction {
                name: "atan".to_string(),
                func: math_atan,
                arity: 1,
            }));
            math.set_property(PropertyKey::from("atan"), JsValue::Object(atan_fn));

            let atan2_fn = create_function(JsFunction::Native(NativeFunction {
                name: "atan2".to_string(),
                func: math_atan2,
                arity: 2,
            }));
            math.set_property(PropertyKey::from("atan2"), JsValue::Object(atan2_fn));

            let sinh_fn = create_function(JsFunction::Native(NativeFunction {
                name: "sinh".to_string(),
                func: math_sinh,
                arity: 1,
            }));
            math.set_property(PropertyKey::from("sinh"), JsValue::Object(sinh_fn));

            let cosh_fn = create_function(JsFunction::Native(NativeFunction {
                name: "cosh".to_string(),
                func: math_cosh,
                arity: 1,
            }));
            math.set_property(PropertyKey::from("cosh"), JsValue::Object(cosh_fn));

            let tanh_fn = create_function(JsFunction::Native(NativeFunction {
                name: "tanh".to_string(),
                func: math_tanh,
                arity: 1,
            }));
            math.set_property(PropertyKey::from("tanh"), JsValue::Object(tanh_fn));

            let asinh_fn = create_function(JsFunction::Native(NativeFunction {
                name: "asinh".to_string(),
                func: math_asinh,
                arity: 1,
            }));
            math.set_property(PropertyKey::from("asinh"), JsValue::Object(asinh_fn));

            let acosh_fn = create_function(JsFunction::Native(NativeFunction {
                name: "acosh".to_string(),
                func: math_acosh,
                arity: 1,
            }));
            math.set_property(PropertyKey::from("acosh"), JsValue::Object(acosh_fn));

            let atanh_fn = create_function(JsFunction::Native(NativeFunction {
                name: "atanh".to_string(),
                func: math_atanh,
                arity: 1,
            }));
            math.set_property(PropertyKey::from("atanh"), JsValue::Object(atanh_fn));

            let cbrt_fn = create_function(JsFunction::Native(NativeFunction {
                name: "cbrt".to_string(),
                func: math_cbrt,
                arity: 1,
            }));
            math.set_property(PropertyKey::from("cbrt"), JsValue::Object(cbrt_fn));

            let hypot_fn = create_function(JsFunction::Native(NativeFunction {
                name: "hypot".to_string(),
                func: math_hypot,
                arity: 2,
            }));
            math.set_property(PropertyKey::from("hypot"), JsValue::Object(hypot_fn));

            let log10_fn = create_function(JsFunction::Native(NativeFunction {
                name: "log10".to_string(),
                func: math_log10,
                arity: 1,
            }));
            math.set_property(PropertyKey::from("log10"), JsValue::Object(log10_fn));

            let log2_fn = create_function(JsFunction::Native(NativeFunction {
                name: "log2".to_string(),
                func: math_log2,
                arity: 1,
            }));
            math.set_property(PropertyKey::from("log2"), JsValue::Object(log2_fn));

            let log1p_fn = create_function(JsFunction::Native(NativeFunction {
                name: "log1p".to_string(),
                func: math_log1p,
                arity: 1,
            }));
            math.set_property(PropertyKey::from("log1p"), JsValue::Object(log1p_fn));

            let expm1_fn = create_function(JsFunction::Native(NativeFunction {
                name: "expm1".to_string(),
                func: math_expm1,
                arity: 1,
            }));
            math.set_property(PropertyKey::from("expm1"), JsValue::Object(expm1_fn));
        }
        env.define("Math".to_string(), JsValue::Object(math_object), false);

        // Add global functions
        let parseint_fn = create_function(JsFunction::Native(NativeFunction {
            name: "parseInt".to_string(),
            func: global_parse_int,
            arity: 2,
        }));
        env.define("parseInt".to_string(), JsValue::Object(parseint_fn), false);

        let parsefloat_fn = create_function(JsFunction::Native(NativeFunction {
            name: "parseFloat".to_string(),
            func: global_parse_float,
            arity: 1,
        }));
        env.define("parseFloat".to_string(), JsValue::Object(parsefloat_fn), false);

        // Add global isNaN
        let isnan_fn = create_function(JsFunction::Native(NativeFunction {
            name: "isNaN".to_string(),
            func: global_is_nan,
            arity: 1,
        }));
        env.define("isNaN".to_string(), JsValue::Object(isnan_fn), false);

        // Add global isFinite
        let isfinite_fn = create_function(JsFunction::Native(NativeFunction {
            name: "isFinite".to_string(),
            func: global_is_finite,
            arity: 1,
        }));
        env.define("isFinite".to_string(), JsValue::Object(isfinite_fn), false);

        // Add URI encoding/decoding functions
        let encodeuri_fn = create_function(JsFunction::Native(NativeFunction {
            name: "encodeURI".to_string(),
            func: global_encode_uri,
            arity: 1,
        }));
        env.define("encodeURI".to_string(), JsValue::Object(encodeuri_fn), false);

        let decodeuri_fn = create_function(JsFunction::Native(NativeFunction {
            name: "decodeURI".to_string(),
            func: global_decode_uri,
            arity: 1,
        }));
        env.define("decodeURI".to_string(), JsValue::Object(decodeuri_fn), false);

        let encodeuricomponent_fn = create_function(JsFunction::Native(NativeFunction {
            name: "encodeURIComponent".to_string(),
            func: global_encode_uri_component,
            arity: 1,
        }));
        env.define("encodeURIComponent".to_string(), JsValue::Object(encodeuricomponent_fn), false);

        let decodeuricomponent_fn = create_function(JsFunction::Native(NativeFunction {
            name: "decodeURIComponent".to_string(),
            func: global_decode_uri_component,
            arity: 1,
        }));
        env.define("decodeURIComponent".to_string(), JsValue::Object(decodeuricomponent_fn), false);

        // Add Number object
        let number_proto = create_object();
        {
            let mut proto = number_proto.borrow_mut();

            let tofixed_fn = create_function(JsFunction::Native(NativeFunction {
                name: "toFixed".to_string(),
                func: number_to_fixed,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("toFixed"), JsValue::Object(tofixed_fn));

            let tostring_fn = create_function(JsFunction::Native(NativeFunction {
                name: "toString".to_string(),
                func: number_to_string,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("toString"), JsValue::Object(tostring_fn));

            let toprecision_fn = create_function(JsFunction::Native(NativeFunction {
                name: "toPrecision".to_string(),
                func: number_to_precision,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("toPrecision"), JsValue::Object(toprecision_fn));

            let toexponential_fn = create_function(JsFunction::Native(NativeFunction {
                name: "toExponential".to_string(),
                func: number_to_exponential,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("toExponential"), JsValue::Object(toexponential_fn));
        }

        let number_obj = create_object();
        {
            let mut num = number_obj.borrow_mut();

            // Static methods
            let isnan_fn = create_function(JsFunction::Native(NativeFunction {
                name: "isNaN".to_string(),
                func: number_is_nan,
                arity: 1,
            }));
            num.set_property(PropertyKey::from("isNaN"), JsValue::Object(isnan_fn));

            let isfinite_fn = create_function(JsFunction::Native(NativeFunction {
                name: "isFinite".to_string(),
                func: number_is_finite,
                arity: 1,
            }));
            num.set_property(PropertyKey::from("isFinite"), JsValue::Object(isfinite_fn));

            let isinteger_fn = create_function(JsFunction::Native(NativeFunction {
                name: "isInteger".to_string(),
                func: number_is_integer,
                arity: 1,
            }));
            num.set_property(PropertyKey::from("isInteger"), JsValue::Object(isinteger_fn));

            let issafeinteger_fn = create_function(JsFunction::Native(NativeFunction {
                name: "isSafeInteger".to_string(),
                func: number_is_safe_integer,
                arity: 1,
            }));
            num.set_property(PropertyKey::from("isSafeInteger"), JsValue::Object(issafeinteger_fn));

            let parseint_fn = create_function(JsFunction::Native(NativeFunction {
                name: "parseInt".to_string(),
                func: global_parse_int,
                arity: 2,
            }));
            num.set_property(PropertyKey::from("parseInt"), JsValue::Object(parseint_fn));

            let parsefloat_fn = create_function(JsFunction::Native(NativeFunction {
                name: "parseFloat".to_string(),
                func: global_parse_float,
                arity: 1,
            }));
            num.set_property(PropertyKey::from("parseFloat"), JsValue::Object(parsefloat_fn));

            // Constants
            num.set_property(PropertyKey::from("POSITIVE_INFINITY"), JsValue::Number(f64::INFINITY));
            num.set_property(PropertyKey::from("NEGATIVE_INFINITY"), JsValue::Number(f64::NEG_INFINITY));
            num.set_property(PropertyKey::from("MAX_VALUE"), JsValue::Number(f64::MAX));
            num.set_property(PropertyKey::from("MIN_VALUE"), JsValue::Number(f64::MIN_POSITIVE));
            num.set_property(PropertyKey::from("MAX_SAFE_INTEGER"), JsValue::Number(9007199254740991.0));
            num.set_property(PropertyKey::from("MIN_SAFE_INTEGER"), JsValue::Number(-9007199254740991.0));
            num.set_property(PropertyKey::from("EPSILON"), JsValue::Number(f64::EPSILON));
            num.set_property(PropertyKey::from("NaN"), JsValue::Number(f64::NAN));

            num.set_property(PropertyKey::from("prototype"), JsValue::Object(number_proto.clone()));
        }
        env.define("Number".to_string(), JsValue::Object(number_obj), false);

        // Add Error constructors
        let error_fn = create_function(JsFunction::Native(NativeFunction {
            name: "Error".to_string(),
            func: error_constructor,
            arity: 1,
        }));
        env.define("Error".to_string(), JsValue::Object(error_fn), false);

        let type_error_fn = create_function(JsFunction::Native(NativeFunction {
            name: "TypeError".to_string(),
            func: type_error_constructor,
            arity: 1,
        }));
        env.define("TypeError".to_string(), JsValue::Object(type_error_fn), false);

        let reference_error_fn = create_function(JsFunction::Native(NativeFunction {
            name: "ReferenceError".to_string(),
            func: reference_error_constructor,
            arity: 1,
        }));
        env.define("ReferenceError".to_string(), JsValue::Object(reference_error_fn), false);

        let syntax_error_fn = create_function(JsFunction::Native(NativeFunction {
            name: "SyntaxError".to_string(),
            func: syntax_error_constructor,
            arity: 1,
        }));
        env.define("SyntaxError".to_string(), JsValue::Object(syntax_error_fn), false);

        let range_error_fn = create_function(JsFunction::Native(NativeFunction {
            name: "RangeError".to_string(),
            func: range_error_constructor,
            arity: 1,
        }));
        env.define("RangeError".to_string(), JsValue::Object(range_error_fn), false);

        // Create Function.prototype with call, apply, bind
        let function_prototype = create_object();
        {
            let mut proto = function_prototype.borrow_mut();

            let call_fn = create_function(JsFunction::Native(NativeFunction {
                name: "call".to_string(),
                func: function_call,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("call"), JsValue::Object(call_fn));

            let apply_fn = create_function(JsFunction::Native(NativeFunction {
                name: "apply".to_string(),
                func: function_apply,
                arity: 2,
            }));
            proto.set_property(PropertyKey::from("apply"), JsValue::Object(apply_fn));

            let bind_fn = create_function(JsFunction::Native(NativeFunction {
                name: "bind".to_string(),
                func: function_bind,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("bind"), JsValue::Object(bind_fn));
        }

        // Create Map.prototype with Map methods
        let map_prototype = create_object();
        {
            let mut proto = map_prototype.borrow_mut();

            let get_fn = create_function(JsFunction::Native(NativeFunction {
                name: "get".to_string(),
                func: map_get,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("get"), JsValue::Object(get_fn));

            let set_fn = create_function(JsFunction::Native(NativeFunction {
                name: "set".to_string(),
                func: map_set,
                arity: 2,
            }));
            proto.set_property(PropertyKey::from("set"), JsValue::Object(set_fn));

            let has_fn = create_function(JsFunction::Native(NativeFunction {
                name: "has".to_string(),
                func: map_has,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("has"), JsValue::Object(has_fn));

            let delete_fn = create_function(JsFunction::Native(NativeFunction {
                name: "delete".to_string(),
                func: map_delete,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("delete"), JsValue::Object(delete_fn));

            let clear_fn = create_function(JsFunction::Native(NativeFunction {
                name: "clear".to_string(),
                func: map_clear,
                arity: 0,
            }));
            proto.set_property(PropertyKey::from("clear"), JsValue::Object(clear_fn));

            let foreach_fn = create_function(JsFunction::Native(NativeFunction {
                name: "forEach".to_string(),
                func: map_foreach,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("forEach"), JsValue::Object(foreach_fn));
        }

        // Add Map constructor
        let map_constructor_fn = create_function(JsFunction::Native(NativeFunction {
            name: "Map".to_string(),
            func: map_constructor,
            arity: 0,
        }));
        env.define("Map".to_string(), JsValue::Object(map_constructor_fn), false);

        // Create Set.prototype with Set methods
        let set_prototype = create_object();
        {
            let mut proto = set_prototype.borrow_mut();

            let add_fn = create_function(JsFunction::Native(NativeFunction {
                name: "add".to_string(),
                func: set_add,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("add"), JsValue::Object(add_fn));

            let has_fn = create_function(JsFunction::Native(NativeFunction {
                name: "has".to_string(),
                func: set_has,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("has"), JsValue::Object(has_fn));

            let delete_fn = create_function(JsFunction::Native(NativeFunction {
                name: "delete".to_string(),
                func: set_delete,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("delete"), JsValue::Object(delete_fn));

            let clear_fn = create_function(JsFunction::Native(NativeFunction {
                name: "clear".to_string(),
                func: set_clear,
                arity: 0,
            }));
            proto.set_property(PropertyKey::from("clear"), JsValue::Object(clear_fn));

            let foreach_fn = create_function(JsFunction::Native(NativeFunction {
                name: "forEach".to_string(),
                func: set_foreach,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("forEach"), JsValue::Object(foreach_fn));
        }

        // Add Set constructor
        let set_constructor_fn = create_function(JsFunction::Native(NativeFunction {
            name: "Set".to_string(),
            func: set_constructor,
            arity: 0,
        }));
        env.define("Set".to_string(), JsValue::Object(set_constructor_fn), false);

        // Create Date.prototype with Date methods
        let date_prototype = create_object();
        {
            let mut proto = date_prototype.borrow_mut();

            let get_time_fn = create_function(JsFunction::Native(NativeFunction {
                name: "getTime".to_string(),
                func: date_get_time,
                arity: 0,
            }));
            proto.set_property(PropertyKey::from("getTime"), JsValue::Object(get_time_fn));

            let get_full_year_fn = create_function(JsFunction::Native(NativeFunction {
                name: "getFullYear".to_string(),
                func: date_get_full_year,
                arity: 0,
            }));
            proto.set_property(PropertyKey::from("getFullYear"), JsValue::Object(get_full_year_fn));

            let get_month_fn = create_function(JsFunction::Native(NativeFunction {
                name: "getMonth".to_string(),
                func: date_get_month,
                arity: 0,
            }));
            proto.set_property(PropertyKey::from("getMonth"), JsValue::Object(get_month_fn));

            let get_date_fn = create_function(JsFunction::Native(NativeFunction {
                name: "getDate".to_string(),
                func: date_get_date,
                arity: 0,
            }));
            proto.set_property(PropertyKey::from("getDate"), JsValue::Object(get_date_fn));

            let get_day_fn = create_function(JsFunction::Native(NativeFunction {
                name: "getDay".to_string(),
                func: date_get_day,
                arity: 0,
            }));
            proto.set_property(PropertyKey::from("getDay"), JsValue::Object(get_day_fn));

            let get_hours_fn = create_function(JsFunction::Native(NativeFunction {
                name: "getHours".to_string(),
                func: date_get_hours,
                arity: 0,
            }));
            proto.set_property(PropertyKey::from("getHours"), JsValue::Object(get_hours_fn));

            let get_minutes_fn = create_function(JsFunction::Native(NativeFunction {
                name: "getMinutes".to_string(),
                func: date_get_minutes,
                arity: 0,
            }));
            proto.set_property(PropertyKey::from("getMinutes"), JsValue::Object(get_minutes_fn));

            let get_seconds_fn = create_function(JsFunction::Native(NativeFunction {
                name: "getSeconds".to_string(),
                func: date_get_seconds,
                arity: 0,
            }));
            proto.set_property(PropertyKey::from("getSeconds"), JsValue::Object(get_seconds_fn));

            let get_milliseconds_fn = create_function(JsFunction::Native(NativeFunction {
                name: "getMilliseconds".to_string(),
                func: date_get_milliseconds,
                arity: 0,
            }));
            proto.set_property(PropertyKey::from("getMilliseconds"), JsValue::Object(get_milliseconds_fn));

            let to_iso_string_fn = create_function(JsFunction::Native(NativeFunction {
                name: "toISOString".to_string(),
                func: date_to_iso_string,
                arity: 0,
            }));
            proto.set_property(PropertyKey::from("toISOString"), JsValue::Object(to_iso_string_fn));

            let to_json_fn = create_function(JsFunction::Native(NativeFunction {
                name: "toJSON".to_string(),
                func: date_to_iso_string, // toJSON returns the same as toISOString
                arity: 0,
            }));
            proto.set_property(PropertyKey::from("toJSON"), JsValue::Object(to_json_fn));

            let value_of_fn = create_function(JsFunction::Native(NativeFunction {
                name: "valueOf".to_string(),
                func: date_get_time, // valueOf returns the same as getTime
                arity: 0,
            }));
            proto.set_property(PropertyKey::from("valueOf"), JsValue::Object(value_of_fn));
        }

        // Add Date constructor with static methods
        let date_obj = create_function(JsFunction::Native(NativeFunction {
            name: "Date".to_string(),
            func: date_constructor,
            arity: 0,
        }));
        {
            let mut date = date_obj.borrow_mut();

            let now_fn = create_function(JsFunction::Native(NativeFunction {
                name: "now".to_string(),
                func: date_now,
                arity: 0,
            }));
            date.set_property(PropertyKey::from("now"), JsValue::Object(now_fn));

            let utc_fn = create_function(JsFunction::Native(NativeFunction {
                name: "UTC".to_string(),
                func: date_utc,
                arity: 7,
            }));
            date.set_property(PropertyKey::from("UTC"), JsValue::Object(utc_fn));

            let parse_fn = create_function(JsFunction::Native(NativeFunction {
                name: "parse".to_string(),
                func: date_parse,
                arity: 1,
            }));
            date.set_property(PropertyKey::from("parse"), JsValue::Object(parse_fn));

            date.set_property(PropertyKey::from("prototype"), JsValue::Object(date_prototype.clone()));
        }
        env.define("Date".to_string(), JsValue::Object(date_obj), false);

        // Create RegExp.prototype with RegExp methods
        let regexp_prototype = create_object();
        {
            let mut proto = regexp_prototype.borrow_mut();

            let test_fn = create_function(JsFunction::Native(NativeFunction {
                name: "test".to_string(),
                func: regexp_test,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("test"), JsValue::Object(test_fn));

            let exec_fn = create_function(JsFunction::Native(NativeFunction {
                name: "exec".to_string(),
                func: regexp_exec,
                arity: 1,
            }));
            proto.set_property(PropertyKey::from("exec"), JsValue::Object(exec_fn));
        }

        // Add RegExp constructor
        let regexp_constructor_fn = create_function(JsFunction::Native(NativeFunction {
            name: "RegExp".to_string(),
            func: regexp_constructor,
            arity: 2,
        }));
        {
            let mut re = regexp_constructor_fn.borrow_mut();
            re.set_property(PropertyKey::from("prototype"), JsValue::Object(regexp_prototype.clone()));
        }
        env.define("RegExp".to_string(), JsValue::Object(regexp_constructor_fn), false);

        Self {
            global,
            env,
            object_prototype,
            array_prototype,
            string_prototype,
            number_prototype: number_proto,
            function_prototype,
            map_prototype,
            set_prototype,
            date_prototype,
            regexp_prototype,
        }
    }

    /// Create an array with the proper prototype
    pub fn create_array(&self, elements: Vec<JsValue>) -> JsObjectRef {
        let arr = create_array(elements);
        arr.borrow_mut().prototype = Some(self.array_prototype.clone());
        arr
    }

    /// Execute a program
    pub fn execute(&mut self, program: &Program) -> Result<JsValue, JsError> {
        let mut result = JsValue::Undefined;

        for stmt in &program.body {
            match self.execute_statement(stmt)? {
                Completion::Normal(val) => result = val,
                Completion::Return(val) => return Ok(val),
                Completion::Break(_) => {
                    return Err(JsError::syntax_error("Illegal break statement", 0, 0));
                }
                Completion::Continue(_) => {
                    return Err(JsError::syntax_error("Illegal continue statement", 0, 0));
                }
            }
        }

        Ok(result)
    }

    /// Execute a statement
    pub fn execute_statement(&mut self, stmt: &Statement) -> Result<Completion, JsError> {
        match stmt {
            Statement::Expression(expr) => {
                let value = self.evaluate(&expr.expression)?;
                Ok(Completion::Normal(value))
            }

            Statement::VariableDeclaration(decl) => {
                self.execute_variable_declaration(decl)?;
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::FunctionDeclaration(decl) => {
                self.execute_function_declaration(decl)?;
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::Block(block) => self.execute_block(block),

            Statement::If(if_stmt) => {
                let test = self.evaluate(&if_stmt.test)?;
                if test.to_boolean() {
                    self.execute_statement(&if_stmt.consequent)
                } else if let Some(alt) = &if_stmt.alternate {
                    self.execute_statement(alt)
                } else {
                    Ok(Completion::Normal(JsValue::Undefined))
                }
            }

            Statement::While(while_stmt) => {
                loop {
                    let test = self.evaluate(&while_stmt.test)?;
                    if !test.to_boolean() {
                        break;
                    }

                    match self.execute_statement(&while_stmt.body)? {
                        Completion::Break(_) => break,
                        Completion::Continue(_) => continue,
                        Completion::Return(val) => return Ok(Completion::Return(val)),
                        Completion::Normal(_) => {}
                    }
                }
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::DoWhile(do_while) => {
                loop {
                    match self.execute_statement(&do_while.body)? {
                        Completion::Break(_) => break,
                        Completion::Continue(_) => {}
                        Completion::Return(val) => return Ok(Completion::Return(val)),
                        Completion::Normal(_) => {}
                    }

                    let test = self.evaluate(&do_while.test)?;
                    if !test.to_boolean() {
                        break;
                    }
                }
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::For(for_stmt) => self.execute_for(for_stmt),

            Statement::ForIn(for_in) => self.execute_for_in(for_in),

            Statement::ForOf(for_of) => self.execute_for_of(for_of),

            Statement::Return(ret) => {
                let value = if let Some(arg) = &ret.argument {
                    self.evaluate(arg)?
                } else {
                    JsValue::Undefined
                };
                Ok(Completion::Return(value))
            }

            Statement::Break(brk) => {
                Ok(Completion::Break(brk.label.as_ref().map(|l| l.name.clone())))
            }

            Statement::Continue(cont) => {
                Ok(Completion::Continue(cont.label.as_ref().map(|l| l.name.clone())))
            }

            Statement::Throw(throw) => {
                let value = self.evaluate(&throw.argument)?;
                Err(JsError::RuntimeError {
                    kind: "Error".to_string(),
                    message: value.to_js_string().to_string(),
                    stack: vec![],
                })
            }

            Statement::Try(try_stmt) => {
                let result = self.execute_block(&try_stmt.block);

                match result {
                    Ok(completion) => {
                        if let Some(finalizer) = &try_stmt.finalizer {
                            self.execute_block(finalizer)?;
                        }
                        Ok(completion)
                    }
                    Err(err) => {
                        if let Some(handler) = &try_stmt.handler {
                            // Create error value
                            let error_value = JsValue::from(err.to_string());

                            // Bind catch parameter
                            let prev_env = self.env.clone();
                            self.env = Environment::with_outer(self.env.clone());

                            if let Some(param) = &handler.param {
                                self.bind_pattern(param, error_value, true)?;
                            }

                            let result = self.execute_block(&handler.body);
                            self.env = prev_env;

                            if let Some(finalizer) = &try_stmt.finalizer {
                                self.execute_block(finalizer)?;
                            }

                            result
                        } else if let Some(finalizer) = &try_stmt.finalizer {
                            self.execute_block(finalizer)?;
                            Err(err)
                        } else {
                            Err(err)
                        }
                    }
                }
            }

            Statement::Switch(switch) => {
                let discriminant = self.evaluate(&switch.discriminant)?;
                let mut matched = false;
                let mut default_index = None;

                // Find matching case or default
                for (i, case) in switch.cases.iter().enumerate() {
                    if case.test.is_none() {
                        default_index = Some(i);
                        continue;
                    }

                    if !matched {
                        let test = self.evaluate(case.test.as_ref().unwrap())?;
                        if discriminant.strict_equals(&test) {
                            matched = true;
                        }
                    }

                    if matched {
                        for stmt in &case.consequent {
                            match self.execute_statement(stmt)? {
                                Completion::Break(_) => return Ok(Completion::Normal(JsValue::Undefined)),
                                Completion::Return(val) => return Ok(Completion::Return(val)),
                                Completion::Continue(label) => return Ok(Completion::Continue(label)),
                                Completion::Normal(_) => {}
                            }
                        }
                    }
                }

                // Fall through to default if no match
                if !matched {
                    if let Some(idx) = default_index {
                        for case in switch.cases.iter().skip(idx) {
                            for stmt in &case.consequent {
                                match self.execute_statement(stmt)? {
                                    Completion::Break(_) => return Ok(Completion::Normal(JsValue::Undefined)),
                                    Completion::Return(val) => return Ok(Completion::Return(val)),
                                    Completion::Continue(label) => return Ok(Completion::Continue(label)),
                                    Completion::Normal(_) => {}
                                }
                            }
                        }
                    }
                }

                Ok(Completion::Normal(JsValue::Undefined))
            }

            // TypeScript declarations - no-ops at runtime
            Statement::TypeAlias(_) | Statement::InterfaceDeclaration(_) => {
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::EnumDeclaration(enum_decl) => {
                self.execute_enum(enum_decl)?;
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::ClassDeclaration(class) => {
                self.execute_class_declaration(class)?;
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::Import(_) | Statement::Export(_) => {
                // Module handling would go here
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::Empty | Statement::Debugger => {
                Ok(Completion::Normal(JsValue::Undefined))
            }

            Statement::Labeled(labeled) => {
                self.execute_statement(&labeled.body)
            }
        }
    }

    fn execute_variable_declaration(&mut self, decl: &VariableDeclaration) -> Result<(), JsError> {
        let mutable = decl.kind != VariableKind::Const;

        for declarator in &decl.declarations {
            let value = if let Some(init) = &declarator.init {
                self.evaluate(init)?
            } else {
                JsValue::Undefined
            };

            self.bind_pattern(&declarator.id, value, mutable)?;
        }

        Ok(())
    }

    fn execute_function_declaration(&mut self, decl: &FunctionDeclaration) -> Result<(), JsError> {
        let func = InterpretedFunction {
            name: decl.id.as_ref().map(|id| id.name.clone()),
            params: decl.params.clone(),
            body: FunctionBody::Block(decl.body.clone()),
            closure: self.env.clone(),
            source_location: decl.span,
        };

        let func_obj = create_function(JsFunction::Interpreted(func));

        if let Some(id) = &decl.id {
            self.env.define(id.name.clone(), JsValue::Object(func_obj), true);
        }

        Ok(())
    }

    fn execute_class_declaration(&mut self, _class: &ClassDeclaration) -> Result<(), JsError> {
        // Simplified class handling - create constructor function
        // Full implementation would handle methods, static members, etc.
        Ok(())
    }

    fn execute_enum(&mut self, enum_decl: &EnumDeclaration) -> Result<(), JsError> {
        let obj = create_object();
        let mut next_value = 0i32;

        for member in &enum_decl.members {
            let value = if let Some(init) = &member.initializer {
                let val = self.evaluate(init)?;
                if let JsValue::Number(n) = val {
                    next_value = n as i32 + 1;
                }
                val
            } else {
                let val = JsValue::Number(next_value as f64);
                next_value += 1;
                val
            };

            // Forward mapping: name -> value
            obj.borrow_mut().set_property(
                PropertyKey::from(member.id.name.as_str()),
                value.clone(),
            );

            // Reverse mapping for numeric enums: value -> name
            if let JsValue::Number(n) = &value {
                obj.borrow_mut().set_property(
                    PropertyKey::from(n.to_string()),
                    JsValue::String(JsString::from(member.id.name.clone())),
                );
            }
        }

        self.env.define(enum_decl.id.name.clone(), JsValue::Object(obj), false);
        Ok(())
    }

    fn execute_block(&mut self, block: &BlockStatement) -> Result<Completion, JsError> {
        let prev_env = self.env.clone();
        self.env = Environment::with_outer(self.env.clone());

        let mut result = Completion::Normal(JsValue::Undefined);

        for stmt in &block.body {
            result = self.execute_statement(stmt)?;
            match &result {
                Completion::Normal(_) => {}
                _ => break,
            }
        }

        self.env = prev_env;
        Ok(result)
    }

    fn execute_for(&mut self, for_stmt: &ForStatement) -> Result<Completion, JsError> {
        let prev_env = self.env.clone();
        self.env = Environment::with_outer(self.env.clone());

        // Init
        if let Some(init) = &for_stmt.init {
            match init {
                ForInit::Variable(decl) => {
                    self.execute_variable_declaration(decl)?;
                }
                ForInit::Expression(expr) => {
                    self.evaluate(expr)?;
                }
            }
        }

        // Loop
        loop {
            // Test
            if let Some(test) = &for_stmt.test {
                let test_val = self.evaluate(test)?;
                if !test_val.to_boolean() {
                    break;
                }
            }

            // Body
            match self.execute_statement(&for_stmt.body)? {
                Completion::Break(_) => break,
                Completion::Continue(_) => {}
                Completion::Return(val) => {
                    self.env = prev_env;
                    return Ok(Completion::Return(val));
                }
                Completion::Normal(_) => {}
            }

            // Update
            if let Some(update) = &for_stmt.update {
                self.evaluate(update)?;
            }
        }

        self.env = prev_env;
        Ok(Completion::Normal(JsValue::Undefined))
    }

    fn execute_for_in(&mut self, for_in: &ForInStatement) -> Result<Completion, JsError> {
        let right = self.evaluate(&for_in.right)?;

        let keys = match &right {
            JsValue::Object(obj) => {
                obj.borrow()
                    .properties
                    .iter()
                    .filter(|(_, prop)| prop.enumerable)
                    .map(|(key, _)| key.to_string())
                    .collect::<Vec<_>>()
            }
            _ => vec![],
        };

        let prev_env = self.env.clone();

        for key in keys {
            self.env = Environment::with_outer(prev_env.clone());

            let key_value = JsValue::String(JsString::from(key));

            match &for_in.left {
                ForInOfLeft::Variable(decl) => {
                    let mutable = decl.kind != VariableKind::Const;
                    if let Some(declarator) = decl.declarations.first() {
                        self.bind_pattern(&declarator.id, key_value, mutable)?;
                    }
                }
                ForInOfLeft::Pattern(pattern) => {
                    self.bind_pattern(pattern, key_value, true)?;
                }
            }

            match self.execute_statement(&for_in.body)? {
                Completion::Break(_) => break,
                Completion::Continue(_) => continue,
                Completion::Return(val) => {
                    self.env = prev_env;
                    return Ok(Completion::Return(val));
                }
                Completion::Normal(_) => {}
            }
        }

        self.env = prev_env;
        Ok(Completion::Normal(JsValue::Undefined))
    }

    fn execute_for_of(&mut self, for_of: &ForOfStatement) -> Result<Completion, JsError> {
        let right = self.evaluate(&for_of.right)?;

        let items = match &right {
            JsValue::Object(obj) => {
                let obj_ref = obj.borrow();
                match &obj_ref.exotic {
                    ExoticObject::Array { length } => {
                        let mut items = Vec::with_capacity(*length as usize);
                        for i in 0..*length {
                            if let Some(val) = obj_ref.get_property(&PropertyKey::Index(i)) {
                                items.push(val);
                            } else {
                                items.push(JsValue::Undefined);
                            }
                        }
                        items
                    }
                    _ => vec![],
                }
            }
            JsValue::String(s) => {
                s.as_str().chars().map(|c| JsValue::from(c.to_string())).collect()
            }
            _ => vec![],
        };

        let prev_env = self.env.clone();

        for item in items {
            self.env = Environment::with_outer(prev_env.clone());

            match &for_of.left {
                ForInOfLeft::Variable(decl) => {
                    let mutable = decl.kind != VariableKind::Const;
                    if let Some(declarator) = decl.declarations.first() {
                        self.bind_pattern(&declarator.id, item, mutable)?;
                    }
                }
                ForInOfLeft::Pattern(pattern) => {
                    self.bind_pattern(pattern, item, true)?;
                }
            }

            match self.execute_statement(&for_of.body)? {
                Completion::Break(_) => break,
                Completion::Continue(_) => continue,
                Completion::Return(val) => {
                    self.env = prev_env;
                    return Ok(Completion::Return(val));
                }
                Completion::Normal(_) => {}
            }
        }

        self.env = prev_env;
        Ok(Completion::Normal(JsValue::Undefined))
    }

    fn bind_pattern(&mut self, pattern: &Pattern, value: JsValue, mutable: bool) -> Result<(), JsError> {
        match pattern {
            Pattern::Identifier(id) => {
                self.env.define(id.name.clone(), value, mutable);
                Ok(())
            }

            Pattern::Object(obj_pattern) => {
                let obj = match &value {
                    JsValue::Object(o) => o.clone(),
                    _ => return Err(JsError::type_error("Cannot destructure non-object")),
                };

                for prop in &obj_pattern.properties {
                    match prop {
                        ObjectPatternProperty::KeyValue { key, value: pattern, .. } => {
                            let key_str = match key {
                                ObjectPropertyKey::Identifier(id) => id.name.clone(),
                                ObjectPropertyKey::String(s) => s.value.clone(),
                                ObjectPropertyKey::Number(l) => {
                                    if let LiteralValue::Number(n) = &l.value {
                                        n.to_string()
                                    } else {
                                        continue;
                                    }
                                }
                                ObjectPropertyKey::Computed(_) => continue,
                            };

                            let prop_value = obj
                                .borrow()
                                .get_property(&PropertyKey::from(key_str.as_str()))
                                .unwrap_or(JsValue::Undefined);

                            self.bind_pattern(pattern, prop_value, mutable)?;
                        }
                        ObjectPatternProperty::Rest(rest) => {
                            // Collect remaining properties
                            let rest_obj = create_object();
                            // Simplified - would need to track which keys were already destructured
                            self.bind_pattern(&rest.argument, JsValue::Object(rest_obj), mutable)?;
                        }
                    }
                }

                Ok(())
            }

            Pattern::Array(arr_pattern) => {
                let items: Vec<JsValue> = match &value {
                    JsValue::Object(obj) => {
                        let obj_ref = obj.borrow();
                        if let ExoticObject::Array { length } = &obj_ref.exotic {
                            let mut items = Vec::with_capacity(*length as usize);
                            for i in 0..*length {
                                items.push(
                                    obj_ref
                                        .get_property(&PropertyKey::Index(i))
                                        .unwrap_or(JsValue::Undefined),
                                );
                            }
                            items
                        } else {
                            vec![]
                        }
                    }
                    _ => vec![],
                };

                for (i, elem) in arr_pattern.elements.iter().enumerate() {
                    if let Some(pattern) = elem {
                        match pattern {
                            Pattern::Rest(rest) => {
                                let remaining: Vec<JsValue> = items.iter().skip(i).cloned().collect();
                                self.bind_pattern(
                                    &rest.argument,
                                    JsValue::Object(create_array(remaining)),
                                    mutable,
                                )?;
                                break;
                            }
                            _ => {
                                let val = items.get(i).cloned().unwrap_or(JsValue::Undefined);
                                self.bind_pattern(pattern, val, mutable)?;
                            }
                        }
                    }
                }

                Ok(())
            }

            Pattern::Assignment(assign) => {
                let val = if value == JsValue::Undefined {
                    self.evaluate(&assign.right)?
                } else {
                    value
                };
                self.bind_pattern(&assign.left, val, mutable)
            }

            Pattern::Rest(rest) => {
                self.bind_pattern(&rest.argument, value, mutable)
            }
        }
    }

    /// Evaluate an expression
    pub fn evaluate(&mut self, expr: &Expression) -> Result<JsValue, JsError> {
        match expr {
            Expression::Literal(lit) => self.evaluate_literal(&lit.value),

            Expression::Identifier(id) => {
                self.env
                    .get(&id.name)
                    .ok_or_else(|| JsError::reference_error(&id.name))
            }

            Expression::This(_) => {
                // Look up 'this' from the environment
                Ok(self.env.get("this").unwrap_or(JsValue::Undefined))
            }

            Expression::Array(arr) => {
                let mut elements = vec![];
                for elem in &arr.elements {
                    match elem {
                        Some(ArrayElement::Expression(e)) => {
                            elements.push(self.evaluate(e)?);
                        }
                        Some(ArrayElement::Spread(spread)) => {
                            let val = self.evaluate(&spread.argument)?;
                            if let JsValue::Object(obj) = val {
                                let obj_ref = obj.borrow();
                                if let ExoticObject::Array { length } = &obj_ref.exotic {
                                    for i in 0..*length {
                                        if let Some(v) = obj_ref.get_property(&PropertyKey::Index(i)) {
                                            elements.push(v);
                                        }
                                    }
                                }
                            }
                        }
                        None => {
                            elements.push(JsValue::Undefined);
                        }
                    }
                }
                Ok(JsValue::Object(self.create_array(elements)))
            }

            Expression::Object(obj) => {
                let result = create_object();
                for prop in &obj.properties {
                    match prop {
                        ObjectProperty::Property(p) => {
                            let key = self.evaluate_property_key(&p.key)?;
                            let value = if p.method {
                                // Method shorthand - would need to handle this specially
                                self.evaluate(&p.value)?
                            } else {
                                self.evaluate(&p.value)?
                            };
                            result.borrow_mut().set_property(key, value);
                        }
                        ObjectProperty::Spread(spread) => {
                            let val = self.evaluate(&spread.argument)?;
                            if let JsValue::Object(src) = val {
                                let src_ref = src.borrow();
                                for (key, prop) in src_ref.properties.iter() {
                                    if prop.enumerable {
                                        result.borrow_mut().set_property(key.clone(), prop.value.clone());
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(JsValue::Object(result))
            }

            Expression::Function(func) => {
                let interpreted = InterpretedFunction {
                    name: func.id.as_ref().map(|id| id.name.clone()),
                    params: func.params.clone(),
                    body: FunctionBody::Block(func.body.clone()),
                    closure: self.env.clone(),
                    source_location: func.span,
                };
                Ok(JsValue::Object(create_function(JsFunction::Interpreted(interpreted))))
            }

            Expression::ArrowFunction(arrow) => {
                let interpreted = InterpretedFunction {
                    name: None,
                    params: arrow.params.clone(),
                    body: arrow.body.clone().into(),
                    closure: self.env.clone(),
                    source_location: arrow.span,
                };
                Ok(JsValue::Object(create_function(JsFunction::Interpreted(interpreted))))
            }

            Expression::Unary(unary) => self.evaluate_unary(unary),
            Expression::Binary(binary) => self.evaluate_binary(binary),
            Expression::Logical(logical) => self.evaluate_logical(logical),
            Expression::Conditional(cond) => self.evaluate_conditional(cond),
            Expression::Assignment(assign) => self.evaluate_assignment(assign),
            Expression::Update(update) => self.evaluate_update(update),
            Expression::Member(member) => self.evaluate_member(member),
            Expression::Call(call) => self.evaluate_call(call),
            Expression::New(new) => self.evaluate_new(new),

            Expression::Sequence(seq) => {
                let mut result = JsValue::Undefined;
                for expr in &seq.expressions {
                    result = self.evaluate(expr)?;
                }
                Ok(result)
            }

            Expression::Template(template) => {
                let mut result = String::new();
                for (i, quasi) in template.quasis.iter().enumerate() {
                    result.push_str(&quasi.value);
                    if i < template.expressions.len() {
                        let val = self.evaluate(&template.expressions[i])?;
                        result.push_str(&val.to_js_string().to_string());
                    }
                }
                Ok(JsValue::String(JsString::from(result)))
            }

            Expression::Parenthesized(inner, _) => self.evaluate(inner),

            // TypeScript expressions - evaluate the inner expression
            Expression::TypeAssertion(ta) => self.evaluate(&ta.expression),
            Expression::NonNull(nn) => self.evaluate(&nn.expression),

            Expression::Spread(spread) => self.evaluate(&spread.argument),

            Expression::Await(_) | Expression::Yield(_) => {
                Err(JsError::type_error("Async/generators not supported"))
            }

            Expression::Super(_) | Expression::Class(_) => {
                Err(JsError::type_error("Not implemented"))
            }

            Expression::OptionalChain(chain) => {
                // Simplified optional chain handling
                self.evaluate(&chain.base)
            }
        }
    }

    fn evaluate_literal(&self, value: &LiteralValue) -> Result<JsValue, JsError> {
        Ok(match value {
            LiteralValue::Null => JsValue::Null,
            LiteralValue::Undefined => JsValue::Undefined,
            LiteralValue::Boolean(b) => JsValue::Boolean(*b),
            LiteralValue::Number(n) => JsValue::Number(*n),
            LiteralValue::String(s) => JsValue::String(JsString::from(s.clone())),
            LiteralValue::RegExp { .. } => {
                // Would need RegExp object
                JsValue::Object(create_object())
            }
        })
    }

    fn evaluate_property_key(&mut self, key: &ObjectPropertyKey) -> Result<PropertyKey, JsError> {
        Ok(match key {
            ObjectPropertyKey::Identifier(id) => PropertyKey::from(id.name.as_str()),
            ObjectPropertyKey::String(s) => PropertyKey::from(s.value.as_str()),
            ObjectPropertyKey::Number(lit) => {
                if let LiteralValue::Number(n) = &lit.value {
                    PropertyKey::from_value(&JsValue::Number(*n))
                } else {
                    PropertyKey::from("undefined")
                }
            }
            ObjectPropertyKey::Computed(expr) => {
                let val = self.evaluate(expr)?;
                PropertyKey::from_value(&val)
            }
        })
    }

    fn evaluate_unary(&mut self, unary: &UnaryExpression) -> Result<JsValue, JsError> {
        let arg = self.evaluate(&unary.argument)?;

        Ok(match unary.operator {
            UnaryOp::Minus => JsValue::Number(-arg.to_number()),
            UnaryOp::Plus => JsValue::Number(arg.to_number()),
            UnaryOp::Not => JsValue::Boolean(!arg.to_boolean()),
            UnaryOp::BitNot => JsValue::Number(!(arg.to_number() as i32) as f64),
            UnaryOp::Typeof => JsValue::String(JsString::from(arg.type_of())),
            UnaryOp::Void => JsValue::Undefined,
            UnaryOp::Delete => {
                // Simplified - would need to actually delete property
                JsValue::Boolean(true)
            }
        })
    }

    fn evaluate_binary(&mut self, binary: &BinaryExpression) -> Result<JsValue, JsError> {
        let left = self.evaluate(&binary.left)?;
        let right = self.evaluate(&binary.right)?;

        Ok(match binary.operator {
            // Arithmetic
            BinaryOp::Add => {
                if left.is_string() || right.is_string() {
                    let ls = left.to_js_string();
                    let rs = right.to_js_string();
                    JsValue::String(ls + &rs)
                } else {
                    JsValue::Number(left.to_number() + right.to_number())
                }
            }
            BinaryOp::Sub => JsValue::Number(left.to_number() - right.to_number()),
            BinaryOp::Mul => JsValue::Number(left.to_number() * right.to_number()),
            BinaryOp::Div => JsValue::Number(left.to_number() / right.to_number()),
            BinaryOp::Mod => JsValue::Number(left.to_number() % right.to_number()),
            BinaryOp::Exp => JsValue::Number(left.to_number().powf(right.to_number())),

            // Comparison
            BinaryOp::Lt => JsValue::Boolean(left.to_number() < right.to_number()),
            BinaryOp::LtEq => JsValue::Boolean(left.to_number() <= right.to_number()),
            BinaryOp::Gt => JsValue::Boolean(left.to_number() > right.to_number()),
            BinaryOp::GtEq => JsValue::Boolean(left.to_number() >= right.to_number()),

            // Equality
            BinaryOp::Eq => {
                // Abstract equality - simplified
                JsValue::Boolean(left.strict_equals(&right))
            }
            BinaryOp::NotEq => JsValue::Boolean(!left.strict_equals(&right)),
            BinaryOp::StrictEq => JsValue::Boolean(left.strict_equals(&right)),
            BinaryOp::StrictNotEq => JsValue::Boolean(!left.strict_equals(&right)),

            // Bitwise
            BinaryOp::BitAnd => JsValue::Number((left.to_number() as i32 & right.to_number() as i32) as f64),
            BinaryOp::BitOr => JsValue::Number((left.to_number() as i32 | right.to_number() as i32) as f64),
            BinaryOp::BitXor => JsValue::Number((left.to_number() as i32 ^ right.to_number() as i32) as f64),
            BinaryOp::LShift => JsValue::Number(((left.to_number() as i32) << (right.to_number() as u32 & 0x1f)) as f64),
            BinaryOp::RShift => JsValue::Number(((left.to_number() as i32) >> (right.to_number() as u32 & 0x1f)) as f64),
            BinaryOp::URShift => JsValue::Number(((left.to_number() as u32) >> (right.to_number() as u32 & 0x1f)) as f64),

            // Other
            BinaryOp::In => {
                if let JsValue::Object(obj) = right {
                    let key = crate::value::PropertyKey::from_value(&left);
                    JsValue::Boolean(obj.borrow().has_own_property(&key))
                } else {
                    return Err(JsError::type_error("Cannot use 'in' operator on non-object"));
                }
            }
            BinaryOp::Instanceof => {
                // Simplified
                JsValue::Boolean(false)
            }
        })
    }

    fn evaluate_logical(&mut self, logical: &LogicalExpression) -> Result<JsValue, JsError> {
        let left = self.evaluate(&logical.left)?;

        match logical.operator {
            LogicalOp::And => {
                if !left.to_boolean() {
                    Ok(left)
                } else {
                    self.evaluate(&logical.right)
                }
            }
            LogicalOp::Or => {
                if left.to_boolean() {
                    Ok(left)
                } else {
                    self.evaluate(&logical.right)
                }
            }
            LogicalOp::NullishCoalescing => {
                if left.is_null_or_undefined() {
                    self.evaluate(&logical.right)
                } else {
                    Ok(left)
                }
            }
        }
    }

    fn evaluate_conditional(&mut self, cond: &ConditionalExpression) -> Result<JsValue, JsError> {
        let test = self.evaluate(&cond.test)?;
        if test.to_boolean() {
            self.evaluate(&cond.consequent)
        } else {
            self.evaluate(&cond.alternate)
        }
    }

    fn evaluate_assignment(&mut self, assign: &AssignmentExpression) -> Result<JsValue, JsError> {
        let right = self.evaluate(&assign.right)?;

        let value = if assign.operator != AssignmentOp::Assign {
            let left = match &assign.left {
                AssignmentTarget::Identifier(id) => self.env.get(&id.name).unwrap_or(JsValue::Undefined),
                AssignmentTarget::Member(member) => self.evaluate_member(member)?,
                AssignmentTarget::Pattern(_) => {
                    return Err(JsError::syntax_error("Invalid assignment target", 0, 0));
                }
            };

            match assign.operator {
                AssignmentOp::AddAssign => {
                    if left.is_string() || right.is_string() {
                        JsValue::String(left.to_js_string() + &right.to_js_string())
                    } else {
                        JsValue::Number(left.to_number() + right.to_number())
                    }
                }
                AssignmentOp::SubAssign => JsValue::Number(left.to_number() - right.to_number()),
                AssignmentOp::MulAssign => JsValue::Number(left.to_number() * right.to_number()),
                AssignmentOp::DivAssign => JsValue::Number(left.to_number() / right.to_number()),
                AssignmentOp::ModAssign => JsValue::Number(left.to_number() % right.to_number()),
                AssignmentOp::ExpAssign => JsValue::Number(left.to_number().powf(right.to_number())),
                AssignmentOp::BitAndAssign => JsValue::Number((left.to_number() as i32 & right.to_number() as i32) as f64),
                AssignmentOp::BitOrAssign => JsValue::Number((left.to_number() as i32 | right.to_number() as i32) as f64),
                AssignmentOp::BitXorAssign => JsValue::Number((left.to_number() as i32 ^ right.to_number() as i32) as f64),
                AssignmentOp::LShiftAssign => JsValue::Number(((left.to_number() as i32) << (right.to_number() as u32 & 0x1f)) as f64),
                AssignmentOp::RShiftAssign => JsValue::Number(((left.to_number() as i32) >> (right.to_number() as u32 & 0x1f)) as f64),
                AssignmentOp::URShiftAssign => JsValue::Number(((left.to_number() as u32) >> (right.to_number() as u32 & 0x1f)) as f64),
                AssignmentOp::AndAssign => {
                    if !left.to_boolean() {
                        left
                    } else {
                        right
                    }
                }
                AssignmentOp::OrAssign => {
                    if left.to_boolean() {
                        left
                    } else {
                        right
                    }
                }
                AssignmentOp::NullishAssign => {
                    if left.is_null_or_undefined() {
                        right
                    } else {
                        left
                    }
                }
                AssignmentOp::Assign => unreachable!(),
            }
        } else {
            right
        };

        match &assign.left {
            AssignmentTarget::Identifier(id) => {
                self.env.set(&id.name, value.clone())?;
            }
            AssignmentTarget::Member(member) => {
                self.set_member(member, value.clone())?;
            }
            AssignmentTarget::Pattern(pattern) => {
                self.bind_pattern(pattern, value.clone(), true)?;
            }
        }

        Ok(value)
    }

    fn evaluate_update(&mut self, update: &UpdateExpression) -> Result<JsValue, JsError> {
        let old_value = self.evaluate(&update.argument)?;
        let old_num = old_value.to_number();

        let new_value = match update.operator {
            UpdateOp::Increment => JsValue::Number(old_num + 1.0),
            UpdateOp::Decrement => JsValue::Number(old_num - 1.0),
        };

        // Set the new value
        match update.argument.as_ref() {
            Expression::Identifier(id) => {
                self.env.set(&id.name, new_value.clone())?;
            }
            Expression::Member(member) => {
                self.set_member(member, new_value.clone())?;
            }
            _ => return Err(JsError::syntax_error("Invalid update target", 0, 0)),
        }

        Ok(if update.prefix { new_value } else { JsValue::Number(old_num) })
    }

    fn evaluate_member(&mut self, member: &MemberExpression) -> Result<JsValue, JsError> {
        let object = self.evaluate(&member.object)?;

        let key = match &member.property {
            MemberProperty::Identifier(id) => crate::value::PropertyKey::from(id.name.as_str()),
            MemberProperty::Expression(expr) => {
                let val = self.evaluate(expr)?;
                crate::value::PropertyKey::from_value(&val)
            }
            MemberProperty::PrivateIdentifier(_) => {
                return Err(JsError::type_error("Private fields not supported"));
            }
        };

        match object {
            JsValue::Object(obj) => {
                // First, try own properties and prototype chain
                if let Some(val) = obj.borrow().get_property(&key) {
                    return Ok(val);
                }
                // For functions, check Function.prototype
                if obj.borrow().is_callable() {
                    if let Some(method) = self.function_prototype.borrow().get_property(&key) {
                        return Ok(method);
                    }
                }
                // Fall back to Object.prototype for ordinary objects
                // (but not for objects created with Object.create(null))
                if !obj.borrow().null_prototype {
                    if let Some(method) = self.object_prototype.borrow().get_property(&key) {
                        return Ok(method);
                    }
                }
                Ok(JsValue::Undefined)
            }
            JsValue::String(s) => {
                // String indexing
                if let crate::value::PropertyKey::Index(i) = key {
                    if let Some(ch) = s.as_str().chars().nth(i as usize) {
                        return Ok(JsValue::String(JsString::from(ch.to_string())));
                    }
                }
                if key.to_string() == "length" {
                    return Ok(JsValue::Number(s.len() as f64));
                }
                // Look up on String.prototype
                if let Some(method) = self.string_prototype.borrow().get_property(&key) {
                    return Ok(method);
                }
                Ok(JsValue::Undefined)
            }
            JsValue::Number(_) => {
                // Look up on Number.prototype
                if let Some(method) = self.number_prototype.borrow().get_property(&key) {
                    return Ok(method);
                }
                Ok(JsValue::Undefined)
            }
            _ => Ok(JsValue::Undefined),
        }
    }

    fn set_member(&mut self, member: &MemberExpression, value: JsValue) -> Result<(), JsError> {
        let object = self.evaluate(&member.object)?;

        let key = match &member.property {
            MemberProperty::Identifier(id) => crate::value::PropertyKey::from(id.name.as_str()),
            MemberProperty::Expression(expr) => {
                let val = self.evaluate(expr)?;
                crate::value::PropertyKey::from_value(&val)
            }
            MemberProperty::PrivateIdentifier(_) => {
                return Err(JsError::type_error("Private fields not supported"));
            }
        };

        match object {
            JsValue::Object(obj) => {
                obj.borrow_mut().set_property(key, value);
                Ok(())
            }
            _ => Err(JsError::type_error("Cannot set property on non-object")),
        }
    }

    fn evaluate_call(&mut self, call: &CallExpression) -> Result<JsValue, JsError> {
        let callee = self.evaluate(&call.callee)?;

        // Determine 'this' binding
        let this_value = if let Expression::Member(member) = call.callee.as_ref() {
            self.evaluate(&member.object)?
        } else {
            JsValue::Undefined
        };

        // Evaluate arguments
        let mut args = vec![];
        for arg in &call.arguments {
            match arg {
                Argument::Expression(expr) => {
                    args.push(self.evaluate(expr)?);
                }
                Argument::Spread(spread) => {
                    let val = self.evaluate(&spread.argument)?;
                    if let JsValue::Object(obj) = val {
                        let obj_ref = obj.borrow();
                        if let ExoticObject::Array { length } = &obj_ref.exotic {
                            for i in 0..*length {
                                if let Some(v) = obj_ref.get_property(&PropertyKey::Index(i)) {
                                    args.push(v);
                                }
                            }
                        }
                    }
                }
            }
        }

        self.call_function(callee, this_value, args)
    }

    fn evaluate_new(&mut self, new_expr: &NewExpression) -> Result<JsValue, JsError> {
        let callee = self.evaluate(&new_expr.callee)?;

        let mut args = vec![];
        for arg in &new_expr.arguments {
            match arg {
                Argument::Expression(expr) => {
                    args.push(self.evaluate(expr)?);
                }
                Argument::Spread(spread) => {
                    let val = self.evaluate(&spread.argument)?;
                    if let JsValue::Object(obj) = val {
                        let obj_ref = obj.borrow();
                        if let ExoticObject::Array { length } = &obj_ref.exotic {
                            for i in 0..*length {
                                if let Some(v) = obj_ref.get_property(&PropertyKey::Index(i)) {
                                    args.push(v);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Create new object
        let new_obj = create_object();

        // Call constructor
        let result = self.call_function(callee, JsValue::Object(new_obj.clone()), args)?;

        // Return result if it's an object, otherwise return new_obj
        match result {
            JsValue::Object(_) => Ok(result),
            _ => Ok(JsValue::Object(new_obj)),
        }
    }

    pub fn call_function(
        &mut self,
        callee: JsValue,
        this_value: JsValue,
        args: Vec<JsValue>,
    ) -> Result<JsValue, JsError> {
        let JsValue::Object(obj) = callee else {
            return Err(JsError::type_error("Not a function"));
        };

        let func = {
            let obj_ref = obj.borrow();
            match &obj_ref.exotic {
                ExoticObject::Function(f) => f.clone(),
                _ => return Err(JsError::type_error("Not a function")),
            }
        };

        match func {
            JsFunction::Interpreted(interpreted) => {
                let prev_env = self.env.clone();
                self.env = Environment::with_outer(interpreted.closure.clone());

                // Bind 'this' value
                self.env.define("this".to_string(), this_value.clone(), false);

                // Bind parameters
                for (i, param) in interpreted.params.iter().enumerate() {
                    let arg = args.get(i).cloned().unwrap_or(JsValue::Undefined);
                    self.bind_pattern(&param.pattern, arg, true)?;
                }

                // Execute body
                let result = match &interpreted.body {
                    FunctionBody::Block(block) => {
                        match self.execute_block(block)? {
                            Completion::Return(val) => val,
                            Completion::Normal(val) => val,
                            _ => JsValue::Undefined,
                        }
                    }
                    FunctionBody::Expression(expr) => self.evaluate(expr)?,
                };

                self.env = prev_env;
                Ok(result)
            }

            JsFunction::Native(native) => {
                (native.func)(self, this_value, args)
            }

            JsFunction::Bound(bound_data) => {
                // For bound functions:
                // - Use the bound this value (ignore the passed this_value)
                // - Prepend bound args to the call args
                let bound_this = bound_data.this_arg.clone();
                let mut full_args = bound_data.bound_args.clone();
                full_args.extend(args);

                // Call the target function with bound this and combined args
                self.call_function(
                    JsValue::Object(bound_data.target.clone()),
                    bound_this,
                    full_args,
                )
            }
        }
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    fn eval(source: &str) -> JsValue {
        let mut parser = Parser::new(source);
        let program = parser.parse_program().unwrap();
        let mut interp = Interpreter::new();
        interp.execute(&program).unwrap()
    }

    #[test]
    fn test_arithmetic() {
        assert_eq!(eval("1 + 2"), JsValue::Number(3.0));
        assert_eq!(eval("10 - 4"), JsValue::Number(6.0));
        assert_eq!(eval("3 * 4"), JsValue::Number(12.0));
        assert_eq!(eval("15 / 3"), JsValue::Number(5.0));
        assert_eq!(eval("2 ** 3"), JsValue::Number(8.0));
    }

    #[test]
    fn test_precedence() {
        assert_eq!(eval("1 + 2 * 3"), JsValue::Number(7.0));
        assert_eq!(eval("(1 + 2) * 3"), JsValue::Number(9.0));
    }

    #[test]
    fn test_comparison() {
        assert_eq!(eval("1 < 2"), JsValue::Boolean(true));
        assert_eq!(eval("2 > 1"), JsValue::Boolean(true));
        assert_eq!(eval("1 === 1"), JsValue::Boolean(true));
        assert_eq!(eval("1 !== 2"), JsValue::Boolean(true));
    }

    #[test]
    fn test_variables() {
        assert_eq!(eval("let x = 5; x"), JsValue::Number(5.0));
        assert_eq!(eval("let x = 5; x = 10; x"), JsValue::Number(10.0));
    }

    #[test]
    fn test_conditional() {
        assert_eq!(eval("true ? 1 : 2"), JsValue::Number(1.0));
        assert_eq!(eval("false ? 1 : 2"), JsValue::Number(2.0));
    }

    #[test]
    fn test_function() {
        assert_eq!(eval("function add(a, b) { return a + b; } add(2, 3)"), JsValue::Number(5.0));
    }

    #[test]
    fn test_this_binding() {
        // Test that 'this' is properly bound in method calls
        assert_eq!(eval("let obj = {x: 42, getX: function() { return this.x; }}; obj.getX()"), JsValue::Number(42.0));
    }

    #[test]
    fn test_function_call() {
        assert_eq!(eval("function greet() { return 'Hello ' + this.name; } greet.call({name: 'World'})"), JsValue::from("Hello World"));
        assert_eq!(eval("function add(a, b) { return a + b; } add.call(null, 2, 3)"), JsValue::Number(5.0));
    }

    #[test]
    fn test_function_apply() {
        assert_eq!(eval("function greet() { return 'Hello ' + this.name; } greet.apply({name: 'World'})"), JsValue::from("Hello World"));
        assert_eq!(eval("function add(a, b) { return a + b; } add.apply(null, [2, 3])"), JsValue::Number(5.0));
    }

    #[test]
    fn test_function_bind() {
        assert_eq!(eval("function greet() { return 'Hello ' + this.name; } const boundGreet = greet.bind({name: 'World'}); boundGreet()"), JsValue::from("Hello World"));
        assert_eq!(eval("function add(a, b) { return a + b; } const add5 = add.bind(null, 5); add5(3)"), JsValue::Number(8.0));
    }

    #[test]
    fn test_arrow_function() {
        assert_eq!(eval("const add = (a, b) => a + b; add(2, 3)"), JsValue::Number(5.0));
    }

    #[test]
    fn test_object() {
        assert_eq!(eval("const obj = { a: 1 }; obj.a"), JsValue::Number(1.0));
    }

    #[test]
    fn test_array() {
        assert_eq!(eval("const arr = [1, 2, 3]; arr[1]"), JsValue::Number(2.0));
    }

    // Array.prototype.push tests
    #[test]
    fn test_array_push_single() {
        assert_eq!(eval("const arr = [1, 2]; arr.push(3); arr.length"), JsValue::Number(3.0));
    }

    #[test]
    fn test_array_push_returns_length() {
        assert_eq!(eval("const arr = [1, 2]; arr.push(3)"), JsValue::Number(3.0));
    }

    #[test]
    fn test_array_push_multiple() {
        assert_eq!(eval("const arr = [1]; arr.push(2, 3, 4); arr.length"), JsValue::Number(4.0));
    }

    #[test]
    fn test_array_push_element_access() {
        assert_eq!(eval("const arr = [1, 2]; arr.push(3); arr[2]"), JsValue::Number(3.0));
    }

    // Array.prototype.pop tests
    #[test]
    fn test_array_pop_returns_last() {
        assert_eq!(eval("const arr = [1, 2, 3]; arr.pop()"), JsValue::Number(3.0));
    }

    #[test]
    fn test_array_pop_modifies_length() {
        assert_eq!(eval("const arr = [1, 2, 3]; arr.pop(); arr.length"), JsValue::Number(2.0));
    }

    #[test]
    fn test_array_pop_empty() {
        assert_eq!(eval("const arr = []; arr.pop()"), JsValue::Undefined);
    }

    // Array.prototype.map tests
    #[test]
    fn test_array_map_double() {
        // [1, 2, 3].map(x => x * 2) should equal [2, 4, 6]
        assert_eq!(eval("const arr = [1, 2, 3].map(x => x * 2); arr[0]"), JsValue::Number(2.0));
        assert_eq!(eval("const arr = [1, 2, 3].map(x => x * 2); arr[1]"), JsValue::Number(4.0));
        assert_eq!(eval("const arr = [1, 2, 3].map(x => x * 2); arr[2]"), JsValue::Number(6.0));
    }

    #[test]
    fn test_array_map_preserves_length() {
        assert_eq!(eval("[1, 2, 3].map(x => x * 2).length"), JsValue::Number(3.0));
    }

    #[test]
    fn test_array_map_with_index() {
        // map callback receives (element, index, array)
        assert_eq!(eval("[10, 20, 30].map((x, i) => i)[1]"), JsValue::Number(1.0));
    }

    #[test]
    fn test_array_map_to_strings() {
        assert_eq!(eval("[1, 2, 3].map(x => 'n' + x)[0]"), JsValue::String(JsString::from("n1")));
    }

    // Array.prototype.filter tests
    #[test]
    fn test_array_filter_evens() {
        assert_eq!(eval("[1, 2, 3, 4].filter(x => x % 2 === 0).length"), JsValue::Number(2.0));
    }

    #[test]
    fn test_array_filter_values() {
        assert_eq!(eval("[1, 2, 3, 4].filter(x => x % 2 === 0)[0]"), JsValue::Number(2.0));
        assert_eq!(eval("[1, 2, 3, 4].filter(x => x % 2 === 0)[1]"), JsValue::Number(4.0));
    }

    #[test]
    fn test_array_filter_none_match() {
        assert_eq!(eval("[1, 2, 3].filter(x => x > 10).length"), JsValue::Number(0.0));
    }

    #[test]
    fn test_array_filter_all_match() {
        assert_eq!(eval("[1, 2, 3].filter(x => x > 0).length"), JsValue::Number(3.0));
    }

    #[test]
    fn test_array_filter_with_index() {
        // Filter elements at even indices
        assert_eq!(eval("[10, 20, 30, 40].filter((x, i) => i % 2 === 0).length"), JsValue::Number(2.0));
    }

    // Chaining tests
    #[test]
    fn test_array_map_filter_chain() {
        // [1, 2, 3, 4].map(x => x * 2).filter(x => x > 4) should be [6, 8]
        assert_eq!(eval("[1, 2, 3, 4].map(x => x * 2).filter(x => x > 4).length"), JsValue::Number(2.0));
        assert_eq!(eval("[1, 2, 3, 4].map(x => x * 2).filter(x => x > 4)[0]"), JsValue::Number(6.0));
    }

    // Array.prototype.forEach tests
    #[test]
    fn test_array_foreach_side_effect() {
        assert_eq!(eval("let sum = 0; [1, 2, 3].forEach(x => sum += x); sum"), JsValue::Number(6.0));
    }

    #[test]
    fn test_array_foreach_returns_undefined() {
        assert_eq!(eval("[1, 2, 3].forEach(x => x * 2)"), JsValue::Undefined);
    }

    #[test]
    fn test_array_foreach_with_index() {
        assert_eq!(eval("let result = 0; [10, 20, 30].forEach((x, i) => result += i); result"), JsValue::Number(3.0));
    }

    // Array.prototype.reduce tests
    #[test]
    fn test_array_reduce_sum() {
        assert_eq!(eval("[1, 2, 3, 4].reduce((acc, x) => acc + x, 0)"), JsValue::Number(10.0));
    }

    #[test]
    fn test_array_reduce_no_initial() {
        // Without initial value, uses first element as initial
        assert_eq!(eval("[1, 2, 3, 4].reduce((acc, x) => acc + x)"), JsValue::Number(10.0));
    }

    #[test]
    fn test_array_reduce_multiply() {
        assert_eq!(eval("[1, 2, 3, 4].reduce((acc, x) => acc * x, 1)"), JsValue::Number(24.0));
    }

    #[test]
    fn test_array_reduce_with_index() {
        // Sum of indices
        assert_eq!(eval("[10, 20, 30].reduce((acc, x, i) => acc + i, 0)"), JsValue::Number(3.0));
    }

    #[test]
    fn test_array_reduce_to_object() {
        assert_eq!(eval("const obj = [['a', 1], ['b', 2]].reduce((acc, [k, v]) => { acc[k] = v; return acc; }, {}); obj.a"), JsValue::Number(1.0));
    }

    // Array.prototype.find tests
    #[test]
    fn test_array_find_found() {
        assert_eq!(eval("[1, 2, 3, 4].find(x => x > 2)"), JsValue::Number(3.0));
    }

    #[test]
    fn test_array_find_not_found() {
        assert_eq!(eval("[1, 2, 3].find(x => x > 10)"), JsValue::Undefined);
    }

    #[test]
    fn test_array_find_with_index() {
        assert_eq!(eval("[10, 20, 30].find((x, i) => i === 1)"), JsValue::Number(20.0));
    }

    // Array.prototype.findIndex tests
    #[test]
    fn test_array_findindex_found() {
        assert_eq!(eval("[1, 2, 3, 4].findIndex(x => x > 2)"), JsValue::Number(2.0));
    }

    #[test]
    fn test_array_findindex_not_found() {
        assert_eq!(eval("[1, 2, 3].findIndex(x => x > 10)"), JsValue::Number(-1.0));
    }

    #[test]
    fn test_array_findindex_first() {
        assert_eq!(eval("[5, 10, 15].findIndex(x => x >= 5)"), JsValue::Number(0.0));
    }

    // Array.prototype.indexOf tests
    #[test]
    fn test_array_indexof_found() {
        assert_eq!(eval("[1, 2, 3, 4].indexOf(3)"), JsValue::Number(2.0));
    }

    #[test]
    fn test_array_indexof_not_found() {
        assert_eq!(eval("[1, 2, 3].indexOf(5)"), JsValue::Number(-1.0));
    }

    #[test]
    fn test_array_indexof_first_occurrence() {
        assert_eq!(eval("[1, 2, 3, 2, 1].indexOf(2)"), JsValue::Number(1.0));
    }

    #[test]
    fn test_array_indexof_from_index() {
        assert_eq!(eval("[1, 2, 3, 2, 1].indexOf(2, 2)"), JsValue::Number(3.0));
    }

    // Array.prototype.includes tests
    #[test]
    fn test_array_includes_found() {
        assert_eq!(eval("[1, 2, 3].includes(2)"), JsValue::Boolean(true));
    }

    #[test]
    fn test_array_includes_not_found() {
        assert_eq!(eval("[1, 2, 3].includes(5)"), JsValue::Boolean(false));
    }

    #[test]
    fn test_array_includes_from_index() {
        assert_eq!(eval("[1, 2, 3].includes(1, 1)"), JsValue::Boolean(false));
    }

    // Array.prototype.slice tests
    #[test]
    fn test_array_slice_basic() {
        assert_eq!(eval("[1, 2, 3, 4, 5].slice(1, 4).length"), JsValue::Number(3.0));
        assert_eq!(eval("[1, 2, 3, 4, 5].slice(1, 4)[0]"), JsValue::Number(2.0));
    }

    #[test]
    fn test_array_slice_no_args() {
        assert_eq!(eval("[1, 2, 3].slice().length"), JsValue::Number(3.0));
    }

    #[test]
    fn test_array_slice_negative() {
        assert_eq!(eval("[1, 2, 3, 4, 5].slice(-2).length"), JsValue::Number(2.0));
        assert_eq!(eval("[1, 2, 3, 4, 5].slice(-2)[0]"), JsValue::Number(4.0));
    }

    #[test]
    fn test_array_slice_start_only() {
        assert_eq!(eval("[1, 2, 3, 4].slice(2).length"), JsValue::Number(2.0));
    }

    // Array.prototype.concat tests
    #[test]
    fn test_array_concat_arrays() {
        assert_eq!(eval("[1, 2].concat([3, 4]).length"), JsValue::Number(4.0));
        assert_eq!(eval("[1, 2].concat([3, 4])[2]"), JsValue::Number(3.0));
    }

    #[test]
    fn test_array_concat_values() {
        assert_eq!(eval("[1, 2].concat(3, 4).length"), JsValue::Number(4.0));
    }

    #[test]
    fn test_array_concat_mixed() {
        assert_eq!(eval("[1].concat([2, 3], 4, [5]).length"), JsValue::Number(5.0));
    }

    // Array.prototype.join tests
    #[test]
    fn test_array_join_default() {
        assert_eq!(eval("[1, 2, 3].join()"), JsValue::String(JsString::from("1,2,3")));
    }

    #[test]
    fn test_array_join_custom_separator() {
        assert_eq!(eval("[1, 2, 3].join('-')"), JsValue::String(JsString::from("1-2-3")));
    }

    #[test]
    fn test_array_join_empty() {
        assert_eq!(eval("[1, 2, 3].join('')"), JsValue::String(JsString::from("123")));
    }

    // Array.prototype.every tests
    #[test]
    fn test_array_every_all_pass() {
        assert_eq!(eval("[2, 4, 6].every(x => x % 2 === 0)"), JsValue::Boolean(true));
    }

    #[test]
    fn test_array_every_some_fail() {
        assert_eq!(eval("[2, 3, 6].every(x => x % 2 === 0)"), JsValue::Boolean(false));
    }

    #[test]
    fn test_array_every_empty() {
        assert_eq!(eval("[].every(x => false)"), JsValue::Boolean(true));
    }

    // Array.prototype.some tests
    #[test]
    fn test_array_some_one_passes() {
        assert_eq!(eval("[1, 2, 3].some(x => x > 2)"), JsValue::Boolean(true));
    }

    #[test]
    fn test_array_some_none_pass() {
        assert_eq!(eval("[1, 2, 3].some(x => x > 10)"), JsValue::Boolean(false));
    }

    #[test]
    fn test_array_some_empty() {
        assert_eq!(eval("[].some(x => true)"), JsValue::Boolean(false));
    }

    // String method tests
    #[test]
    fn test_string_charat() {
        assert_eq!(eval("'hello'.charAt(1)"), JsValue::String(JsString::from("e")));
    }

    #[test]
    fn test_string_indexof() {
        assert_eq!(eval("'hello world'.indexOf('world')"), JsValue::Number(6.0));
        assert_eq!(eval("'hello'.indexOf('x')"), JsValue::Number(-1.0));
    }

    #[test]
    fn test_string_includes() {
        assert_eq!(eval("'hello world'.includes('world')"), JsValue::Boolean(true));
        assert_eq!(eval("'hello'.includes('x')"), JsValue::Boolean(false));
    }

    #[test]
    fn test_string_startswith() {
        assert_eq!(eval("'hello world'.startsWith('hello')"), JsValue::Boolean(true));
        assert_eq!(eval("'hello world'.startsWith('world')"), JsValue::Boolean(false));
    }

    #[test]
    fn test_string_endswith() {
        assert_eq!(eval("'hello world'.endsWith('world')"), JsValue::Boolean(true));
        assert_eq!(eval("'hello world'.endsWith('hello')"), JsValue::Boolean(false));
    }

    #[test]
    fn test_string_slice() {
        assert_eq!(eval("'hello'.slice(1, 4)"), JsValue::String(JsString::from("ell")));
        assert_eq!(eval("'hello'.slice(-2)"), JsValue::String(JsString::from("lo")));
    }

    #[test]
    fn test_string_substring() {
        assert_eq!(eval("'hello'.substring(1, 4)"), JsValue::String(JsString::from("ell")));
    }

    #[test]
    fn test_string_tolowercase() {
        assert_eq!(eval("'HELLO'.toLowerCase()"), JsValue::String(JsString::from("hello")));
    }

    #[test]
    fn test_string_touppercase() {
        assert_eq!(eval("'hello'.toUpperCase()"), JsValue::String(JsString::from("HELLO")));
    }

    #[test]
    fn test_string_trim() {
        assert_eq!(eval("'  hello  '.trim()"), JsValue::String(JsString::from("hello")));
    }

    #[test]
    fn test_string_trimstart() {
        assert_eq!(eval("'  hello  '.trimStart()"), JsValue::String(JsString::from("hello  ")));
    }

    #[test]
    fn test_string_trimend() {
        assert_eq!(eval("'  hello  '.trimEnd()"), JsValue::String(JsString::from("  hello")));
    }

    #[test]
    fn test_string_split() {
        assert_eq!(eval("'a,b,c'.split(',').length"), JsValue::Number(3.0));
        assert_eq!(eval("'a,b,c'.split(',')[1]"), JsValue::String(JsString::from("b")));
    }

    #[test]
    fn test_string_repeat() {
        assert_eq!(eval("'ab'.repeat(3)"), JsValue::String(JsString::from("ababab")));
    }

    #[test]
    fn test_string_replace() {
        assert_eq!(eval("'hello world'.replace('world', 'rust')"), JsValue::String(JsString::from("hello rust")));
    }

    #[test]
    fn test_string_padstart() {
        assert_eq!(eval("'5'.padStart(3, '0')"), JsValue::String(JsString::from("005")));
    }

    #[test]
    fn test_string_padend() {
        assert_eq!(eval("'5'.padEnd(3, '0')"), JsValue::String(JsString::from("500")));
    }

    // Math tests
    #[test]
    fn test_math_abs() {
        assert_eq!(eval("Math.abs(-5)"), JsValue::Number(5.0));
        assert_eq!(eval("Math.abs(5)"), JsValue::Number(5.0));
    }

    #[test]
    fn test_math_floor_ceil_round() {
        assert_eq!(eval("Math.floor(4.7)"), JsValue::Number(4.0));
        assert_eq!(eval("Math.ceil(4.3)"), JsValue::Number(5.0));
        assert_eq!(eval("Math.round(4.5)"), JsValue::Number(5.0));
        assert_eq!(eval("Math.round(4.4)"), JsValue::Number(4.0));
    }

    #[test]
    fn test_math_trunc_sign() {
        assert_eq!(eval("Math.trunc(4.7)"), JsValue::Number(4.0));
        assert_eq!(eval("Math.trunc(-4.7)"), JsValue::Number(-4.0));
        assert_eq!(eval("Math.sign(-5)"), JsValue::Number(-1.0));
        assert_eq!(eval("Math.sign(5)"), JsValue::Number(1.0));
        assert_eq!(eval("Math.sign(0)"), JsValue::Number(0.0));
    }

    #[test]
    fn test_math_min_max() {
        assert_eq!(eval("Math.min(1, 2, 3)"), JsValue::Number(1.0));
        assert_eq!(eval("Math.max(1, 2, 3)"), JsValue::Number(3.0));
    }

    #[test]
    fn test_math_pow_sqrt() {
        assert_eq!(eval("Math.pow(2, 3)"), JsValue::Number(8.0));
        assert_eq!(eval("Math.sqrt(16)"), JsValue::Number(4.0));
    }

    #[test]
    fn test_math_log_exp() {
        assert_eq!(eval("Math.log(Math.E)"), JsValue::Number(1.0));
        assert_eq!(eval("Math.exp(0)"), JsValue::Number(1.0));
    }

    #[test]
    fn test_math_constants() {
        assert!(matches!(eval("Math.PI"), JsValue::Number(n) if (n - std::f64::consts::PI).abs() < 0.0001));
        assert!(matches!(eval("Math.E"), JsValue::Number(n) if (n - std::f64::consts::E).abs() < 0.0001));
    }

    #[test]
    fn test_math_random() {
        // Random should return a number between 0 and 1
        let result = eval("Math.random()");
        if let JsValue::Number(n) = result {
            assert!(n >= 0.0 && n < 1.0);
        } else {
            panic!("Math.random() should return a number");
        }
    }

    #[test]
    fn test_math_trig() {
        assert_eq!(eval("Math.sin(0)"), JsValue::Number(0.0));
        assert_eq!(eval("Math.cos(0)"), JsValue::Number(1.0));
    }

    #[test]
    fn test_math_cbrt() {
        assert_eq!(eval("Math.cbrt(27)"), JsValue::Number(3.0));
        assert_eq!(eval("Math.cbrt(8)"), JsValue::Number(2.0));
        assert_eq!(eval("Math.cbrt(-8)"), JsValue::Number(-2.0));
    }

    #[test]
    fn test_math_hypot() {
        assert_eq!(eval("Math.hypot(3, 4)"), JsValue::Number(5.0));
        assert_eq!(eval("Math.hypot(5, 12)"), JsValue::Number(13.0));
        assert_eq!(eval("Math.hypot()"), JsValue::Number(0.0));
    }

    #[test]
    fn test_math_log10_log2() {
        assert_eq!(eval("Math.log10(100)"), JsValue::Number(2.0));
        assert_eq!(eval("Math.log10(1000)"), JsValue::Number(3.0));
        assert_eq!(eval("Math.log2(8)"), JsValue::Number(3.0));
        assert_eq!(eval("Math.log2(16)"), JsValue::Number(4.0));
    }

    #[test]
    fn test_math_log1p_expm1() {
        // log1p(0) = 0
        assert_eq!(eval("Math.log1p(0)"), JsValue::Number(0.0));
        // expm1(0) = 0
        assert_eq!(eval("Math.expm1(0)"), JsValue::Number(0.0));
    }

    #[test]
    fn test_math_inverse_trig() {
        assert_eq!(eval("Math.asin(0)"), JsValue::Number(0.0));
        assert_eq!(eval("Math.acos(1)"), JsValue::Number(0.0));
        assert_eq!(eval("Math.atan(0)"), JsValue::Number(0.0));
        assert_eq!(eval("Math.atan2(0, 1)"), JsValue::Number(0.0));
    }

    #[test]
    fn test_math_hyperbolic() {
        assert_eq!(eval("Math.sinh(0)"), JsValue::Number(0.0));
        assert_eq!(eval("Math.cosh(0)"), JsValue::Number(1.0));
        assert_eq!(eval("Math.tanh(0)"), JsValue::Number(0.0));
        assert_eq!(eval("Math.asinh(0)"), JsValue::Number(0.0));
        assert_eq!(eval("Math.atanh(0)"), JsValue::Number(0.0));
    }

    // Global function tests
    #[test]
    fn test_parseint() {
        assert_eq!(eval("parseInt('42')"), JsValue::Number(42.0));
        assert_eq!(eval("parseInt('  42  ')"), JsValue::Number(42.0));
        assert_eq!(eval("parseInt('42.5')"), JsValue::Number(42.0));
        assert_eq!(eval("parseInt('ff', 16)"), JsValue::Number(255.0));
        assert_eq!(eval("parseInt('101', 2)"), JsValue::Number(5.0));
    }

    #[test]
    fn test_parsefloat() {
        assert_eq!(eval("parseFloat('3.14')"), JsValue::Number(3.14));
        assert_eq!(eval("parseFloat('  3.14  ')"), JsValue::Number(3.14));
        assert_eq!(eval("parseFloat('3.14abc')"), JsValue::Number(3.14));
    }

    #[test]
    fn test_isnan() {
        assert_eq!(eval("isNaN(NaN)"), JsValue::Boolean(true));
        assert_eq!(eval("isNaN(42)"), JsValue::Boolean(false));
        assert_eq!(eval("isNaN('hello')"), JsValue::Boolean(true));
        assert_eq!(eval("isNaN('42')"), JsValue::Boolean(false));
    }

    #[test]
    fn test_isfinite() {
        assert_eq!(eval("isFinite(42)"), JsValue::Boolean(true));
        assert_eq!(eval("isFinite(Infinity)"), JsValue::Boolean(false));
        assert_eq!(eval("isFinite(-Infinity)"), JsValue::Boolean(false));
        assert_eq!(eval("isFinite(NaN)"), JsValue::Boolean(false));
    }

    #[test]
    fn test_encodeuri() {
        assert_eq!(eval("encodeURI('hello world')"), JsValue::from("hello%20world"));
        assert_eq!(eval("encodeURI('a=1&b=2')"), JsValue::from("a=1&b=2"));
        assert_eq!(eval("encodeURI('http://example.com/path?q=hello world')"), JsValue::from("http://example.com/path?q=hello%20world"));
    }

    #[test]
    fn test_decodeuri() {
        assert_eq!(eval("decodeURI('hello%20world')"), JsValue::from("hello world"));
        assert_eq!(eval("decodeURI('a=1&b=2')"), JsValue::from("a=1&b=2"));
    }

    #[test]
    fn test_encodeuricomponent() {
        assert_eq!(eval("encodeURIComponent('hello world')"), JsValue::from("hello%20world"));
        assert_eq!(eval("encodeURIComponent('a=1&b=2')"), JsValue::from("a%3D1%26b%3D2"));
        assert_eq!(eval("encodeURIComponent('http://example.com')"), JsValue::from("http%3A%2F%2Fexample.com"));
    }

    #[test]
    fn test_decodeuricomponent() {
        assert_eq!(eval("decodeURIComponent('hello%20world')"), JsValue::from("hello world"));
        assert_eq!(eval("decodeURIComponent('a%3D1%26b%3D2')"), JsValue::from("a=1&b=2"));
    }

    #[test]
    fn test_number_isnan() {
        assert_eq!(eval("Number.isNaN(NaN)"), JsValue::Boolean(true));
        assert_eq!(eval("Number.isNaN(42)"), JsValue::Boolean(false));
        assert_eq!(eval("Number.isNaN('NaN')"), JsValue::Boolean(false)); // Different from global isNaN
    }

    #[test]
    fn test_number_isfinite() {
        assert_eq!(eval("Number.isFinite(42)"), JsValue::Boolean(true));
        assert_eq!(eval("Number.isFinite(Infinity)"), JsValue::Boolean(false));
        assert_eq!(eval("Number.isFinite('42')"), JsValue::Boolean(false)); // Different from global isFinite
    }

    #[test]
    fn test_number_isinteger() {
        assert_eq!(eval("Number.isInteger(42)"), JsValue::Boolean(true));
        assert_eq!(eval("Number.isInteger(42.0)"), JsValue::Boolean(true));
        assert_eq!(eval("Number.isInteger(42.5)"), JsValue::Boolean(false));
        assert_eq!(eval("Number.isInteger('42')"), JsValue::Boolean(false));
    }

    #[test]
    fn test_number_issafeinteger() {
        assert_eq!(eval("Number.isSafeInteger(42)"), JsValue::Boolean(true));
        assert_eq!(eval("Number.isSafeInteger(9007199254740991)"), JsValue::Boolean(true));
        assert_eq!(eval("Number.isSafeInteger(9007199254740992)"), JsValue::Boolean(false));
    }

    #[test]
    fn test_number_constants() {
        assert_eq!(eval("Number.POSITIVE_INFINITY"), JsValue::Number(f64::INFINITY));
        assert_eq!(eval("Number.NEGATIVE_INFINITY"), JsValue::Number(f64::NEG_INFINITY));
        assert_eq!(eval("Number.MAX_SAFE_INTEGER"), JsValue::Number(9007199254740991.0));
        assert_eq!(eval("Number.MIN_SAFE_INTEGER"), JsValue::Number(-9007199254740991.0));
    }

    #[test]
    fn test_number_tofixed() {
        assert_eq!(eval("(3.14159).toFixed(2)"), JsValue::String(JsString::from("3.14")));
        assert_eq!(eval("(3.14159).toFixed(0)"), JsValue::String(JsString::from("3")));
        assert_eq!(eval("(3.5).toFixed(0)"), JsValue::String(JsString::from("4")));
    }

    #[test]
    fn test_number_tostring() {
        assert_eq!(eval("(255).toString(16)"), JsValue::String(JsString::from("ff")));
        assert_eq!(eval("(10).toString(2)"), JsValue::String(JsString::from("1010")));
        assert_eq!(eval("(42).toString()"), JsValue::String(JsString::from("42")));
    }

    #[test]
    fn test_number_toprecision() {
        assert_eq!(eval("(123.456).toPrecision(4)"), JsValue::String(JsString::from("123.5")));
        assert_eq!(eval("(0.000123).toPrecision(2)"), JsValue::String(JsString::from("0.00012")));
        assert_eq!(eval("(1234.5).toPrecision(2)"), JsValue::String(JsString::from("1.2e+3")));
    }

    #[test]
    fn test_number_toexponential() {
        assert_eq!(eval("(123.456).toExponential(2)"), JsValue::String(JsString::from("1.23e+2")));
        assert_eq!(eval("(0.00123).toExponential(2)"), JsValue::String(JsString::from("1.23e-3")));
        assert_eq!(eval("(12345).toExponential(1)"), JsValue::String(JsString::from("1.2e+4")));
    }

    #[test]
    fn test_array_shift() {
        assert_eq!(eval("let a = [1, 2, 3]; a.shift()"), JsValue::Number(1.0));
        assert_eq!(eval("let a = [1, 2, 3]; a.shift(); a.length"), JsValue::Number(2.0));
        assert_eq!(eval("let a = [1, 2, 3]; a.shift(); a[0]"), JsValue::Number(2.0));
        assert_eq!(eval("let a = []; a.shift()"), JsValue::Undefined);
    }

    #[test]
    fn test_array_unshift() {
        assert_eq!(eval("let a = [1, 2, 3]; a.unshift(0)"), JsValue::Number(4.0));
        assert_eq!(eval("let a = [1, 2, 3]; a.unshift(0); a[0]"), JsValue::Number(0.0));
        assert_eq!(eval("let a = [1, 2, 3]; a.unshift(-1, 0); a.length"), JsValue::Number(5.0));
    }

    #[test]
    fn test_array_reverse() {
        assert_eq!(eval("let a = [1, 2, 3]; a.reverse(); a[0]"), JsValue::Number(3.0));
        assert_eq!(eval("let a = [1, 2, 3]; a.reverse(); a[2]"), JsValue::Number(1.0));
    }

    #[test]
    fn test_array_sort() {
        assert_eq!(eval("let a = [3, 1, 2]; a.sort(); a[0]"), JsValue::Number(1.0));
        assert_eq!(eval("let a = [3, 1, 2]; a.sort(); a[2]"), JsValue::Number(3.0));
        assert_eq!(eval("let a = ['c', 'a', 'b']; a.sort(); a[0]"), JsValue::String(JsString::from("a")));
        // Sort with comparator
        assert_eq!(eval("let a = [3, 1, 2]; a.sort((a, b) => b - a); a[0]"), JsValue::Number(3.0));
    }

    #[test]
    fn test_array_fill() {
        assert_eq!(eval("let a = [1, 2, 3]; a.fill(0); a[1]"), JsValue::Number(0.0));
        assert_eq!(eval("let a = [1, 2, 3]; a.fill(0, 1); a[0]"), JsValue::Number(1.0));
        assert_eq!(eval("let a = [1, 2, 3]; a.fill(0, 1); a[1]"), JsValue::Number(0.0));
        assert_eq!(eval("let a = [1, 2, 3]; a.fill(0, 1, 2); a[2]"), JsValue::Number(3.0));
    }

    #[test]
    fn test_array_copywithin() {
        assert_eq!(eval("let a = [1, 2, 3, 4, 5]; a.copyWithin(0, 3); a[0]"), JsValue::Number(4.0));
        assert_eq!(eval("let a = [1, 2, 3, 4, 5]; a.copyWithin(0, 3); a[1]"), JsValue::Number(5.0));
    }

    #[test]
    fn test_array_splice() {
        assert_eq!(eval("let a = [1, 2, 3]; let r = a.splice(1, 1); r[0]"), JsValue::Number(2.0));
        assert_eq!(eval("let a = [1, 2, 3]; a.splice(1, 1); a.length"), JsValue::Number(2.0));
        assert_eq!(eval("let a = [1, 2, 3]; a.splice(1, 1, 'a', 'b'); a.length"), JsValue::Number(4.0));
        assert_eq!(eval("let a = [1, 2, 3]; a.splice(1, 1, 'a', 'b'); a[1]"), JsValue::String(JsString::from("a")));
    }

    #[test]
    fn test_array_of() {
        assert_eq!(eval("Array.of(1, 2, 3).length"), JsValue::Number(3.0));
        assert_eq!(eval("Array.of(1, 2, 3)[0]"), JsValue::Number(1.0));
        assert_eq!(eval("Array.of(7).length"), JsValue::Number(1.0));
        assert_eq!(eval("Array.of().length"), JsValue::Number(0.0));
    }

    #[test]
    fn test_array_from() {
        assert_eq!(eval("Array.from([1, 2, 3]).length"), JsValue::Number(3.0));
        assert_eq!(eval("Array.from([1, 2, 3])[1]"), JsValue::Number(2.0));
        // With map function
        assert_eq!(eval("Array.from([1, 2, 3], x => x * 2)[1]"), JsValue::Number(4.0));
    }

    #[test]
    fn test_array_at() {
        assert_eq!(eval("[1, 2, 3].at(0)"), JsValue::Number(1.0));
        assert_eq!(eval("[1, 2, 3].at(2)"), JsValue::Number(3.0));
        assert_eq!(eval("[1, 2, 3].at(-1)"), JsValue::Number(3.0));
        assert_eq!(eval("[1, 2, 3].at(-2)"), JsValue::Number(2.0));
        assert_eq!(eval("[1, 2, 3].at(5)"), JsValue::Undefined);
    }

    #[test]
    fn test_array_lastindexof() {
        assert_eq!(eval("[1, 2, 3, 2, 1].lastIndexOf(2)"), JsValue::Number(3.0));
        assert_eq!(eval("[1, 2, 3].lastIndexOf(4)"), JsValue::Number(-1.0));
    }

    #[test]
    fn test_array_reduceright() {
        assert_eq!(eval("[1, 2, 3].reduceRight((acc, x) => acc + x, 0)"), JsValue::Number(6.0));
        assert_eq!(eval("['a', 'b', 'c'].reduceRight((acc, x) => acc + x, '')"), JsValue::String(JsString::from("cba")));
    }

    #[test]
    fn test_array_flat() {
        assert_eq!(eval("[[1, 2], [3, 4]].flat()[0]"), JsValue::Number(1.0));
        assert_eq!(eval("[[1, 2], [3, 4]].flat().length"), JsValue::Number(4.0));
        assert_eq!(eval("[1, [2, [3]]].flat(2).length"), JsValue::Number(3.0));
    }

    #[test]
    fn test_array_flatmap() {
        assert_eq!(eval("[1, 2, 3].flatMap(x => [x, x * 2]).length"), JsValue::Number(6.0));
        assert_eq!(eval("[1, 2, 3].flatMap(x => [x, x * 2])[1]"), JsValue::Number(2.0));
    }

    #[test]
    fn test_object_hasownproperty() {
        assert_eq!(eval("({a: 1}).hasOwnProperty('a')"), JsValue::Boolean(true));
        assert_eq!(eval("({a: 1}).hasOwnProperty('b')"), JsValue::Boolean(false));
        assert_eq!(eval("let o = {x: 1}; o.hasOwnProperty('x')"), JsValue::Boolean(true));
    }

    #[test]
    fn test_object_tostring() {
        assert_eq!(eval("({}).toString()"), JsValue::String(JsString::from("[object Object]")));
        assert_eq!(eval("[1,2,3].toString()"), JsValue::String(JsString::from("1,2,3")));
    }

    #[test]
    fn test_error_constructor() {
        assert_eq!(eval("new Error('oops').message"), JsValue::from("oops"));
        assert_eq!(eval("new Error('oops').name"), JsValue::from("Error"));
        assert_eq!(eval("new TypeError('bad type').name"), JsValue::from("TypeError"));
        assert_eq!(eval("new RangeError('out of range').name"), JsValue::from("RangeError"));
    }

    #[test]
    fn test_map() {
        // Basic Map creation and operations
        assert_eq!(eval("let m = new Map(); m.size"), JsValue::Number(0.0));
        assert_eq!(eval("let m = new Map(); m.set('a', 1); m.get('a')"), JsValue::Number(1.0));
        assert_eq!(eval("let m = new Map(); m.set('a', 1); m.has('a')"), JsValue::Boolean(true));
        assert_eq!(eval("let m = new Map(); m.has('a')"), JsValue::Boolean(false));
        assert_eq!(eval("let m = new Map(); m.set('a', 1); m.size"), JsValue::Number(1.0));

        // Delete and clear (use bracket notation for 'delete' since it's a reserved word)
        assert_eq!(eval("let m = new Map(); m.set('a', 1); m['delete']('a'); m.has('a')"), JsValue::Boolean(false));
        assert_eq!(eval("let m = new Map(); m.set('a', 1); m.set('b', 2); m.clear(); m.size"), JsValue::Number(0.0));

        // Object keys
        assert_eq!(eval("let m = new Map(); let obj = {}; m.set(obj, 'value'); m.get(obj)"), JsValue::from("value"));

        // Initialize with array of pairs
        assert_eq!(eval("let m = new Map([['a', 1], ['b', 2]]); m.get('b')"), JsValue::Number(2.0));

        // forEach
        assert_eq!(eval("let result = []; let m = new Map([['a', 1], ['b', 2]]); m.forEach((v, k) => result.push(k + ':' + v)); result.join(',')"), JsValue::from("a:1,b:2"));

        // Method chaining (set returns Map)
        assert_eq!(eval("let m = new Map(); m.set('a', 1).set('b', 2).get('b')"), JsValue::Number(2.0));
    }

    #[test]
    fn test_set() {
        // Basic Set creation and operations
        assert_eq!(eval("let s = new Set(); s.size"), JsValue::Number(0.0));
        assert_eq!(eval("let s = new Set(); s.add(1); s.has(1)"), JsValue::Boolean(true));
        assert_eq!(eval("let s = new Set(); s.has(1)"), JsValue::Boolean(false));
        assert_eq!(eval("let s = new Set(); s.add(1); s.size"), JsValue::Number(1.0));

        // Uniqueness - adding same value twice doesn't increase size
        assert_eq!(eval("let s = new Set(); s.add(1); s.add(1); s.size"), JsValue::Number(1.0));

        // Delete and clear (use bracket notation for 'delete' since it's a reserved word)
        assert_eq!(eval("let s = new Set(); s.add(1); s['delete'](1); s.has(1)"), JsValue::Boolean(false));
        assert_eq!(eval("let s = new Set(); s.add(1); s.add(2); s.clear(); s.size"), JsValue::Number(0.0));

        // Object values
        assert_eq!(eval("let s = new Set(); let obj = {}; s.add(obj); s.has(obj)"), JsValue::Boolean(true));

        // Initialize with array
        assert_eq!(eval("let s = new Set([1, 2, 3]); s.size"), JsValue::Number(3.0));
        assert_eq!(eval("let s = new Set([1, 2, 2, 3]); s.size"), JsValue::Number(3.0)); // Duplicates removed

        // forEach
        assert_eq!(eval("let result = []; let s = new Set([1, 2, 3]); s.forEach(v => result.push(v)); result.join(',')"), JsValue::from("1,2,3"));

        // Method chaining (add returns Set)
        assert_eq!(eval("let s = new Set(); s.add(1).add(2).has(2)"), JsValue::Boolean(true));
    }

    #[test]
    fn test_date() {
        // Date.now() returns a number (timestamp)
        let result = eval("Date.now()");
        assert!(matches!(result, JsValue::Number(_)));

        // new Date() with timestamp
        assert_eq!(eval("new Date(0).getTime()"), JsValue::Number(0.0));
        assert_eq!(eval("new Date(1000).getTime()"), JsValue::Number(1000.0));

        // Date methods
        assert_eq!(eval("new Date(0).getFullYear()"), JsValue::Number(1970.0));
        assert_eq!(eval("new Date(0).getMonth()"), JsValue::Number(0.0)); // January = 0
        assert_eq!(eval("new Date(0).getDate()"), JsValue::Number(1.0));

        // Date.UTC
        assert_eq!(eval("Date.UTC(1970, 0, 1)"), JsValue::Number(0.0));

        // toISOString
        assert_eq!(eval("new Date(0).toISOString()"), JsValue::from("1970-01-01T00:00:00.000Z"));
    }

    #[test]
    fn test_regexp() {
        // Basic RegExp creation and test
        assert_eq!(eval("new RegExp('abc').test('abc')"), JsValue::Boolean(true));
        assert_eq!(eval("new RegExp('abc').test('def')"), JsValue::Boolean(false));
        assert_eq!(eval("new RegExp('a.c').test('abc')"), JsValue::Boolean(true));
        assert_eq!(eval("new RegExp('a.c').test('adc')"), JsValue::Boolean(true));

        // Case insensitive flag
        assert_eq!(eval("new RegExp('abc', 'i').test('ABC')"), JsValue::Boolean(true));

        // Source and flags properties
        assert_eq!(eval("new RegExp('abc', 'gi').source"), JsValue::from("abc"));
        assert_eq!(eval("new RegExp('abc', 'gi').flags"), JsValue::from("gi"));

        // exec method
        assert_eq!(eval("new RegExp('a(b)c').exec('abc')[0]"), JsValue::from("abc"));
        assert_eq!(eval("new RegExp('a(b)c').exec('abc')[1]"), JsValue::from("b"));
        assert_eq!(eval("new RegExp('xyz').exec('abc')"), JsValue::Null);
    }

    #[test]
    fn test_string_concat() {
        assert_eq!(eval("'hello'.concat(' ', 'world')"), JsValue::String(JsString::from("hello world")));
    }

    #[test]
    fn test_string_charat_index() {
        assert_eq!(eval("'hello'.charCodeAt(0)"), JsValue::Number(104.0));
        assert_eq!(eval("'hello'.charCodeAt(1)"), JsValue::Number(101.0));
    }

    #[test]
    fn test_string_fromcharcode() {
        assert_eq!(eval("String.fromCharCode(104, 105)"), JsValue::String(JsString::from("hi")));
    }

    #[test]
    fn test_string_lastindexof() {
        assert_eq!(eval("'hello world'.lastIndexOf('o')"), JsValue::Number(7.0));
        assert_eq!(eval("'hello world'.lastIndexOf('l')"), JsValue::Number(9.0));
        assert_eq!(eval("'hello world'.lastIndexOf('x')"), JsValue::Number(-1.0));
        assert_eq!(eval("'hello world'.lastIndexOf('o', 5)"), JsValue::Number(4.0));
        assert_eq!(eval("'hello'.lastIndexOf('')"), JsValue::Number(5.0));
    }

    #[test]
    fn test_string_at() {
        assert_eq!(eval("'hello'.at(0)"), JsValue::String(JsString::from("h")));
        assert_eq!(eval("'hello'.at(1)"), JsValue::String(JsString::from("e")));
        assert_eq!(eval("'hello'.at(-1)"), JsValue::String(JsString::from("o")));
        assert_eq!(eval("'hello'.at(-2)"), JsValue::String(JsString::from("l")));
        assert_eq!(eval("'hello'.at(10)"), JsValue::Undefined);
        assert_eq!(eval("'hello'.at(-10)"), JsValue::Undefined);
    }

    #[test]
    fn test_string_replaceall() {
        assert_eq!(eval("'aabbcc'.replaceAll('b', 'x')"), JsValue::String(JsString::from("aaxxcc")));
        assert_eq!(eval("'hello world'.replaceAll('o', '0')"), JsValue::String(JsString::from("hell0 w0rld")));
        assert_eq!(eval("'aaa'.replaceAll('a', 'bb')"), JsValue::String(JsString::from("bbbbbb")));
        assert_eq!(eval("'hello'.replaceAll('x', 'y')"), JsValue::String(JsString::from("hello")));
        assert_eq!(eval("''.replaceAll('a', 'b')"), JsValue::String(JsString::from("")));
    }

    #[test]
    fn test_console_methods() {
        // All console methods return undefined
        assert_eq!(eval("console.log('test')"), JsValue::Undefined);
        assert_eq!(eval("console.error('test')"), JsValue::Undefined);
        assert_eq!(eval("console.warn('test')"), JsValue::Undefined);
        assert_eq!(eval("console.info('test')"), JsValue::Undefined);
        assert_eq!(eval("console.debug('test')"), JsValue::Undefined);
    }

    #[test]
    fn test_array_findlast() {
        assert_eq!(eval("[1, 2, 3, 2].findLast(x => x === 2)"), JsValue::Number(2.0));
        assert_eq!(eval("[1, 2, 3].findLast(x => x > 1)"), JsValue::Number(3.0));
        assert_eq!(eval("[1, 2, 3].findLast(x => x > 10)"), JsValue::Undefined);
    }

    #[test]
    fn test_array_findlastindex() {
        assert_eq!(eval("[1, 2, 3, 2].findLastIndex(x => x === 2)"), JsValue::Number(3.0));
        assert_eq!(eval("[1, 2, 3].findLastIndex(x => x > 1)"), JsValue::Number(2.0));
        assert_eq!(eval("[1, 2, 3].findLastIndex(x => x > 10)"), JsValue::Number(-1.0));
    }

    #[test]
    fn test_array_toreversed() {
        assert_eq!(eval("let a = [1, 2, 3]; let b = a.toReversed(); b[0]"), JsValue::Number(3.0));
        assert_eq!(eval("let a = [1, 2, 3]; let b = a.toReversed(); a[0]"), JsValue::Number(1.0)); // Original unchanged
    }

    #[test]
    fn test_array_tosorted() {
        assert_eq!(eval("let a = [3, 1, 2]; let b = a.toSorted(); b[0]"), JsValue::Number(1.0));
        assert_eq!(eval("let a = [3, 1, 2]; let b = a.toSorted(); a[0]"), JsValue::Number(3.0)); // Original unchanged
    }

    #[test]
    fn test_array_tospliced() {
        assert_eq!(eval("[1, 2, 3].toSpliced(1, 1, 'a', 'b')[1]"), JsValue::String(JsString::from("a")));
        assert_eq!(eval("[1, 2, 3].toSpliced(1, 1, 'a', 'b').length"), JsValue::Number(4.0));
    }

    #[test]
    fn test_array_with() {
        assert_eq!(eval("[1, 2, 3].with(1, 'x')[1]"), JsValue::String(JsString::from("x")));
        assert_eq!(eval("let a = [1, 2, 3]; let b = a.with(1, 'x'); a[1]"), JsValue::Number(2.0)); // Original unchanged
    }

    #[test]
    fn test_object_fromentries() {
        assert_eq!(eval("Object.fromEntries([['a', 1], ['b', 2]]).a"), JsValue::Number(1.0));
        assert_eq!(eval("Object.fromEntries([['a', 1], ['b', 2]]).b"), JsValue::Number(2.0));
    }

    #[test]
    fn test_object_hasown() {
        assert_eq!(eval("Object.hasOwn({a: 1}, 'a')"), JsValue::Boolean(true));
        assert_eq!(eval("Object.hasOwn({a: 1}, 'b')"), JsValue::Boolean(false));
    }

    #[test]
    fn test_object_create() {
        assert_eq!(eval("Object.create(null).hasOwnProperty"), JsValue::Undefined);
        assert_eq!(eval("let proto = {x: 1}; let o = Object.create(proto); o.x"), JsValue::Number(1.0));
    }

    #[test]
    fn test_object_freeze() {
        assert_eq!(eval("let o = {a: 1}; Object.freeze(o); o.a = 2; o.a"), JsValue::Number(1.0));
        assert_eq!(eval("Object.isFrozen(Object.freeze({a: 1}))"), JsValue::Boolean(true));
    }

    #[test]
    fn test_object_seal() {
        assert_eq!(eval("Object.isSealed(Object.seal({a: 1}))"), JsValue::Boolean(true));
    }
}
