//! Interpreter for executing TypeScript AST

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
    /// Array.prototype for all array instances
    pub array_prototype: JsObjectRef,
    /// String.prototype for string methods
    pub string_prototype: JsObjectRef,
    /// Number.prototype for number methods
    pub number_prototype: JsObjectRef,
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

        // Add console.log as a basic output
        let console = create_object();
        {
            let log_fn = create_function(JsFunction::Native(NativeFunction {
                name: "log".to_string(),
                func: console_log,
                arity: 0,
            }));
            console.borrow_mut().set_property(PropertyKey::from("log"), JsValue::Object(log_fn));
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
        }

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

        Self { global, env, array_prototype, string_prototype, number_prototype: number_proto }
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
                // Simplified - would need proper this binding
                Ok(JsValue::Undefined)
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
                Ok(obj.borrow().get_property(&key).unwrap_or(JsValue::Undefined))
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
        }
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

// Native function implementations

fn console_log(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let output: Vec<String> = args.iter().map(|v| format!("{:?}", v)).collect();
    println!("{}", output.join(" "));
    Ok(JsValue::Undefined)
}

fn json_stringify(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let value = args.first().cloned().unwrap_or(JsValue::Undefined);
    let json = js_value_to_json(&value)?;
    Ok(JsValue::String(JsString::from(json.to_string())))
}

fn json_parse(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let text = args.first().cloned().unwrap_or(JsValue::Undefined);
    let text_str = text.to_js_string();

    let json: serde_json::Value = serde_json::from_str(text_str.as_str())
        .map_err(|e| JsError::syntax_error(format!("JSON parse error: {}", e), 0, 0))?;

    json_to_js_value(&json)
}

fn object_constructor(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let value = args.first().cloned().unwrap_or(JsValue::Undefined);
    match value {
        JsValue::Null | JsValue::Undefined => Ok(JsValue::Object(create_object())),
        JsValue::Object(_) => Ok(value),
        _ => Ok(JsValue::Object(create_object())),
    }
}

fn object_keys(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);
    let JsValue::Object(obj_ref) = obj else {
        return Err(JsError::type_error("Object.keys requires an object"));
    };

    let keys: Vec<JsValue> = obj_ref
        .borrow()
        .properties
        .iter()
        .filter(|(_, prop)| prop.enumerable)
        .map(|(key, _)| JsValue::String(JsString::from(key.to_string())))
        .collect();

    Ok(JsValue::Object(create_array(keys)))
}

fn object_values(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);
    let JsValue::Object(obj_ref) = obj else {
        return Err(JsError::type_error("Object.values requires an object"));
    };

    let values: Vec<JsValue> = obj_ref
        .borrow()
        .properties
        .iter()
        .filter(|(_, prop)| prop.enumerable)
        .map(|(_, prop)| prop.value.clone())
        .collect();

    Ok(JsValue::Object(create_array(values)))
}

fn object_entries(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let obj = args.first().cloned().unwrap_or(JsValue::Undefined);
    let JsValue::Object(obj_ref) = obj else {
        return Err(JsError::type_error("Object.entries requires an object"));
    };

    let entries: Vec<JsValue> = obj_ref
        .borrow()
        .properties
        .iter()
        .filter(|(_, prop)| prop.enumerable)
        .map(|(key, prop)| {
            JsValue::Object(create_array(vec![
                JsValue::String(JsString::from(key.to_string())),
                prop.value.clone(),
            ]))
        })
        .collect();

    Ok(JsValue::Object(create_array(entries)))
}

fn object_assign(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let target = args.first().cloned().unwrap_or(JsValue::Undefined);
    let JsValue::Object(target_ref) = target.clone() else {
        return Err(JsError::type_error("Object.assign requires an object target"));
    };

    for source in args.iter().skip(1) {
        if let JsValue::Object(src_ref) = source {
            let src = src_ref.borrow();
            for (key, prop) in src.properties.iter() {
                if prop.enumerable {
                    target_ref.borrow_mut().set_property(key.clone(), prop.value.clone());
                }
            }
        }
    }

    Ok(target)
}

fn array_constructor_fn(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    if args.len() == 1 {
        if let JsValue::Number(n) = &args[0] {
            let len = *n as u32;
            let mut elements = Vec::with_capacity(len as usize);
            for _ in 0..len {
                elements.push(JsValue::Undefined);
            }
            return Ok(JsValue::Object(create_array(elements)));
        }
    }
    Ok(JsValue::Object(create_array(args)))
}

