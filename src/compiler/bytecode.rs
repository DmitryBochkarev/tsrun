//! Bytecode instruction set and chunk format
//!
//! This module defines the bytecode format used by the VM.
//! We use a register-based design with up to 256 virtual registers.

use crate::lexer::Span;
use crate::value::JsString;
use std::rc::Rc;

/// Virtual register index (0-255)
pub type Register = u8;

/// Constant pool index (0-65535)
pub type ConstantIndex = u16;

/// Jump target (instruction offset)
pub type JumpTarget = u32;

/// Bytecode instruction
///
/// Each instruction operates on virtual registers. The register-based design
/// generates fewer instructions than a stack-based VM and has better cache locality.
#[derive(Debug, Clone)]
pub enum Op {
    // ═══════════════════════════════════════════════════════════════════════════════
    // Constants & Register Operations
    // ═══════════════════════════════════════════════════════════════════════════════
    /// Load constant from pool: r[dst] = constants[idx]
    LoadConst { dst: Register, idx: ConstantIndex },

    /// Load undefined: r[dst] = undefined
    LoadUndefined { dst: Register },

    /// Load null: r[dst] = null
    LoadNull { dst: Register },

    /// Load boolean: r[dst] = value
    LoadBool { dst: Register, value: bool },

    /// Load integer (small numbers without constant pool): r[dst] = value
    LoadInt { dst: Register, value: i32 },

    /// Move register: r[dst] = r[src]
    Move { dst: Register, src: Register },

    // ═══════════════════════════════════════════════════════════════════════════════
    // Binary Arithmetic Operations
    // ═══════════════════════════════════════════════════════════════════════════════
    /// Add: r[dst] = r[left] + r[right]
    Add {
        dst: Register,
        left: Register,
        right: Register,
    },

    /// Subtract: r[dst] = r[left] - r[right]
    Sub {
        dst: Register,
        left: Register,
        right: Register,
    },

    /// Multiply: r[dst] = r[left] * r[right]
    Mul {
        dst: Register,
        left: Register,
        right: Register,
    },

    /// Divide: r[dst] = r[left] / r[right]
    Div {
        dst: Register,
        left: Register,
        right: Register,
    },

    /// Modulo: r[dst] = r[left] % r[right]
    Mod {
        dst: Register,
        left: Register,
        right: Register,
    },

    /// Exponentiation: r[dst] = r[left] ** r[right]
    Exp {
        dst: Register,
        left: Register,
        right: Register,
    },

    // ═══════════════════════════════════════════════════════════════════════════════
    // Comparison Operations
    // ═══════════════════════════════════════════════════════════════════════════════
    /// Loose equality: r[dst] = r[left] == r[right]
    Eq {
        dst: Register,
        left: Register,
        right: Register,
    },

    /// Loose inequality: r[dst] = r[left] != r[right]
    NotEq {
        dst: Register,
        left: Register,
        right: Register,
    },

    /// Strict equality: r[dst] = r[left] === r[right]
    StrictEq {
        dst: Register,
        left: Register,
        right: Register,
    },

    /// Strict inequality: r[dst] = r[left] !== r[right]
    StrictNotEq {
        dst: Register,
        left: Register,
        right: Register,
    },

    /// Less than: r[dst] = r[left] < r[right]
    Lt {
        dst: Register,
        left: Register,
        right: Register,
    },

    /// Less than or equal: r[dst] = r[left] <= r[right]
    LtEq {
        dst: Register,
        left: Register,
        right: Register,
    },

    /// Greater than: r[dst] = r[left] > r[right]
    Gt {
        dst: Register,
        left: Register,
        right: Register,
    },

    /// Greater than or equal: r[dst] = r[left] >= r[right]
    GtEq {
        dst: Register,
        left: Register,
        right: Register,
    },

    // ═══════════════════════════════════════════════════════════════════════════════
    // Bitwise Operations
    // ═══════════════════════════════════════════════════════════════════════════════
    /// Bitwise AND: r[dst] = r[left] & r[right]
    BitAnd {
        dst: Register,
        left: Register,
        right: Register,
    },

