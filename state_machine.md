# State Machine Architecture for Async/Import Support

## Overview

This document describes the architectural changes needed to support ES modules (`import`/`export`) and async/await in the TypeScript interpreter. The key insight is that both features require **suspending execution** and **resuming later** with an external value.

### Design Goals

1. **Host-agnostic suspension**: The interpreter returns a "pending" state; the host decides how to fulfill it (sync/async Rust code)
2. **True state capture**: Save exact interpreter state, resume without re-execution
3. **Composable**: Imports and async operations use the same slot mechanism
4. **No runtime coupling**: The interpreter doesn't know if the host uses tokio, async-std, or blocking I/O

### Execution Model

```
                    ┌─────────────────────┐
                    │   Runtime::eval()   │
                    └──────────┬──────────┘
                               │
                               ▼
              ┌────────────────────────────────┐
              │         RuntimeResult          │
              └────────────────────────────────┘
                    /         |         \
                   /          |          \
                  ▼           ▼           ▼
          ┌──────────┐ ┌────────────┐ ┌─────────────┐
          │ Complete │ │ImportAwaited│ │AsyncAwaited │
          │ (value)  │ │ (slot,spec)│ │(slot,promise)│
          └──────────┘ └────────────┘ └─────────────┘
                             │               │
                             │   Host loads  │  Host resolves
                             │   module      │  promise
                             │               │
                             ▼               ▼
                    ┌─────────────────────────────┐
                    │   slot.set(Ok(value))       │
                    │   slot.set(Err(error))      │
                    └─────────────────────────────┘
                               │
                               ▼
                    ┌─────────────────────────────┐
                    │  Runtime::continue_eval()   │
                    └─────────────────────────────┘
                               │
                               ▼
                          (loop back)
```

---

## Core Types

### RuntimeResult

```rust
/// Result of evaluating TypeScript code
pub enum RuntimeResult {
    /// Execution completed with a final value
    Complete(JsValue),

    /// Execution suspended waiting for a module to be loaded
    ImportAwaited {
        /// Slot to fill with the loaded module
        slot: PendingSlot,
        /// Module specifier (e.g., "./utils" or "lodash")
        specifier: String,
    },

    /// Execution suspended waiting for a promise to resolve
    AsyncAwaited {
        /// Slot to fill with the resolved value
        slot: PendingSlot,
        /// The promise being awaited (for debugging/inspection)
        promise: JsValue,
    },
}
```

### PendingSlot

The slot mechanism allows the host to provide values asynchronously:

```rust
/// A slot that can be filled with a value or error
#[derive(Clone)]
pub struct PendingSlot {
    id: u64,
    value: Rc<RefCell<Option<Result<JsValue, JsError>>>>,
}

impl PendingSlot {
    /// Create a new pending slot
    pub fn new(id: u64) -> Self {
        PendingSlot {
            id,
            value: Rc::new(RefCell::new(None)),
        }
    }

    /// Fill the slot with a successful value
    ///
    /// IMPORTANT: The JsValue MUST be created via Runtime methods
    /// (create_module_from_source, create_value_from_json, etc.)
    /// to ensure proper prototype chains and internal state.
    pub fn set_success(&self, value: JsValue) {
        *self.value.borrow_mut() = Some(Ok(value));
    }

    /// Fill the slot with an error (will be thrown at resume point)
    pub fn set_error(&self, error: JsError) {
        *self.value.borrow_mut() = Some(Err(error));
    }

    /// Check if the slot has been filled
    pub fn is_filled(&self) -> bool {
        self.value.borrow().is_some()
    }

    /// Take the value out of the slot (used internally)
    pub(crate) fn take(&self) -> Option<Result<JsValue, JsError>> {
        self.value.borrow_mut().take()
    }

    /// Get the slot's unique ID
    pub fn id(&self) -> u64 {
        self.id
    }
}
```

---

## Explicit Evaluation Stack

The current interpreter uses Rust's call stack for recursion. To support true state capture, we need an **explicit evaluation stack** that can be saved and restored.

### EvalFrame

Each frame represents a pending operation:

```rust
/// A frame on the evaluation stack
pub enum EvalFrame {
    // ═══════════════════════════════════════════════════════════════
    // Expression Evaluation Frames
    // ═══════════════════════════════════════════════════════════════

    /// Evaluate an expression and push result to value stack
    EvaluateExpr(Expression),

    /// Binary expression: left evaluated, need right
    BinaryRight {
        op: BinaryOp,
        right: Box<Expression>,
        span: Span,
    },

    /// Binary expression: both sides evaluated, compute result
    BinaryComplete {
        op: BinaryOp,
        span: Span,
    },

    /// Unary expression: operand evaluated, apply operator
    UnaryComplete {
        op: UnaryOp,
        span: Span,
    },

    /// Logical expression: left evaluated, may short-circuit
    LogicalRight {
        op: LogicalOp,
        right: Box<Expression>,
        span: Span,
    },

    /// Conditional: condition evaluated, pick branch
    ConditionalBranch {
        consequent: Box<Expression>,
        alternate: Box<Expression>,
        span: Span,
    },

    /// Member access: object evaluated, access property
    MemberAccess {
        property: MemberProperty,
        optional: bool,
        span: Span,
    },

    /// Computed member: object evaluated, need property expression
    ComputedMemberExpr {
        property: Box<Expression>,
        optional: bool,
        span: Span,
    },

    /// Computed member: both evaluated
    ComputedMemberComplete {
        optional: bool,
        span: Span,
    },

    /// Call expression: callee evaluated, evaluate args
    CallArgs {
        args_remaining: Vec<Expression>,
        args_done: Vec<JsValue>,
        optional: bool,
        span: Span,
    },

    /// Call expression: all args evaluated, execute call
    CallExecute {
        args: Vec<JsValue>,
        span: Span,
    },

    /// New expression: constructor evaluated, evaluate args
    NewArgs {
        args_remaining: Vec<Expression>,
        args_done: Vec<JsValue>,
        span: Span,
    },

    /// New expression: ready to construct
    NewExecute {
        args: Vec<JsValue>,
        span: Span,
    },

    /// Array literal: evaluate remaining elements
    ArrayElements {
        elements_remaining: Vec<Option<ArrayElement>>,
        elements_done: Vec<JsValue>,
        span: Span,
    },

    /// Object literal: evaluate remaining properties
    ObjectProperties {
        properties_remaining: Vec<ObjectProperty>,
        properties_done: Vec<(PropertyKey, JsValue)>,
        span: Span,
    },

    /// Assignment: target and value evaluated
    AssignmentComplete {
        target: AssignmentTarget,
        op: AssignmentOp,
        span: Span,
    },

    /// Await expression: promise evaluated, check if resolved
    AwaitPromise {
        span: Span,
    },

    // ═══════════════════════════════════════════════════════════════
    // Statement Execution Frames
    // ═══════════════════════════════════════════════════════════════

    /// Execute a statement
    ExecuteStmt(Statement),

    /// Execute remaining statements in a block
    ExecuteBlock {
        statements: Vec<Statement>,
        index: usize,
        env: Environment,  // Block's environment for cleanup
    },

    /// Variable declaration: initializer evaluated, bind
    VariableDeclarationBind {
        pattern: Pattern,
        kind: VariableKind,
    },

    /// If statement: condition evaluated, pick branch
    IfBranch {
        consequent: Box<Statement>,
        alternate: Option<Box<Statement>>,
    },

    /// For loop: various states
    ForLoopInit {
        init: Option<ForInit>,
        test: Option<Box<Expression>>,
        update: Option<Box<Expression>>,
        body: Box<Statement>,
        label: Option<String>,
    },
    ForLoopTest {
        test: Option<Box<Expression>>,
        update: Option<Box<Expression>>,
        body: Box<Statement>,
        label: Option<String>,
    },
    ForLoopBody {
        test: Option<Box<Expression>>,
        update: Option<Box<Expression>>,
        body: Box<Statement>,
        label: Option<String>,
    },
    ForLoopUpdate {
        test: Option<Box<Expression>>,
        update: Option<Box<Expression>>,
        body: Box<Statement>,
        label: Option<String>,
    },

    /// While loop
    WhileTest {
        test: Box<Expression>,
        body: Box<Statement>,
        label: Option<String>,
    },
    WhileBody {
        test: Box<Expression>,
        body: Box<Statement>,
        label: Option<String>,
    },

    /// Try/catch/finally handling
    TryCatch {
        catch_clause: Option<CatchClause>,
        finally_block: Option<BlockStatement>,
    },
    FinallyBlock {
        block: BlockStatement,
        completion: Completion,  // Original completion to restore
    },

    /// Return statement: value evaluated
    ReturnComplete,

    /// Throw statement: value evaluated
    ThrowComplete {
        span: Span,
    },

    // ═══════════════════════════════════════════════════════════════
    // Function Execution Frames
    // ═══════════════════════════════════════════════════════════════

    /// Function call setup: bind params and execute body
    FunctionSetup {
        func: InterpretedFunction,
        this_value: JsValue,
        args: Vec<JsValue>,
        saved_env: Environment,
    },

    /// Function body execution complete, restore environment
    FunctionTeardown {
        saved_env: Environment,
    },

    // ═══════════════════════════════════════════════════════════════
    // Import/Module Frames
    // ═══════════════════════════════════════════════════════════════

    /// Static import: waiting for module, then bind
    ImportBind {
        slot_id: u64,
        bindings: ImportBindings,
    },

    /// Dynamic import: create promise
    DynamicImportPromise {
        slot_id: u64,
        span: Span,
    },

    // ═══════════════════════════════════════════════════════════════
    // Async Function Frames
    // ═══════════════════════════════════════════════════════════════

    /// Async function: wrap result in promise
    AsyncFunctionWrap {
        result_promise: JsObjectRef,
    },

    /// Await resume: slot filled, continue with value or throw
    AwaitResume {
        slot_id: u64,
    },
}

/// Import bindings from an import declaration
pub enum ImportBindings {
    /// import { a, b as c } from "mod"
    Named(Vec<(String, String)>),  // (imported, local)
    /// import def from "mod"
    Default(String),
    /// import * as ns from "mod"
    Namespace(String),
    /// import "mod" (side-effect only)
    SideEffect,
}
```