fn array_is_array(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let value = args.first().cloned().unwrap_or(JsValue::Undefined);
    let is_array = match value {
        JsValue::Object(obj) => matches!(obj.borrow().exotic, ExoticObject::Array { .. }),
        _ => false,
    };
    Ok(JsValue::Boolean(is_array))
}

fn array_push(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error("Array.prototype.push called on non-object"));
    };

    let mut arr_ref = arr.borrow_mut();

    // Get current length
    let mut current_length = match &arr_ref.exotic {
        ExoticObject::Array { length } => *length,
        _ => return Err(JsError::type_error("Array.prototype.push called on non-array")),
    };

    // Add each argument
    for arg in args {
        arr_ref.properties.insert(
            PropertyKey::Index(current_length),
            crate::value::Property::data(arg),
        );
        current_length += 1;
    }

    // Update the exotic length
    if let ExoticObject::Array { ref mut length } = arr_ref.exotic {
        *length = current_length;
    }

    // Update length property
    arr_ref.properties.insert(
        PropertyKey::from("length"),
        crate::value::Property {
            value: JsValue::Number(current_length as f64),
            writable: true,
            enumerable: false,
            configurable: false,
        },
    );

    Ok(JsValue::Number(current_length as f64))
}

fn array_pop(_interp: &mut Interpreter, this: JsValue, _args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error("Array.prototype.pop called on non-object"));
    };

    let mut arr_ref = arr.borrow_mut();

    // Get current length
    let current_length = match &arr_ref.exotic {
        ExoticObject::Array { length } => *length,
        _ => return Err(JsError::type_error("Array.prototype.pop called on non-array")),
    };

    if current_length == 0 {
        return Ok(JsValue::Undefined);
    }

    let new_length = current_length - 1;

    // Get and remove the last element
    let value = arr_ref
        .properties
        .remove(&PropertyKey::Index(new_length))
        .map(|p| p.value)
        .unwrap_or(JsValue::Undefined);

    // Update the exotic length
    if let ExoticObject::Array { ref mut length } = arr_ref.exotic {
        *length = new_length;
    }

    // Update length property
    arr_ref.properties.insert(
        PropertyKey::from("length"),
        crate::value::Property {
            value: JsValue::Number(new_length as f64),
            writable: true,
            enumerable: false,
            configurable: false,
        },
    );

    Ok(value)
}

fn array_map(interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error("Array.prototype.map called on non-object"));
    };

    let callback = args.first().cloned().unwrap_or(JsValue::Undefined);
    if !callback.is_callable() {
        return Err(JsError::type_error("Array.prototype.map callback is not a function"));
    }

    let this_arg = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    // Get array length
    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

    // Map elements
    let mut result = Vec::with_capacity(length as usize);
    for i in 0..length {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

        // Call callback(element, index, array)
        let mapped = interp.call_function(
            callback.clone(),
            this_arg.clone(),
            vec![elem, JsValue::Number(i as f64), this.clone()],
        )?;

        result.push(mapped);
    }

    Ok(JsValue::Object(interp.create_array(result)))
}

fn array_filter(interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error("Array.prototype.filter called on non-object"));
    };

    let callback = args.first().cloned().unwrap_or(JsValue::Undefined);
    if !callback.is_callable() {
        return Err(JsError::type_error("Array.prototype.filter callback is not a function"));
    }

    let this_arg = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    // Get array length
    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

    // Filter elements
    let mut result = Vec::new();
    for i in 0..length {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

        // Call callback(element, index, array)
        let keep = interp.call_function(
            callback.clone(),
            this_arg.clone(),
            vec![elem.clone(), JsValue::Number(i as f64), this.clone()],
        )?;

        if keep.to_boolean() {
            result.push(elem);
        }
    }

    Ok(JsValue::Object(interp.create_array(result)))
}

fn array_foreach(interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error("Array.prototype.forEach called on non-object"));
    };

    let callback = args.first().cloned().unwrap_or(JsValue::Undefined);
    if !callback.is_callable() {
        return Err(JsError::type_error("Array.prototype.forEach callback is not a function"));
    }

    let this_arg = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    // Get array length
    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

    // Call callback for each element
    for i in 0..length {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

        // Call callback(element, index, array)
        interp.call_function(
            callback.clone(),
            this_arg.clone(),
            vec![elem, JsValue::Number(i as f64), this.clone()],
        )?;
    }

    Ok(JsValue::Undefined)
}