    /// Bitwise OR: r[dst] = r[left] | r[right]
    BitOr {
        dst: Register,
        left: Register,
        right: Register,
    },

    /// Bitwise XOR: r[dst] = r[left] ^ r[right]
    BitXor {
        dst: Register,
        left: Register,
        right: Register,
    },

    /// Left shift: r[dst] = r[left] << r[right]
    LShift {
        dst: Register,
        left: Register,
        right: Register,
    },

    /// Signed right shift: r[dst] = r[left] >> r[right]
    RShift {
        dst: Register,
        left: Register,
        right: Register,
    },

    /// Unsigned right shift: r[dst] = r[left] >>> r[right]
    URShift {
        dst: Register,
        left: Register,
        right: Register,
    },

    // ═══════════════════════════════════════════════════════════════════════════════
    // Special Binary Operations
    // ═══════════════════════════════════════════════════════════════════════════════
    /// In operator: r[dst] = r[left] in r[right]
    In {
        dst: Register,
        left: Register,
        right: Register,
    },

    /// Instanceof: r[dst] = r[left] instanceof r[right]
    Instanceof {
        dst: Register,
        left: Register,
        right: Register,
    },

    // ═══════════════════════════════════════════════════════════════════════════════
    // Unary Operations
    // ═══════════════════════════════════════════════════════════════════════════════
    /// Negate: r[dst] = -r[src]
    Neg { dst: Register, src: Register },

    /// Unary plus: r[dst] = +r[src] (ToNumber)
    Plus { dst: Register, src: Register },

    /// Logical not: r[dst] = !r[src]
    Not { dst: Register, src: Register },

    /// Bitwise not: r[dst] = ~r[src]
    BitNot { dst: Register, src: Register },

    /// Typeof: r[dst] = typeof r[src]
    Typeof { dst: Register, src: Register },

    /// Void: r[dst] = void r[src] (always undefined)
    Void { dst: Register, src: Register },

    // ═══════════════════════════════════════════════════════════════════════════════
    // Control Flow
    // ═══════════════════════════════════════════════════════════════════════════════
    /// Unconditional jump
    Jump { target: JumpTarget },

    /// Jump if r[cond] is truthy
    JumpIfTrue { cond: Register, target: JumpTarget },

    /// Jump if r[cond] is falsy
    JumpIfFalse { cond: Register, target: JumpTarget },

    /// Jump if r[cond] is null or undefined (for ??)
    JumpIfNullish { cond: Register, target: JumpTarget },

    /// Jump if r[cond] is NOT null or undefined (for ?.)
    JumpIfNotNullish { cond: Register, target: JumpTarget },

    /// Break to target (runs finally blocks first)
    /// try_depth is the try stack depth at the target loop
    Break { target: JumpTarget, try_depth: u8 },

    /// Continue to target (runs finally blocks first)
    /// try_depth is the try stack depth at the target loop
    Continue { target: JumpTarget, try_depth: u8 },

    // ═══════════════════════════════════════════════════════════════════════════════
    // Variable Access
    // ═══════════════════════════════════════════════════════════════════════════════
    /// Load variable: r[dst] = env[name]
    GetVar { dst: Register, name: ConstantIndex },

    /// Try to load variable, returns undefined if not found: r[dst] = env[name] ?? undefined
    TryGetVar { dst: Register, name: ConstantIndex },

    /// Store variable: env[name] = r[src]
    SetVar { name: ConstantIndex, src: Register },

    /// Declare variable with let/const: env.define(name, r[init], mutable)
    DeclareVar {
        name: ConstantIndex,
        init: Register,
        mutable: bool,
    },

    /// Declare variable with var (hoisted): env.define_var(name, r[init])
    DeclareVarHoisted { name: ConstantIndex, init: Register },

    /// Get global variable (optimized path for globals)
    GetGlobal { dst: Register, name: ConstantIndex },

    /// Set global variable
    SetGlobal { name: ConstantIndex, src: Register },

    // ═══════════════════════════════════════════════════════════════════════════════
    // Object/Array Operations
    // ═══════════════════════════════════════════════════════════════════════════════
    /// Create empty object: r[dst] = {}
    CreateObject { dst: Register },