### Interpreter State

```rust
pub struct Interpreter {
    // Existing fields...
    pub global: JsObjectRef,
    pub env: Environment,
    pub object_prototype: JsObjectRef,
    // ... other prototypes ...
    pub exports: HashMap<String, JsValue>,
    pub call_stack: Vec<StackFrame>,

    // New fields for state machine execution

    /// Explicit evaluation stack (replaces Rust call stack)
    eval_stack: Vec<EvalFrame>,

    /// Value stack for intermediate results
    value_stack: Vec<JsValue>,

    /// Completion stack for statement results
    completion_stack: Vec<Completion>,

    /// Currently pending slot (if suspended)
    pending_slot: Option<PendingSlot>,

    /// Counter for generating unique slot IDs
    next_slot_id: u64,

    /// Static imports collected during initial parse
    static_imports: Vec<StaticImport>,

    /// Index of next static import to process
    static_import_index: usize,
}

/// A static import declaration
pub struct StaticImport {
    pub specifier: String,
    pub bindings: ImportBindings,
    pub span: Span,
}
```

---

## Import Handling

### Static Imports (Hoisted)

Static imports are hoisted - they must all be resolved before any code executes.

```typescript
// This works because imports are hoisted
console.log(foo);
import { foo } from './module';
```

**Execution flow:**

1. Parse program, collect all `import` declarations into `static_imports`
2. Before executing any statements:
   - For each static import, create slot and return `ImportAwaited`
   - Host loads module, fills slot, calls `continue_eval()`
   - Bind imported names to environment
3. After all imports resolved, execute program statements

```rust
impl Runtime {
    pub fn eval(&mut self, source: &str) -> Result<RuntimeResult, JsError> {
        // 1. Parse
        let program = Parser::parse(source)?;

        // 2. Collect static imports
        self.interpreter.collect_static_imports(&program);

        // 3. Process first import (or start execution if none)
        self.process_next_import_or_execute(&program)
    }

    fn process_next_import_or_execute(&mut self, program: &Program)
        -> Result<RuntimeResult, JsError>
    {
        // Check if there are pending static imports
        if self.interpreter.static_import_index < self.interpreter.static_imports.len() {
            let import = &self.interpreter.static_imports[self.interpreter.static_import_index];
            let slot = self.interpreter.create_pending_slot();

            // Push frame to bind imports when slot is filled
            self.interpreter.eval_stack.push(EvalFrame::ImportBind {
                slot_id: slot.id(),
                bindings: import.bindings.clone(),
            });

            return Ok(RuntimeResult::ImportAwaited {
                slot,
                specifier: import.specifier.clone(),
            });
        }

        // All imports resolved, start execution
        self.interpreter.setup_program_execution(program);
        self.run_until_suspend_or_complete()
    }
}
```

### Dynamic Imports

Dynamic `import()` returns a Promise immediately. Suspension only happens when the Promise is `await`ed.

```typescript
// import() returns Promise immediately
const promise = import('./module');
// ... other code runs ...
const module = await promise;  // Suspension happens here
```

**Execution flow:**

1. Evaluate `import('./module')` expression
2. Create a Promise with an internal slot
3. Return the Promise (no suspension yet)
4. When `await` is called on the Promise:
   - If Promise already resolved: continue synchronously
   - If Promise pending: return `AsyncAwaited` with the slot

```rust
// In expression evaluation
Expression::Import(specifier_expr) => {
    // 1. Evaluate specifier
    let specifier = self.evaluate(specifier_expr)?.to_string();

    // 2. Create slot for module loading
    let slot = self.create_pending_slot();

    // 3. Create Promise with internal slot reference
    let promise = self.create_import_promise(slot.clone(), specifier.clone());

    // 4. Return Promise (no suspension - host can start loading in background)
    // Note: We return the RuntimeResult to notify host, but don't suspend
    self.pending_import_slots.insert(slot.id(), (slot, specifier));

    Ok(JsValue::Object(promise))
}
```

---

## Promise Implementation

### Promise State