fn array_reduce(interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error("Array.prototype.reduce called on non-object"));
    };

    let callback = args.first().cloned().unwrap_or(JsValue::Undefined);
    if !callback.is_callable() {
        return Err(JsError::type_error("Array.prototype.reduce callback is not a function"));
    }

    // Get array length
    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

    // Determine initial value and starting index
    let (mut accumulator, start_index) = if args.len() >= 2 {
        (args[1].clone(), 0)
    } else {
        if length == 0 {
            return Err(JsError::type_error("Reduce of empty array with no initial value"));
        }
        let first = arr
            .borrow()
            .get_property(&PropertyKey::Index(0))
            .unwrap_or(JsValue::Undefined);
        (first, 1)
    };

    // Reduce
    for i in start_index..length {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

        // Call callback(accumulator, element, index, array)
        accumulator = interp.call_function(
            callback.clone(),
            JsValue::Undefined,
            vec![accumulator, elem, JsValue::Number(i as f64), this.clone()],
        )?;
    }

    Ok(accumulator)
}

fn array_find(interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error("Array.prototype.find called on non-object"));
    };

    let callback = args.first().cloned().unwrap_or(JsValue::Undefined);
    if !callback.is_callable() {
        return Err(JsError::type_error("Array.prototype.find callback is not a function"));
    }

    let this_arg = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

    for i in 0..length {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

        let result = interp.call_function(
            callback.clone(),
            this_arg.clone(),
            vec![elem.clone(), JsValue::Number(i as f64), this.clone()],
        )?;

        if result.to_boolean() {
            return Ok(elem);
        }
    }

    Ok(JsValue::Undefined)
}

fn array_find_index(interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error("Array.prototype.findIndex called on non-object"));
    };

    let callback = args.first().cloned().unwrap_or(JsValue::Undefined);
    if !callback.is_callable() {
        return Err(JsError::type_error("Array.prototype.findIndex callback is not a function"));
    }

    let this_arg = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

    for i in 0..length {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

        let result = interp.call_function(
            callback.clone(),
            this_arg.clone(),
            vec![elem, JsValue::Number(i as f64), this.clone()],
        )?;

        if result.to_boolean() {
            return Ok(JsValue::Number(i as f64));
        }
    }

    Ok(JsValue::Number(-1.0))
}

fn array_index_of(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error("Array.prototype.indexOf called on non-object"));
    };

    let search_element = args.first().cloned().unwrap_or(JsValue::Undefined);
    let from_index = args
        .get(1)
        .map(|v| v.to_number() as i64)
        .unwrap_or(0);

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length as i64,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

    let start = if from_index < 0 {
        (length + from_index).max(0) as u32
    } else {
        from_index.min(length) as u32
    };

    for i in start..(length as u32) {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

        if elem.strict_equals(&search_element) {
            return Ok(JsValue::Number(i as f64));
        }
    }

    Ok(JsValue::Number(-1.0))
}

fn array_includes(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error("Array.prototype.includes called on non-object"));
    };

    let search_element = args.first().cloned().unwrap_or(JsValue::Undefined);
    let from_index = args
        .get(1)
        .map(|v| v.to_number() as i64)
        .unwrap_or(0);

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length as i64,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

    let start = if from_index < 0 {
        (length + from_index).max(0) as u32
    } else {
        from_index.min(length) as u32
    };

    for i in start..(length as u32) {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

        // includes uses SameValueZero which treats NaN as equal to NaN
        let found = match (&elem, &search_element) {
            (JsValue::Number(a), JsValue::Number(b)) if a.is_nan() && b.is_nan() => true,
            _ => elem.strict_equals(&search_element),
        };

        if found {
            return Ok(JsValue::Boolean(true));
        }
    }

    Ok(JsValue::Boolean(false))
}