    /// Create array from registers: r[dst] = [r[start]..r[start+count]]
    CreateArray {
        dst: Register,
        start: Register,
        count: u16,
    },

    /// Get property with computed key: r[dst] = r[obj][r[key]]
    GetProperty {
        dst: Register,
        obj: Register,
        key: Register,
    },

    /// Get property with constant key: r[dst] = r[obj].name
    GetPropertyConst {
        dst: Register,
        obj: Register,
        key: ConstantIndex,
    },

    /// Set property with computed key: r[obj][r[key]] = r[value]
    SetProperty {
        obj: Register,
        key: Register,
        value: Register,
    },

    /// Set property with constant key: r[obj].name = r[value]
    SetPropertyConst {
        obj: Register,
        key: ConstantIndex,
        value: Register,
    },

    /// Delete property: r[dst] = delete r[obj][r[key]]
    DeleteProperty {
        dst: Register,
        obj: Register,
        key: Register,
    },

    /// Delete property with constant key: r[dst] = delete r[obj].name
    DeletePropertyConst {
        dst: Register,
        obj: Register,
        key: ConstantIndex,
    },

    /// Define property with descriptor (for object literals with getters/setters)
    DefineProperty {
        obj: Register,
        key: Register,
        value: Register,
        flags: u8, // writable, enumerable, configurable
    },

    // ═══════════════════════════════════════════════════════════════════════════════
    // Function Operations
    // ═══════════════════════════════════════════════════════════════════════════════
    /// Call function: r[dst] = r[callee].call(r[this], r[args_start..args_start+argc])
    Call {
        dst: Register,
        callee: Register,
        this: Register,
        args_start: Register,
        argc: u8,
    },

    /// Call with spread arguments (some args may need spreading)
    CallSpread {
        dst: Register,
        callee: Register,
        this: Register,
        args_start: Register,
        argc: u8,
    },

    /// Direct eval call: r[dst] = eval(r[arg])
    /// This is a special form that preserves the calling scope for direct eval.
    /// Unlike regular Call, direct eval has access to the lexical scope.
    DirectEval { dst: Register, arg: Register },

    /// Call method: r[dst] = r[obj].name(args...)
    /// Optimized form that preserves `this` correctly
    CallMethod {
        dst: Register,
        obj: Register,
        method: ConstantIndex,
        args_start: Register,
        argc: u8,
    },

    /// Construct: r[dst] = new r[callee](r[args_start..args_start+argc])
    Construct {
        dst: Register,
        callee: Register,
        args_start: Register,
        argc: u8,
    },

    /// Construct with spread arguments (args_start points to an args array)
    ConstructSpread {
        dst: Register,
        callee: Register,
        args_start: Register,
        argc: u8,
    },

    /// Return from function with value
    Return { value: Register },

    /// Return undefined from function
    ReturnUndefined,

    /// Create closure from bytecode chunk: r[dst] = function from chunk[idx]
    CreateClosure {
        dst: Register,
        chunk_idx: ConstantIndex,
    },

    /// Create arrow function (captures lexical this)
    CreateArrow {
        dst: Register,
        chunk_idx: ConstantIndex,
    },

    /// Create generator function
    CreateGenerator {
        dst: Register,
        chunk_idx: ConstantIndex,
    },

    /// Create async function
    CreateAsync {
        dst: Register,
        chunk_idx: ConstantIndex,
    },

    /// Create async generator function
    CreateAsyncGenerator {
        dst: Register,
        chunk_idx: ConstantIndex,
    },

    // ═══════════════════════════════════════════════════════════════════════════════
    // Exception Handling
    // ═══════════════════════════════════════════════════════════════════════════════
    /// Throw exception: throw r[value]
    Throw { value: Register },

    /// Push try handler with catch at catch_target
    /// If finally_target is 0, there's no finally block
    PushTry {
        catch_target: JumpTarget,
        finally_target: JumpTarget,
    },

    /// Pop try handler (normal completion)
    PopTry,

    /// End of finally block - complete any pending return/throw
    FinallyEnd,

    /// Get caught exception value: r[dst] = caught_exception
    GetException { dst: Register },