```rust
/// Internal state of a Promise
pub enum PromiseState {
    /// Promise is waiting for resolution
    Pending {
        /// Callbacks registered via .then()
        fulfill_reactions: Vec<PromiseReaction>,
        /// Callbacks registered via .catch()
        reject_reactions: Vec<PromiseReaction>,
        /// Slot for external resolution (import promises, etc.)
        external_slot: Option<PendingSlot>,
    },

    /// Promise was fulfilled with a value
    Fulfilled(JsValue),

    /// Promise was rejected with a reason
    Rejected(JsValue),
}

/// A reaction (callback) registered on a promise
pub struct PromiseReaction {
    pub handler: JsValue,  // Function to call
    pub result_promise: Option<JsObjectRef>,  // Promise to resolve with result
}
```

### Promise Methods

```rust
// Promise constructor
// new Promise((resolve, reject) => { ... })
pub fn promise_constructor(
    interp: &mut Interpreter,
    _this: JsValue,
    args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    let executor = args.get(0)
        .ok_or_else(|| JsError::type_error("Promise executor required"))?;

    // Create promise in pending state
    let promise = interp.create_promise(PromiseState::Pending {
        fulfill_reactions: vec![],
        reject_reactions: vec![],
        external_slot: None,
    });

    // Create resolve/reject functions bound to this promise
    let resolve = interp.create_promise_resolve_function(promise.clone());
    let reject = interp.create_promise_reject_function(promise.clone());

    // Call executor synchronously
    // Note: If executor throws, promise is rejected
    match interp.call_function(executor.clone(), JsValue::Undefined, vec![resolve, reject]) {
        Ok(_) => {}
        Err(e) => {
            interp.reject_promise(&promise, e.to_js_value());
        }
    }

    Ok(JsValue::Object(promise))
}

// Promise.prototype.then(onFulfilled, onRejected)
pub fn promise_then(
    interp: &mut Interpreter,
    this: JsValue,
    args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    let promise = this.as_promise()?;
    let on_fulfilled = args.get(0).cloned().unwrap_or(JsValue::Undefined);
    let on_rejected = args.get(1).cloned().unwrap_or(JsValue::Undefined);

    // Create result promise
    let result_promise = interp.create_promise(PromiseState::Pending {
        fulfill_reactions: vec![],
        reject_reactions: vec![],
        external_slot: None,
    });

    let state = promise.borrow().get_promise_state();
    match state {
        PromiseState::Pending { .. } => {
            // Register reactions for later
            promise.borrow_mut().add_fulfill_reaction(PromiseReaction {
                handler: on_fulfilled,
                result_promise: Some(result_promise.clone()),
            });
            promise.borrow_mut().add_reject_reaction(PromiseReaction {
                handler: on_rejected,
                result_promise: Some(result_promise.clone()),
            });
        }
        PromiseState::Fulfilled(value) => {
            // Call handler synchronously (simplified microtask semantics)
            if on_fulfilled.is_callable() {
                match interp.call_function(on_fulfilled, JsValue::Undefined, vec![value]) {
                    Ok(result) => interp.resolve_promise(&result_promise, result),
                    Err(e) => interp.reject_promise(&result_promise, e.to_js_value()),
                }
            } else {
                interp.resolve_promise(&result_promise, value);
            }
        }
        PromiseState::Rejected(reason) => {
            // Call handler synchronously
            if on_rejected.is_callable() {
                match interp.call_function(on_rejected, JsValue::Undefined, vec![reason]) {
                    Ok(result) => interp.resolve_promise(&result_promise, result),
                    Err(e) => interp.reject_promise(&result_promise, e.to_js_value()),
                }
            } else {
                interp.reject_promise(&result_promise, reason);
            }
        }
    }

    Ok(JsValue::Object(result_promise))
}

// Promise.resolve(value)
pub fn promise_resolve(
    interp: &mut Interpreter,
    _this: JsValue,
    args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    let value = args.get(0).cloned().unwrap_or(JsValue::Undefined);

    // If value is already a promise, return it
    if value.is_promise() {
        return Ok(value);
    }

    // Create fulfilled promise
    let promise = interp.create_promise(PromiseState::Fulfilled(value));
    Ok(JsValue::Object(promise))
}

// Promise.reject(reason)
pub fn promise_reject(
    interp: &mut Interpreter,
    _this: JsValue,
    args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    let reason = args.get(0).cloned().unwrap_or(JsValue::Undefined);
    let promise = interp.create_promise(PromiseState::Rejected(reason));
    Ok(JsValue::Object(promise))
}

// Promise.all(iterable)
pub fn promise_all(
    interp: &mut Interpreter,
    _this: JsValue,
    args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    let iterable = args.get(0).ok_or_else(|| JsError::type_error("Promise.all requires iterable"))?;
    let promises = interp.iterate_to_vec(iterable)?;

    if promises.is_empty() {
        return Ok(JsValue::Object(interp.create_promise(
            PromiseState::Fulfilled(JsValue::Object(interp.create_array(vec![])))
        )));
    }

    let result_promise = interp.create_promise(PromiseState::Pending {
        fulfill_reactions: vec![],
        reject_reactions: vec![],
        external_slot: None,
    });

    let remaining = Rc::new(RefCell::new(promises.len()));
    let results = Rc::new(RefCell::new(vec![JsValue::Undefined; promises.len()]));

    for (index, promise_value) in promises.into_iter().enumerate() {
        let promise = if promise_value.is_promise() {
            promise_value
        } else {
            JsValue::Object(interp.create_promise(PromiseState::Fulfilled(promise_value)))
        };

        // Create handlers that collect results
        let remaining_clone = remaining.clone();
        let results_clone = results.clone();
        let result_promise_clone = result_promise.clone();

        // ... register then/catch handlers ...
    }

    Ok(JsValue::Object(result_promise))
}
```