fn array_slice(interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error("Array.prototype.slice called on non-object"));
    };

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length as i64,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

    let start_arg = args.first().map(|v| v.to_number() as i64).unwrap_or(0);
    let end_arg = args.get(1).map(|v| v.to_number() as i64).unwrap_or(length);

    let start = if start_arg < 0 {
        (length + start_arg).max(0)
    } else {
        start_arg.min(length)
    };

    let end = if end_arg < 0 {
        (length + end_arg).max(0)
    } else {
        end_arg.min(length)
    };

    let mut result = Vec::new();
    for i in start..end {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i as u32))
            .unwrap_or(JsValue::Undefined);
        result.push(elem);
    }

    Ok(JsValue::Object(interp.create_array(result)))
}

fn array_concat(interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let mut result = Vec::new();

    // Helper to add elements from an array or a single value
    fn add_elements(result: &mut Vec<JsValue>, value: JsValue) {
        match &value {
            JsValue::Object(obj) => {
                let obj_ref = obj.borrow();
                if let ExoticObject::Array { length } = &obj_ref.exotic {
                    for i in 0..*length {
                        let elem = obj_ref
                            .get_property(&PropertyKey::Index(i))
                            .unwrap_or(JsValue::Undefined);
                        result.push(elem);
                    }
                } else {
                    result.push(value.clone());
                }
            }
            _ => result.push(value),
        }
    }

    // Add elements from this array
    add_elements(&mut result, this);

    // Add elements from each argument
    for arg in args {
        add_elements(&mut result, arg);
    }

    Ok(JsValue::Object(interp.create_array(result)))
}

fn array_join(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this else {
        return Err(JsError::type_error("Array.prototype.join called on non-object"));
    };

    let separator = args
        .first()
        .map(|v| {
            if matches!(v, JsValue::Undefined) {
                ",".to_string()
            } else {
                v.to_js_string().to_string()
            }
        })
        .unwrap_or_else(|| ",".to_string());

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

    let mut parts = Vec::with_capacity(length as usize);
    for i in 0..length {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

        let part = match elem {
            JsValue::Undefined | JsValue::Null => String::new(),
            _ => elem.to_js_string().to_string(),
        };
        parts.push(part);
    }

    Ok(JsValue::String(JsString::from(parts.join(&separator))))
}

fn array_every(interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error("Array.prototype.every called on non-object"));
    };

    let callback = args.first().cloned().unwrap_or(JsValue::Undefined);
    if !callback.is_callable() {
        return Err(JsError::type_error("Array.prototype.every callback is not a function"));
    }

    let this_arg = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

    for i in 0..length {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

        let result = interp.call_function(
            callback.clone(),
            this_arg.clone(),
            vec![elem, JsValue::Number(i as f64), this.clone()],
        )?;

        if !result.to_boolean() {
            return Ok(JsValue::Boolean(false));
        }
    }

    Ok(JsValue::Boolean(true))
}

fn array_some(interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let JsValue::Object(arr) = this.clone() else {
        return Err(JsError::type_error("Array.prototype.some called on non-object"));
    };

    let callback = args.first().cloned().unwrap_or(JsValue::Undefined);
    if !callback.is_callable() {
        return Err(JsError::type_error("Array.prototype.some callback is not a function"));
    }

    let this_arg = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    let length = {
        let arr_ref = arr.borrow();
        match &arr_ref.exotic {
            ExoticObject::Array { length } => *length,
            _ => return Err(JsError::type_error("Not an array")),
        }
    };

    for i in 0..length {
        let elem = arr
            .borrow()
            .get_property(&PropertyKey::Index(i))
            .unwrap_or(JsValue::Undefined);

        let result = interp.call_function(
            callback.clone(),
            this_arg.clone(),
            vec![elem, JsValue::Number(i as f64), this.clone()],
        )?;

        if result.to_boolean() {
            return Ok(JsValue::Boolean(true));
        }
    }

    Ok(JsValue::Boolean(false))
}

// String methods

fn string_char_at(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let index = args.first().map(|v| v.to_number() as usize).unwrap_or(0);

    if let Some(ch) = s.as_str().chars().nth(index) {
        Ok(JsValue::String(JsString::from(ch.to_string())))
    } else {
        Ok(JsValue::String(JsString::from("")))
    }
}

fn string_index_of(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let search = args.first().map(|v| v.to_js_string().to_string()).unwrap_or_default();
    let from_index = args.get(1).map(|v| v.to_number() as usize).unwrap_or(0);

    if from_index >= s.len() {
        return Ok(JsValue::Number(-1.0));
    }

    match s.as_str()[from_index..].find(&search) {
        Some(pos) => Ok(JsValue::Number((from_index + pos) as f64)),
        None => Ok(JsValue::Number(-1.0)),
    }
}