    /// Push iterator try handler - like PushTry but calls iterator.return() on exception
    /// Used by for-of loops to implement iterator close protocol on throws
    PushIterTry {
        iterator: Register,
        catch_target: JumpTarget,
    },

    /// Pop iterator try handler (normal exit, no iterator close needed)
    PopIterTry,

    /// Rethrow current exception (in catch block)
    Rethrow,

    // ═══════════════════════════════════════════════════════════════════════════════
    // Async/Generator
    // ═══════════════════════════════════════════════════════════════════════════════
    /// Await: suspend execution, r[dst] = await r[promise]
    Await { dst: Register, promise: Register },

    /// Yield: suspend generator, r[dst] = yield r[value]
    Yield { dst: Register, value: Register },

    /// Yield*: delegate to iterable, r[dst] = yield* r[iterable]
    YieldStar { dst: Register, iterable: Register },

    // ═══════════════════════════════════════════════════════════════════════════════
    // Scope Management
    // ═══════════════════════════════════════════════════════════════════════════════
    /// Push new lexical scope
    PushScope,

    /// Pop lexical scope
    PopScope,

    // ═══════════════════════════════════════════════════════════════════════════════
    // Iteration
    // ═══════════════════════════════════════════════════════════════════════════════
    /// Get iterator: r[dst] = r[obj][Symbol.iterator]()
    GetIterator { dst: Register, obj: Register },

    /// Get keys iterator for for-in loops: iterates over own enumerable string keys
    GetKeysIterator { dst: Register, obj: Register },

    /// Get async iterator: r[dst] = r[obj][Symbol.asyncIterator]()
    GetAsyncIterator { dst: Register, obj: Register },

    /// Iterator next: r[dst] = r[iterator].next()
    IteratorNext { dst: Register, iterator: Register },

    /// Check if iterator result is done: jump if r[result].done
    IteratorDone {
        result: Register,
        target: JumpTarget,
    },

    /// Get iterator result value: r[dst] = r[result].value
    IteratorValue { dst: Register, result: Register },

    /// Close iterator: call r[iterator].return() if it exists
    /// Used for early loop exit (break, return, throw) in for-of loops
    IteratorClose { iterator: Register },

    // ═══════════════════════════════════════════════════════════════════════════════
    // Class Operations
    // ═══════════════════════════════════════════════════════════════════════════════
    /// Create class: r[dst] = class with r[constructor] and r[super_class]
    CreateClass {
        dst: Register,
        constructor: Register,
        super_class: Register,
    },

    /// Define class method on prototype
    DefineMethod {
        class: Register,
        name: ConstantIndex,
        method: Register,
        is_static: bool,
    },

    /// Define getter/setter
    DefineAccessor {
        class: Register,
        name: ConstantIndex,
        getter: Register,
        setter: Register,
        is_static: bool,
    },

    /// Define class method with computed key
    DefineMethodComputed {
        class: Register,
        key: Register,
        method: Register,
        is_static: bool,
    },

    /// Define getter/setter with computed key
    DefineAccessorComputed {
        class: Register,
        key: Register,
        getter: Register,
        setter: Register,
        is_static: bool,
    },

    /// Super call: r[dst] = super(args...)
    SuperCall {
        dst: Register,
        args_start: Register,
        argc: u8,
    },

    /// Super property get: r[dst] = super[r[key]]
    SuperGet { dst: Register, key: Register },

    /// Super property get with constant key: r[dst] = super.name
    SuperGetConst { dst: Register, key: ConstantIndex },

    /// Super property set: super[r[key]] = r[value]
    SuperSet { key: Register, value: Register },

    /// Super property set with constant key: super.name = r[value]
    SuperSetConst { key: ConstantIndex, value: Register },

    /// Apply class decorator: r[class] = decorator(r[class], context)
    /// The class_name constant is optional (ConstantIndex::MAX means no name)
    /// The initializers register holds an array that addInitializer() pushes callbacks to
    ApplyClassDecorator {
        class: Register,
        decorator: Register,
        class_name: ConstantIndex,
        initializers: Register,
    },

    /// Run class decorator initializers: calls each function in r[initializers] with r[class] as `this`
    RunClassInitializers {
        class: Register,
        initializers: Register,
    },