---

## Async/Await

### Async Functions

An `async function` returns a Promise and can use `await` internally.

```typescript
async function fetchData(): Promise<string> {
    const response = await fetch('/api');  // Suspends here
    return response.text();
}
```

**Execution flow:**

1. Call async function
2. Create result Promise for function's eventual return value
3. Execute function body
4. At each `await`:
   - Evaluate awaited expression
   - If Promise is fulfilled: use value, continue
   - If Promise is pending: suspend, return `AsyncAwaited`
5. When function returns: resolve result Promise
6. When function throws: reject result Promise

```rust
// Async function call
fn call_async_function(
    &mut self,
    func: &InterpretedFunction,
    this: JsValue,
    args: Vec<JsValue>,
) -> Result<JsValue, JsError> {
    // Create result promise
    let result_promise = self.create_promise(PromiseState::Pending {
        fulfill_reactions: vec![],
        reject_reactions: vec![],
        external_slot: None,
    });

    // Push frame to wrap result
    self.eval_stack.push(EvalFrame::AsyncFunctionWrap {
        result_promise: result_promise.clone(),
    });

    // Setup function execution
    self.eval_stack.push(EvalFrame::FunctionSetup {
        func: func.clone(),
        this_value: this,
        args,
        saved_env: self.env.clone(),
    });

    // Return promise immediately
    Ok(JsValue::Object(result_promise))
}
```

### Await Expression

```rust
// Await expression evaluation
Expression::Await(expr) => {
    // First evaluate the expression
    self.eval_stack.push(EvalFrame::AwaitPromise { span: expr.span });
    self.eval_stack.push(EvalFrame::EvaluateExpr(*expr.argument.clone()));
}

// Processing AwaitPromise frame
EvalFrame::AwaitPromise { span } => {
    let value = self.value_stack.pop().unwrap();

    // Wrap non-promise in resolved promise
    let promise = if value.is_promise() {
        value.as_object().unwrap()
    } else {
        self.create_promise(PromiseState::Fulfilled(value))
    };

    let state = promise.borrow().get_promise_state();
    match state {
        PromiseState::Fulfilled(value) => {
            // Continue synchronously with the value
            self.value_stack.push(value);
        }
        PromiseState::Rejected(reason) => {
            // Throw the rejection reason
            return Err(JsError::from_js_value(reason));
        }
        PromiseState::Pending { external_slot, .. } => {
            // Need to suspend!
            let slot = external_slot.unwrap_or_else(|| {
                // Create slot and attach to promise
                let slot = self.create_pending_slot();
                promise.borrow_mut().set_external_slot(slot.clone());
                slot
            });

            // Push resume frame
            self.eval_stack.push(EvalFrame::AwaitResume { slot_id: slot.id() });

            // Save state and suspend
            self.pending_slot = Some(slot.clone());
            return Err(JsError::Suspend(RuntimeResult::AsyncAwaited {
                slot,
                promise: JsValue::Object(promise),
            }));
        }
    }
}

// Processing AwaitResume frame (after continue_eval)
EvalFrame::AwaitResume { slot_id } => {
    let slot = self.get_slot(slot_id)?;
    match slot.take() {
        Some(Ok(value)) => {
            self.value_stack.push(value);
        }
        Some(Err(error)) => {
            return Err(error);  // Throws from await point
        }
        None => {
            return Err(JsError::internal("Slot not filled"));
        }
    }
}
```

---

## Runtime API

### Updated Public API