fn string_includes(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let search = args.first().map(|v| v.to_js_string().to_string()).unwrap_or_default();
    let from_index = args.get(1).map(|v| v.to_number() as usize).unwrap_or(0);

    if from_index >= s.len() {
        return Ok(JsValue::Boolean(search.is_empty()));
    }

    Ok(JsValue::Boolean(s.as_str()[from_index..].contains(&search)))
}

fn string_starts_with(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let search = args.first().map(|v| v.to_js_string().to_string()).unwrap_or_default();
    let position = args.get(1).map(|v| v.to_number() as usize).unwrap_or(0);

    if position >= s.len() {
        return Ok(JsValue::Boolean(search.is_empty()));
    }

    Ok(JsValue::Boolean(s.as_str()[position..].starts_with(&search)))
}

fn string_ends_with(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let search = args.first().map(|v| v.to_js_string().to_string()).unwrap_or_default();
    let end_position = args.get(1).map(|v| v.to_number() as usize).unwrap_or(s.len());

    let end = end_position.min(s.len());
    Ok(JsValue::Boolean(s.as_str()[..end].ends_with(&search)))
}

fn string_slice(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let len = s.len() as i64;

    let start_arg = args.first().map(|v| v.to_number() as i64).unwrap_or(0);
    let end_arg = args.get(1).map(|v| v.to_number() as i64).unwrap_or(len);

    let start = if start_arg < 0 { (len + start_arg).max(0) } else { start_arg.min(len) } as usize;
    let end = if end_arg < 0 { (len + end_arg).max(0) } else { end_arg.min(len) } as usize;

    if start >= end {
        return Ok(JsValue::String(JsString::from("")));
    }

    // Need to handle UTF-8 properly - slice by characters, not bytes
    let chars: Vec<char> = s.as_str().chars().collect();
    let result: String = chars[start.min(chars.len())..end.min(chars.len())].iter().collect();
    Ok(JsValue::String(JsString::from(result)))
}

fn string_substring(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let len = s.len();

    let start = args.first().map(|v| {
        let n = v.to_number();
        if n.is_nan() { 0 } else { (n as usize).min(len) }
    }).unwrap_or(0);

    let end = args.get(1).map(|v| {
        let n = v.to_number();
        if n.is_nan() { 0 } else { (n as usize).min(len) }
    }).unwrap_or(len);

    let (start, end) = if start > end { (end, start) } else { (start, end) };

    let chars: Vec<char> = s.as_str().chars().collect();
    let result: String = chars[start.min(chars.len())..end.min(chars.len())].iter().collect();
    Ok(JsValue::String(JsString::from(result)))
}

fn string_to_lower_case(_interp: &mut Interpreter, this: JsValue, _args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    Ok(JsValue::String(JsString::from(s.as_str().to_lowercase())))
}

fn string_to_upper_case(_interp: &mut Interpreter, this: JsValue, _args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    Ok(JsValue::String(JsString::from(s.as_str().to_uppercase())))
}

fn string_trim(_interp: &mut Interpreter, this: JsValue, _args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    Ok(JsValue::String(JsString::from(s.as_str().trim())))
}

fn string_trim_start(_interp: &mut Interpreter, this: JsValue, _args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    Ok(JsValue::String(JsString::from(s.as_str().trim_start())))
}

fn string_trim_end(_interp: &mut Interpreter, this: JsValue, _args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    Ok(JsValue::String(JsString::from(s.as_str().trim_end())))
}

fn string_split(interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let separator = args.first().map(|v| v.to_js_string().to_string());
    let limit = args.get(1).map(|v| v.to_number() as usize);

    let parts: Vec<JsValue> = match separator {
        Some(sep) if !sep.is_empty() => {
            let split: Vec<&str> = match limit {
                Some(l) => s.as_str().splitn(l, &sep).collect(),
                None => s.as_str().split(&sep).collect(),
            };
            split.into_iter().map(|p| JsValue::String(JsString::from(p))).collect()
        }
        Some(_) => {
            // Empty separator - split into characters
            let chars: Vec<JsValue> = s.as_str().chars()
                .map(|c| JsValue::String(JsString::from(c.to_string())))
                .collect();
            match limit {
                Some(l) => chars.into_iter().take(l).collect(),
                None => chars,
            }
        }
        None => vec![JsValue::String(JsString::from(s.to_string()))],
    };

    Ok(JsValue::Object(interp.create_array(parts)))
}