    /// Apply method decorator: r[method] = decorator(r[method], context)
    /// context contains { kind: "method"|"getter"|"setter", name, static, private }
    ApplyMethodDecorator {
        method: Register,
        decorator: Register,
        name: ConstantIndex,
        kind: u8, // 0 = method, 1 = getter, 2 = setter
        is_static: bool,
        is_private: bool,
    },

    /// Apply parameter decorator: decorator(r[target], context)
    /// context contains { kind: "parameter", name, function, index, static }
    /// Parameter decorators are called for side effects only (like metadata registration)
    ApplyParameterDecorator {
        target: Register, // The class (for constructor params) or method function
        decorator: Register,
        method_name: ConstantIndex, // The method/constructor name
        param_name: ConstantIndex,  // The parameter name (may be empty)
        param_index: u8,
        is_static: bool,
    },

    /// Apply field decorator: r[dst] = decorator(undefined, context)
    /// Field decorators receive undefined as first arg, return an initializer transformer
    /// For auto-accessors, is_accessor=true and context.kind will be "accessor" instead of "field"
    ApplyFieldDecorator {
        dst: Register,
        decorator: Register,
        name: ConstantIndex,
        is_static: bool,
        is_private: bool,
        is_accessor: bool,
    },

    /// Store field initializer on class: class.__field_initializers__[name] = r[initializer]
    /// This is used to store the result of field decorators on the class
    StoreFieldInitializer {
        class: Register,
        name: ConstantIndex,
        initializer: Register,
    },

    /// Get field initializer from class: r[dst] = class.__field_initializers__[name]
    /// Used during instance construction to retrieve stored initializers
    GetFieldInitializer {
        dst: Register,
        class: Register,
        name: ConstantIndex,
    },

    /// Apply field initializer: r[value] = r[initializer](r[value])
    /// Transforms the initial value using the decorator's returned initializer
    ApplyFieldInitializer {
        value: Register,
        initializer: Register,
    },

    /// Define auto-accessor property: creates getter/setter and defines them on prototype
    /// r[target_dst] = { get, set } object for decorator use (or undefined if no decorators)
    /// The accessor is defined on the class prototype (or class itself if is_static)
    DefineAutoAccessor {
        class: Register,
        name: ConstantIndex,
        init_value: Register, // Initial value register
        target_dst: Register, // Destination for { get, set } target object
        is_static: bool,
    },

    /// Store auto-accessor (decorated getter/setter) on class
    /// Takes the decorated { get, set } object and defines the accessor property
    StoreAutoAccessor {
        class: Register,
        name: ConstantIndex,
        accessor_obj: Register, // Object with { get, set } properties
        is_static: bool,
    },

    /// Apply auto-accessor decorator: r[target] = decorator(r[target], context)
    /// context contains { kind: "accessor", name, static }
    ApplyAutoAccessorDecorator {
        target: Register, // { get, set } object, mutated in place
        decorator: Register,
        name: ConstantIndex,
        is_static: bool,
    },

    // ═══════════════════════════════════════════════════════════════════════════════
    // Spread/Rest
    // ═══════════════════════════════════════════════════════════════════════════════
    /// Spread array into registers for function calls
    /// Copies elements from r[src] into r[dst..dst+actual_count]
    /// Returns actual count in a hidden register
    SpreadArray { dst: Register, src: Register },

    /// Create rest array from remaining arguments
    CreateRestArray { dst: Register, start_index: u8 },

    /// Create object rest from source object, excluding specified keys
    /// excluded_keys is a constant index pointing to a Vec<JsString> in the constant pool
    CreateObjectRest {
        dst: Register,
        src: Register,
        excluded_keys: ConstantIndex,
    },

    /// Spread object properties: copy all enumerable own properties from r[src] to r[dst]
    SpreadObject { dst: Register, src: Register },

    // ═══════════════════════════════════════════════════════════════════════════════
    // Template Literals
    // ═══════════════════════════════════════════════════════════════════════════════
    /// Concatenate template parts: r[dst] = r[start..start+count].join()
    TemplateConcat {
        dst: Register,
        start: Register,
        count: u8,
    },