```rust
impl Runtime {
    /// Create a new runtime instance
    pub fn new() -> Self { ... }

    /// Evaluate TypeScript source code
    ///
    /// Returns `RuntimeResult::Complete` if execution finishes,
    /// or `RuntimeResult::ImportAwaited`/`AsyncAwaited` if suspended.
    ///
    /// # Example
    /// ```rust
    /// let mut runtime = Runtime::new();
    /// let mut result = runtime.eval(source)?;
    ///
    /// loop {
    ///     match result {
    ///         RuntimeResult::Complete(value) => {
    ///             println!("Result: {:?}", value);
    ///             break;
    ///         }
    ///         RuntimeResult::ImportAwaited { slot, specifier } => {
    ///             // Load module source (sync or async)
    ///             let source = load_module_source(&specifier)?;
    ///             // IMPORTANT: Create module object via Runtime
    ///             let module = runtime.create_module_from_source(&source)?;
    ///             slot.set_success(module);
    ///         }
    ///         RuntimeResult::AsyncAwaited { slot, promise } => {
    ///             // Get raw data (sync or async)
    ///             let data = resolve_somehow(&promise)?;
    ///             // IMPORTANT: Create JsValue via Runtime
    ///             let value = runtime.create_value_from_json(&data)?;
    ///             slot.set_success(value);
    ///         }
    ///     }
    ///     result = runtime.continue_eval()?;
    /// }
    /// ```
    pub fn eval(&mut self, source: &str) -> Result<RuntimeResult, JsError>;

    /// Continue execution after filling a pending slot
    ///
    /// Call this after receiving `ImportAwaited` or `AsyncAwaited` and
    /// filling the slot with `set_success()` or `set_error()`.
    pub fn continue_eval(&mut self) -> Result<RuntimeResult, JsError>;

    /// Call an exported function by name
    ///
    /// This can also suspend if the function is async or performs imports.
    pub fn call_function(
        &mut self,
        name: &str,
        args: &serde_json::Value,
    ) -> Result<RuntimeResult, JsError>;

    /// Get all exported values (only valid after Complete)
    pub fn get_exports(&self) -> &HashMap<String, JsValue>;

    // ═══════════════════════════════════════════════════════════════
    // Value Creation Methods (for slot filling)
    // ═══════════════════════════════════════════════════════════════
    //
    // IMPORTANT: All JsValues assigned to slots MUST be created via
    // these Runtime methods. This ensures proper prototype chains,
    // internal slots, and runtime state consistency.

    /// Create a module object from TypeScript/JavaScript source
    ///
    /// Parses and evaluates the source, returning the module's namespace object
    /// with all exports as properties.
    pub fn create_module_from_source(&mut self, source: &str) -> Result<JsValue, JsError>;

    /// Create a module object from pre-parsed exports
    ///
    /// Use when you've already evaluated the module elsewhere.
    pub fn create_module_object(&mut self, exports: HashMap<String, JsValue>) -> JsValue;

    /// Create a JsValue from JSON
    ///
    /// Useful for creating values from external data (fetch responses, etc.)
    pub fn create_value_from_json(&mut self, json: &serde_json::Value) -> Result<JsValue, JsError>;

    /// Create an array with the given elements
    pub fn create_array(&mut self, elements: Vec<JsValue>) -> JsValue;

    /// Create a plain object with the given properties
    pub fn create_object(&mut self, properties: HashMap<String, JsValue>) -> JsValue;

    /// Create a string value
    pub fn create_string(&mut self, s: &str) -> JsValue;

    /// Create an Error object
    pub fn create_error(&mut self, message: &str) -> JsValue;

    /// Create a TypeError object
    pub fn create_type_error(&mut self, message: &str) -> JsValue;
}
```

### Helper for Sync-Only Usage

For users who want a simpler API with sync handlers for both imports and async operations:

```rust
impl Runtime {
    /// Evaluate source with sync handlers for imports and async operations
    ///
    /// This is a convenience method when you can handle all operations synchronously
    /// (blocking file I/O, blocking HTTP, blocking sleep, etc.)
    pub fn eval_sync<I, A>(
        &mut self,
        source: &str,
        import_handler: I,
        async_handler: A,
    ) -> Result<JsValue, JsError>
    where
        I: Fn(&mut Runtime, &str) -> Result<JsValue, JsError>,
        A: Fn(&mut Runtime, &JsValue) -> Result<JsValue, JsError>,
    {
        let mut result = self.eval(source)?;

        loop {
            match result {
                RuntimeResult::Complete(value) => return Ok(value),
                RuntimeResult::ImportAwaited { slot, specifier } => {
                    // Handler creates module object via Runtime
                    match import_handler(self, &specifier) {
                        Ok(module) => slot.set_success(module),
                        Err(e) => slot.set_error(e),
                    }
                }
                RuntimeResult::AsyncAwaited { slot, promise } => {
                    // Handler can use blocking I/O, sync fetch, sleep, etc.
                    match async_handler(self, &promise) {
                        Ok(value) => slot.set_success(value),
                        Err(e) => slot.set_error(e),
                    }
                }
            }
            result = self.continue_eval()?;
        }
    }
}