fn string_repeat(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let count = args.first().map(|v| v.to_number() as usize).unwrap_or(0);
    Ok(JsValue::String(JsString::from(s.as_str().repeat(count))))
}

fn string_replace(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let search = args.first().map(|v| v.to_js_string().to_string()).unwrap_or_default();
    let replacement = args.get(1).map(|v| v.to_js_string().to_string()).unwrap_or_default();

    // Only replace first occurrence (like JS)
    Ok(JsValue::String(JsString::from(s.as_str().replacen(&search, &replacement, 1))))
}

fn string_pad_start(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let target_length = args.first().map(|v| v.to_number() as usize).unwrap_or(0);
    let pad_string = args.get(1).map(|v| v.to_js_string().to_string()).unwrap_or_else(|| " ".to_string());

    let current_len = s.as_str().chars().count();
    if current_len >= target_length || pad_string.is_empty() {
        return Ok(JsValue::String(s));
    }

    let pad_len = target_length - current_len;
    let mut padding = String::new();
    while padding.len() < pad_len {
        padding.push_str(&pad_string);
    }
    padding.truncate(pad_len);

    Ok(JsValue::String(JsString::from(format!("{}{}", padding, s.as_str()))))
}

fn string_pad_end(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let s = this.to_js_string();
    let target_length = args.first().map(|v| v.to_number() as usize).unwrap_or(0);
    let pad_string = args.get(1).map(|v| v.to_js_string().to_string()).unwrap_or_else(|| " ".to_string());

    let current_len = s.as_str().chars().count();
    if current_len >= target_length || pad_string.is_empty() {
        return Ok(JsValue::String(s));
    }

    let pad_len = target_length - current_len;
    let mut padding = String::new();
    while padding.len() < pad_len {
        padding.push_str(&pad_string);
    }
    padding.truncate(pad_len);

    Ok(JsValue::String(JsString::from(format!("{}{}", s.as_str(), padding))))
}

// Math methods

fn math_abs(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.abs()))
}

fn math_floor(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.floor()))
}

fn math_ceil(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.ceil()))
}

fn math_round(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.round()))
}

fn math_trunc(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.trunc()))
}

fn math_sign(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    let result = if n.is_nan() {
        f64::NAN
    } else if n > 0.0 {
        1.0
    } else if n < 0.0 {
        -1.0
    } else {
        0.0
    };
    Ok(JsValue::Number(result))
}

fn math_min(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    if args.is_empty() {
        return Ok(JsValue::Number(f64::INFINITY));
    }
    let mut min = f64::INFINITY;
    for arg in args {
        let n = arg.to_number();
        if n.is_nan() {
            return Ok(JsValue::Number(f64::NAN));
        }
        if n < min {
            min = n;
        }
    }
    Ok(JsValue::Number(min))
}

fn math_max(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    if args.is_empty() {
        return Ok(JsValue::Number(f64::NEG_INFINITY));
    }
    let mut max = f64::NEG_INFINITY;
    for arg in args {
        let n = arg.to_number();
        if n.is_nan() {
            return Ok(JsValue::Number(f64::NAN));
        }
        if n > max {
            max = n;
        }
    }
    Ok(JsValue::Number(max))
}

fn math_pow(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let base = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    let exp = args.get(1).map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(base.powf(exp)))
}

fn math_sqrt(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.sqrt()))
}

fn math_log(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.ln()))
}

fn math_exp(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.exp()))
}

fn math_random(_interp: &mut Interpreter, _this: JsValue, _args: Vec<JsValue>) -> Result<JsValue, JsError> {
    use std::time::{SystemTime, UNIX_EPOCH};
    // Simple pseudo-random using system time (not cryptographically secure)
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos() as f64;
    let random = (seed / 1_000_000_000.0) % 1.0;
    Ok(JsValue::Number(random))
}

fn math_sin(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.sin()))
}

fn math_cos(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.cos()))
}

fn math_tan(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Number(n.tan()))
}

// Global functions