    /// Tagged template call
    TaggedTemplate {
        dst: Register,
        tag: Register,
        this: Register,
        template: ConstantIndex,
        exprs_start: Register,
        exprs_count: u8,
    },

    // ═══════════════════════════════════════════════════════════════════════════════
    // Private Class Members
    // ═══════════════════════════════════════════════════════════════════════════════
    /// Get private field: r[dst] = r[obj].#name
    /// class_brand is a unique id per class definition (for brand checking)
    /// field_name is a constant index for the field name (including # prefix)
    GetPrivateField {
        dst: Register,
        obj: Register,
        class_brand: u32,
        field_name: ConstantIndex,
    },

    /// Set private field: r[obj].#name = r[value]
    SetPrivateField {
        obj: Register,
        class_brand: u32,
        field_name: ConstantIndex,
        value: Register,
    },

    /// Define private field on object: r[obj].#name = r[value]
    /// Called during instance creation to install private fields
    DefinePrivateField {
        obj: Register,
        class_brand: u32,
        field_name: ConstantIndex,
        value: Register,
    },

    /// Define private method on class (stored in constructor for later installation)
    /// When constructing instances, these get copied to the instance's private_fields
    DefinePrivateMethod {
        class: Register,
        class_brand: u32,
        method_name: ConstantIndex,
        method: Register,
        is_static: bool,
    },

    /// Install stored private method on instance during construction
    /// Reads method from new.target's __private_methods__ and installs on this
    InstallPrivateMethod {
        class_brand: u32,
        method_name: ConstantIndex,
    },

    // ═══════════════════════════════════════════════════════════════════════════════
    // Function Name Inference
    // ═══════════════════════════════════════════════════════════════════════════════
    /// Set function name if the value is an anonymous function
    /// This implements the SetFunctionName abstract operation from the spec.
    /// If r[func] is a function without a name, sets its name to the constant.
    /// If it already has a name or is not a function, this is a no-op.
    SetFunctionName { func: Register, name: ConstantIndex },

    // ═══════════════════════════════════════════════════════════════════════════════
    // Miscellaneous
    // ═══════════════════════════════════════════════════════════════════════════════
    /// No operation (used for alignment/patching)
    Nop,

    /// Halt execution (end of program)
    Halt,

    /// Debugger statement
    Debugger,

    /// Pop value from stack (discard expression result)
    Pop,

    /// Duplicate value: r[dst] = r[src] (same as Move but semantically different)
    Dup { dst: Register, src: Register },

    /// Load `this` value: r[dst] = this
    LoadThis { dst: Register },

    /// Load `arguments` object: r[dst] = arguments
    LoadArguments { dst: Register },

    /// Load `new.target`: r[dst] = new.target
    LoadNewTarget { dst: Register },

    // ═══════════════════════════════════════════════════════════════════════════════
    // Module Operations
    // ═══════════════════════════════════════════════════════════════════════════════
    /// Export a binding: exports[export_name] = { name: binding_name, value: r[value] }
    /// Used for `export const foo = ...` and `export { foo }`
    ExportBinding {
        export_name: ConstantIndex,
        binding_name: ConstantIndex,
        value: Register,
    },

    /// Export a namespace re-export: exports[name] = module_namespace
    /// Used for `export * as ns from "module"`
    ExportNamespace {
        export_name: ConstantIndex,
        module_specifier: ConstantIndex,
    },

    /// Re-export from another module: exports[export_name] = { from: source_module, key: source_key }
    /// Used for `export { foo } from "./bar"`
    ReExport {
        export_name: ConstantIndex,
        source_module: ConstantIndex,
        source_key: ConstantIndex,
    },
}

/// A compiled chunk of bytecode
#[derive(Debug, Clone)]
pub struct BytecodeChunk {
    /// The bytecode instructions
    pub code: Vec<Op>,

    /// Constant pool (strings, numbers, nested chunks)
    pub constants: Vec<Constant>,

    /// Source map: instruction index -> source span
    pub source_map: Vec<SourceMapEntry>,

    /// Number of registers needed for this chunk
    pub register_count: u8,

    /// Function metadata (if this is a function body)
    pub function_info: Option<FunctionInfo>,
}