// Example usage with blocking operations:
runtime.eval_sync(
    source,
    |rt, specifier| {
        // Blocking file read
        let source = std::fs::read_to_string(resolve_path(specifier))?;
        rt.create_module_from_source(&source)
    },
    |rt, promise| {
        // Could be blocking fetch, sleep, etc.
        // The promise object may contain hints about what to do
        if is_fetch_promise(promise) {
            let url = get_fetch_url(promise);
            let response = blocking_fetch(&url)?;  // sync HTTP
            rt.create_response_object(response)
        } else if is_timeout_promise(promise) {
            let ms = get_timeout_ms(promise);
            std::thread::sleep(Duration::from_millis(ms));
            Ok(JsValue::Undefined)
        } else {
            Err(JsError::type_error("Unknown async operation"))
        }
    },
)?;
```

---

## Execution Engine

### Main Execution Loop

```rust
impl Interpreter {
    /// Run until completion or suspension
    pub fn run(&mut self) -> Result<RuntimeResult, JsError> {
        loop {
            // Check if we're done
            if self.eval_stack.is_empty() {
                let value = self.value_stack.pop().unwrap_or(JsValue::Undefined);
                return Ok(RuntimeResult::Complete(value));
            }

            // Process next frame
            let frame = self.eval_stack.pop().unwrap();
            match self.process_frame(frame) {
                Ok(()) => continue,
                Err(JsError::Suspend(result)) => return Ok(result),
                Err(e) => {
                    // Error handling: unwind stack looking for catch
                    self.handle_error(e)?;
                }
            }
        }
    }

    /// Process a single evaluation frame
    fn process_frame(&mut self, frame: EvalFrame) -> Result<(), JsError> {
        match frame {
            EvalFrame::EvaluateExpr(expr) => self.setup_expr_evaluation(expr),
            EvalFrame::BinaryRight { op, right, span } => {
                self.eval_stack.push(EvalFrame::BinaryComplete { op, span });
                self.eval_stack.push(EvalFrame::EvaluateExpr(*right));
                Ok(())
            }
            EvalFrame::BinaryComplete { op, span } => {
                let right = self.value_stack.pop().unwrap();
                let left = self.value_stack.pop().unwrap();
                let result = self.apply_binary_op(op, left, right, span)?;
                self.value_stack.push(result);
                Ok(())
            }
            // ... handle all other frame types ...
        }
    }

    /// Setup expression evaluation by pushing appropriate frames
    fn setup_expr_evaluation(&mut self, expr: Expression) -> Result<(), JsError> {
        match expr {
            Expression::Literal(lit) => {
                let value = self.literal_to_value(&lit)?;
                self.value_stack.push(value);
                Ok(())
            }
            Expression::Binary(bin) => {
                self.eval_stack.push(EvalFrame::BinaryRight {
                    op: bin.operator,
                    right: bin.right,
                    span: bin.span,
                });
                self.eval_stack.push(EvalFrame::EvaluateExpr(*bin.left));
                Ok(())
            }
            Expression::Call(call) => {
                // First evaluate callee, then args
                let args: Vec<Expression> = call.arguments.into_iter()
                    .map(|a| a.expression)
                    .collect();
                self.eval_stack.push(EvalFrame::CallArgs {
                    args_remaining: args,
                    args_done: vec![],
                    optional: call.optional,
                    span: call.span,
                });
                self.eval_stack.push(EvalFrame::EvaluateExpr(*call.callee));
                Ok(())
            }
            // ... handle all expression types ...
        }
    }
}
```

---

## Implementation Phases

### Phase 1: Explicit Evaluation Stack (Foundation)

**Goal:** Refactor interpreter to use explicit stack instead of Rust recursion

1. Define `EvalFrame` enum with all expression/statement variants
2. Add `eval_stack` and `value_stack` to Interpreter
3. Implement `run()` main loop
4. Convert `evaluate()` methods to `setup_expr_evaluation()` + frame handlers
5. Convert `execute_statement()` methods to frame handlers
6. Ensure all existing tests pass

**Estimated scope:** ~2000 lines of refactoring

### Phase 2: Static Imports

**Goal:** Support `import` declarations with hoisting

1. Add `StaticImport` collection during parsing
2. Implement `ImportAwaited` result type
3. Implement `ImportBind` frame for binding imports
4. Add `continue_eval()` to Runtime API
5. Add integration tests for imports

### Phase 3: Promise Implementation

**Goal:** Full Promise support

1. Add `PromiseState` to exotic objects
2. Implement Promise constructor
3. Implement `.then()`, `.catch()`, `.finally()`
4. Implement `Promise.resolve()`, `Promise.reject()`
5. Implement `Promise.all()`, `Promise.race()`, `Promise.allSettled()`, `Promise.any()`
6. Add integration tests

### Phase 4: Async/Await

**Goal:** Async functions and await expressions

1. Parse `async function` and `await`
2. Implement async function calls (return Promise)
3. Implement `AwaitPromise` and `AwaitResume` frames
4. Implement `AsyncAwaited` result type
5. Add integration tests

### Phase 5: Dynamic Import

**Goal:** `import()` expression support

1. Parse `import()` call expression
2. Create import promises with slots
3. Integration with await
4. Add integration tests

---

## Error Handling

### Errors During Suspension

When an error occurs, we need to unwind the evaluation stack looking for try/catch:

```rust
impl Interpreter {
    fn handle_error(&mut self, error: JsError) -> Result<(), JsError> {
        // Walk up eval_stack looking for TryCatch frame
        while let Some(frame) = self.eval_stack.pop() {
            match frame {
                EvalFrame::TryCatch { catch_clause, finally_block } => {
                    if let Some(catch) = catch_clause {
                        // Found catch handler
                        self.setup_catch_handler(catch, error)?;
                        return Ok(());
                    }
                    if let Some(finally) = finally_block {
                        // Execute finally, then re-throw
                        self.eval_stack.push(EvalFrame::ThrowComplete {
                            span: Span::default()
                        });
                        self.value_stack.push(error.to_js_value());
                        self.setup_finally(finally);
                        return Ok(());
                    }
                }
                EvalFrame::FunctionTeardown { saved_env } => {
                    // Restore environment before continuing unwind
                    self.env = saved_env;
                }
                _ => {
                    // Skip other frames
                }
            }
        }

        // No catch handler found, propagate error
        Err(error)
    }
}
```

### Slot Errors

When a slot is filled with an error, it throws at the resume point:

```rust
// In AwaitResume processing
EvalFrame::AwaitResume { slot_id } => {
    match self.get_slot(slot_id)?.take() {
        Some(Ok(value)) => {
            self.value_stack.push(value);
            Ok(())
        }
        Some(Err(error)) => {
            // This becomes a throw from the await expression
            Err(error)
        }
        None => {
            Err(JsError::internal("Await resumed without slot being filled"))
        }
    }
}
```

---

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_simple_await() {
    let mut runtime = Runtime::new();

    let result = runtime.eval("
        async function foo() {
            return await Promise.resolve(42);
        }
        foo()
    ").unwrap();

    // Should complete immediately since Promise.resolve is sync
    match result {
        RuntimeResult::Complete(value) => {
            // Value is a Promise
            assert!(value.is_promise());
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn test_import_suspension() {
    let mut runtime = Runtime::new();

    let result = runtime.eval("
        import { foo } from './module';
        foo
    ").unwrap();

    match result {
        RuntimeResult::ImportAwaited { slot, specifier } => {
            assert_eq!(specifier, "./module");

            // Create mock module
            let module = create_mock_module(vec![("foo", JsValue::Number(42.0))]);
            slot.set_success(module);
        }
        _ => panic!("Expected ImportAwaited"),
    }

    // Continue execution
    let result = runtime.continue_eval().unwrap();
    match result {
        RuntimeResult::Complete(value) => {
            assert_eq!(value, JsValue::Number(42.0));
        }
        _ => panic!("Expected Complete"),
    }
}

#[test]
fn test_async_await_suspension() {
    let mut runtime = Runtime::new();

    let result = runtime.eval("
        async function fetchData() {
            const data = await externalFetch();
            return data + 1;
        }
        fetchData()
    ").unwrap();

    // externalFetch returns a pending promise
    match result {
        RuntimeResult::AsyncAwaited { slot, .. } => {
            // Simulate async resolution
            slot.set_success(JsValue::Number(41.0));
        }
        _ => panic!("Expected AsyncAwaited"),
    }

    let result = runtime.continue_eval().unwrap();
    // Result is still a Promise (the async function's return)
    // We'd need to await that too in a real scenario
}
```