fn global_parse_int(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let string = args.first().map(|v| v.to_js_string()).unwrap_or_else(|| JsString::from(""));
    let string = string.as_str().to_string();
    let radix = args.get(1).map(|v| v.to_number() as i32).unwrap_or(10);

    // Trim whitespace
    let s = string.trim();

    if s.is_empty() {
        return Ok(JsValue::Number(f64::NAN));
    }

    // Handle radix
    let radix = if radix == 0 { 10 } else { radix };
    if !(2..=36).contains(&radix) {
        return Ok(JsValue::Number(f64::NAN));
    }

    // Handle sign
    let (negative, s) = if let Some(rest) = s.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = s.strip_prefix('+') {
        (false, rest)
    } else {
        (false, s)
    };

    // Handle hex prefix for radix 16
    let s = if radix == 16 {
        s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s)
    } else {
        s
    };

    // Parse digits until invalid character
    let mut result: i64 = 0;
    let mut found_digit = false;

    for c in s.chars() {
        let digit = match c.to_digit(radix as u32) {
            Some(d) => d as i64,
            None => break,
        };
        found_digit = true;
        result = result * (radix as i64) + digit;
    }

    if !found_digit {
        return Ok(JsValue::Number(f64::NAN));
    }

    let result = if negative { -result } else { result };
    Ok(JsValue::Number(result as f64))
}

fn global_parse_float(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let string = args.first().map(|v| v.to_js_string()).unwrap_or_else(|| JsString::from(""));
    let string = string.as_str().to_string();
    let s = string.trim();

    if s.is_empty() {
        return Ok(JsValue::Number(f64::NAN));
    }

    // Find the longest valid float prefix
    let mut end = 0;
    let mut has_dot = false;
    let mut has_exp = false;
    let mut chars = s.chars().peekable();

    // Handle sign
    if matches!(chars.peek(), Some('-') | Some('+')) {
        end += 1;
        chars.next();
    }

    // Parse digits and decimal point
    while let Some(&c) = chars.peek() {
        match c {
            '0'..='9' => {
                end += 1;
                chars.next();
            }
            '.' if !has_dot && !has_exp => {
                has_dot = true;
                end += 1;
                chars.next();
            }
            'e' | 'E' if !has_exp => {
                has_exp = true;
                end += 1;
                chars.next();
                // Optional sign after exponent
                if matches!(chars.peek(), Some('-') | Some('+')) {
                    end += 1;
                    chars.next();
                }
            }
            _ => break,
        }
    }

    let num_str = &s[..end];
    match num_str.parse::<f64>() {
        Ok(n) => Ok(JsValue::Number(n)),
        Err(_) => Ok(JsValue::Number(f64::NAN)),
    }
}

// Global isNaN - converts argument to number first
fn global_is_nan(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Boolean(n.is_nan()))
}

// Global isFinite - converts argument to number first
fn global_is_finite(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = args.first().map(|v| v.to_number()).unwrap_or(f64::NAN);
    Ok(JsValue::Boolean(n.is_finite()))
}

// Number.isNaN - stricter, no type coercion
fn number_is_nan(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    match args.first() {
        Some(JsValue::Number(n)) => Ok(JsValue::Boolean(n.is_nan())),
        _ => Ok(JsValue::Boolean(false)),
    }
}

// Number.isFinite - stricter, no type coercion
fn number_is_finite(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    match args.first() {
        Some(JsValue::Number(n)) => Ok(JsValue::Boolean(n.is_finite())),
        _ => Ok(JsValue::Boolean(false)),
    }
}

// Number.isInteger
fn number_is_integer(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    match args.first() {
        Some(JsValue::Number(n)) => {
            let is_int = n.is_finite() && n.trunc() == *n;
            Ok(JsValue::Boolean(is_int))
        }
        _ => Ok(JsValue::Boolean(false)),
    }
}

// Number.isSafeInteger
fn number_is_safe_integer(_interp: &mut Interpreter, _this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    const MAX_SAFE: f64 = 9007199254740991.0;
    match args.first() {
        Some(JsValue::Number(n)) => {
            let is_safe = n.is_finite() && n.trunc() == *n && n.abs() <= MAX_SAFE;
            Ok(JsValue::Boolean(is_safe))
        }
        _ => Ok(JsValue::Boolean(false)),
    }
}