/// Source map entry for debugging
#[derive(Debug, Clone)]
pub struct SourceMapEntry {
    /// Bytecode instruction index
    pub bytecode_offset: usize,
    /// Source location
    pub span: Span,
}

/// Constants that can be stored in the pool
#[derive(Debug, Clone)]
pub enum Constant {
    /// String constant (interned)
    String(JsString),

    /// Number constant
    Number(f64),

    /// Nested bytecode chunk (for closures)
    Chunk(Rc<BytecodeChunk>),

    /// Regular expression (pattern, flags)
    RegExp { pattern: JsString, flags: JsString },

    /// Template strings for tagged templates (cooked strings, raw strings)
    TemplateStrings {
        cooked: Vec<JsString>,
        raw: Vec<JsString>,
    },

    /// List of keys to exclude in object rest destructuring
    ExcludedKeys(Vec<JsString>),
}

/// Function metadata
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    /// Function name (if any)
    pub name: Option<JsString>,

    /// Number of parameters
    pub param_count: usize,

    /// Whether this is a generator function
    pub is_generator: bool,

    /// Whether this is an async function
    pub is_async: bool,

    /// Whether this is an arrow function
    pub is_arrow: bool,

    /// Whether function uses `arguments`
    pub uses_arguments: bool,

    /// Whether function uses `this`
    pub uses_this: bool,

    /// Parameter names (for creating environment)
    pub param_names: Vec<JsString>,

    /// Rest parameter index (if any)
    pub rest_param: Option<usize>,

    /// Expected number of bindings in the function's environment
    /// Used to pre-size the HashMap to avoid resizing during execution
    pub binding_count: usize,
}

impl BytecodeChunk {
    /// Create a new empty bytecode chunk
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
            source_map: Vec::new(),
            register_count: 0,
            function_info: None,
        }
    }

    /// Get the instruction at the given offset
    pub fn get(&self, offset: usize) -> Option<&Op> {
        self.code.get(offset)
    }

    /// Get the source location for a bytecode offset
    pub fn get_source_location(&self, offset: usize) -> Option<Span> {
        // Binary search for the entry
        let idx = self
            .source_map
            .binary_search_by_key(&offset, |e| e.bytecode_offset);

        match idx {
            Ok(i) => self.source_map.get(i).map(|e| e.span),
            Err(i) if i > 0 => self.source_map.get(i - 1).map(|e| e.span),
            _ => None,
        }
    }

    /// Get a constant from the pool
    pub fn get_constant(&self, idx: ConstantIndex) -> Option<&Constant> {
        self.constants.get(idx as usize)
    }
}

impl Default for BytecodeChunk {
    fn default() -> Self {
        Self::new()
    }
}

impl FunctionInfo {
    /// Create info for a regular function
    pub fn regular(name: Option<JsString>, param_count: usize) -> Self {
        Self {
            name,
            param_count,
            is_generator: false,
            is_async: false,
            is_arrow: false,
            uses_arguments: false,
            uses_this: false,
            param_names: Vec::new(),
            rest_param: None,
            binding_count: 0,
        }
    }

    /// Create info for an arrow function
    pub fn arrow(param_count: usize) -> Self {
        Self {
            name: None,
            param_count,
            is_generator: false,
            is_async: false,
            is_arrow: true,
            uses_arguments: false,
            uses_this: false,
            param_names: Vec::new(),
            rest_param: None,
            binding_count: 0,
        }
    }

    /// Create info for a generator function
    pub fn generator(name: Option<JsString>, param_count: usize) -> Self {
        Self {
            name,
            param_count,
            is_generator: true,
            is_async: false,
            is_arrow: false,
            uses_arguments: false,
            uses_this: false,
            param_names: Vec::new(),
            rest_param: None,
            binding_count: 0,
        }
    }

    /// Create info for an async function
    pub fn async_fn(name: Option<JsString>, param_count: usize) -> Self {
        Self {
            name,
            param_count,
            is_generator: false,
            is_async: true,
            is_arrow: false,
            uses_arguments: false,
            uses_this: false,
            param_names: Vec::new(),
            rest_param: None,
            binding_count: 0,
        }
    }
}