### Integration Tests

- Test multiple sequential imports
- Test nested async/await
- Test error propagation through await
- Test Promise.all with mixed sync/async
- Test import cycles (should be handled by host)

---

## Migration Guide

### For Existing Users

**Before (sync-only):**
```rust
let mut runtime = Runtime::new();
let result = runtime.eval("1 + 2")?;
println!("{:?}", result);  // JsValue::Number(3.0)
```

**After (still works for sync code):**
```rust
let mut runtime = Runtime::new();
match runtime.eval("1 + 2")? {
    RuntimeResult::Complete(result) => {
        println!("{:?}", result);  // JsValue::Number(3.0)
    }
    _ => unreachable!("No imports or async in this code"),
}
```

**With imports:**
```rust
let mut runtime = Runtime::new();
let mut result = runtime.eval("import { x } from './mod'; x")?;

loop {
    match result {
        RuntimeResult::Complete(value) => {
            println!("Final: {:?}", value);
            break;
        }
        RuntimeResult::ImportAwaited { slot, specifier } => {
            let module = my_module_loader(&specifier)?;
            slot.set_success(module);
            result = runtime.continue_eval()?;
        }
        RuntimeResult::AsyncAwaited { slot, .. } => {
            // Handle async...
            result = runtime.continue_eval()?;
        }
    }
}
```

---

## Open Questions

1. **Top-level await:** Should we support `await` at module top level?
   - Decision: Yes, treat module as implicit async function

2. **Module caching:** Should runtime cache modules internally?
   - Decision: No, host handles caching. Runtime always asks for module.

3. **Circular imports:** How to handle?
   - Decision: Return partially-initialized module object (ES6 semantics)

4. **Generator + async interaction:** `async function*`?
   - Decision: Defer to later milestone

5. **unhandledrejection:** Global event for unhandled promise rejections?
   - Decision: Defer to later milestone