// Number.prototype.toFixed
fn number_to_fixed(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = this.to_number();
    let digits = args.first().map(|v| v.to_number() as i32).unwrap_or(0);

    if digits < 0 || digits > 100 {
        return Err(JsError::range_error("toFixed() digits argument must be between 0 and 100"));
    }

    let result = format!("{:.prec$}", n, prec = digits as usize);
    Ok(JsValue::String(JsString::from(result)))
}

// Number.prototype.toString
fn number_to_string(_interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let n = this.to_number();
    let radix = args.first().map(|v| v.to_number() as i32).unwrap_or(10);

    if radix < 2 || radix > 36 {
        return Err(JsError::range_error("toString() radix must be between 2 and 36"));
    }

    if radix == 10 {
        return Ok(JsValue::String(JsString::from(format!("{}", n))));
    }

    // For other radixes, we need integer conversion
    if !n.is_finite() || n.fract() != 0.0 {
        return Ok(JsValue::String(JsString::from(format!("{}", n))));
    }

    let int_val = n as i64;
    let result = match radix {
        2 => format!("{:b}", int_val.abs()),
        8 => format!("{:o}", int_val.abs()),
        16 => format!("{:x}", int_val.abs()),
        _ => {
            // Generic radix conversion
            let chars: Vec<char> = "0123456789abcdefghijklmnopqrstuvwxyz".chars().collect();
            let mut num = int_val.abs();
            let mut result = String::new();
            while num > 0 {
                result.insert(0, chars[(num % radix as i64) as usize]);
                num /= radix as i64;
            }
            if result.is_empty() {
                result = "0".to_string();
            }
            result
        }
    };

    let result = if int_val < 0 {
        format!("-{}", result)
    } else {
        result
    };

    Ok(JsValue::String(JsString::from(result)))
}

// JSON conversion helpers

fn js_value_to_json(value: &JsValue) -> Result<serde_json::Value, JsError> {
    Ok(match value {
        JsValue::Undefined => serde_json::Value::Null,
        JsValue::Null => serde_json::Value::Null,
        JsValue::Boolean(b) => serde_json::Value::Bool(*b),
        JsValue::Number(n) => {
            if n.is_finite() {
                serde_json::Value::Number(
                    serde_json::Number::from_f64(*n).unwrap_or(serde_json::Number::from(0)),
                )
            } else {
                serde_json::Value::Null
            }
        }
        JsValue::String(s) => serde_json::Value::String(s.to_string()),
        JsValue::Object(obj) => {
            let obj_ref = obj.borrow();
            match &obj_ref.exotic {
                ExoticObject::Array { length } => {
                    let mut arr = Vec::with_capacity(*length as usize);
                    for i in 0..*length {
                        let val = obj_ref
                            .get_property(&PropertyKey::Index(i))
                            .unwrap_or(JsValue::Undefined);
                        arr.push(js_value_to_json(&val)?);
                    }
                    serde_json::Value::Array(arr)
                }
                ExoticObject::Function(_) => serde_json::Value::Null,
                ExoticObject::Ordinary => {
                    let mut map = serde_json::Map::new();
                    for (key, prop) in obj_ref.properties.iter() {
                        if prop.enumerable {
                            let json_val = js_value_to_json(&prop.value)?;
                            // Skip undefined values in objects
                            if json_val != serde_json::Value::Null || !matches!(prop.value, JsValue::Undefined) {
                                map.insert(key.to_string(), json_val);
                            }
                        }
                    }
                    serde_json::Value::Object(map)
                }
            }
        }
    })
}

fn json_to_js_value(json: &serde_json::Value) -> Result<JsValue, JsError> {
    Ok(match json {
        serde_json::Value::Null => JsValue::Null,
        serde_json::Value::Bool(b) => JsValue::Boolean(*b),
        serde_json::Value::Number(n) => JsValue::Number(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::String(s) => JsValue::String(JsString::from(s.clone())),
        serde_json::Value::Array(arr) => {
            let elements: Result<Vec<_>, _> = arr.iter().map(json_to_js_value).collect();
            JsValue::Object(create_array(elements?))
        }
        serde_json::Value::Object(map) => {
            let obj = create_object();
            for (key, value) in map {
                obj.borrow_mut()
                    .set_property(PropertyKey::from(key.as_str()), json_to_js_value(value)?);
            }
            JsValue::Object(obj)
        }
    })
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
}
