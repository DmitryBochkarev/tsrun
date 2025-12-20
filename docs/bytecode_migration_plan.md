# Bytecode Interpreter Migration Plan

## Overview

Migrate the TypeScript interpreter from AST-based frame interpretation to bytecode-based execution for improved performance.

**Current state**: The interpreter already uses a stack-based execution model (`src/interpreter/stack.rs`) with `Frame` enum variants representing pending operations. This provides an excellent foundation for bytecode migration.

**Goals**: Performance improvement, cleaner execution model.

**Strategy**: Incremental migration with feature flag, keeping both implementations during transition.

---

## Phase 1: Bytecode Format & Infrastructure

### 1.1 Create compiler module structure

```
src/compiler/
├── mod.rs           # Module exports, compile() entry point
├── bytecode.rs      # Op enum, BytecodeChunk, Constant
├── builder.rs       # BytecodeBuilder for emitting instructions
├── compile_expr.rs  # Expression compilation
├── compile_stmt.rs  # Statement compilation
└── compile_pattern.rs # Destructuring pattern compilation
```

### 1.2 Define bytecode instruction set

**File**: `src/compiler/bytecode.rs`

Use a **register-based VM** (not stack-based) for better performance:

```rust
pub type Register = u8;        // 256 registers max
pub type ConstantIndex = u16;  // 64K constants
pub type JumpTarget = u32;     // Instruction offset

pub enum Op {
    // Constants & Registers
    LoadConst { dst: Register, idx: ConstantIndex },
    LoadUndefined { dst: Register },
    LoadNull { dst: Register },
    LoadBool { dst: Register, value: bool },
    Move { dst: Register, src: Register },

    // Binary ops (24 variants)
    Add { dst: Register, left: Register, right: Register },
    Sub { dst: Register, left: Register, right: Register },
    // ... Mul, Div, Mod, Exp, Eq, StrictEq, Lt, LtEq, Gt, GtEq,
    // ... BitAnd, BitOr, BitXor, LShift, RShift, URShift, In, Instanceof

    // Unary ops
    Neg { dst: Register, src: Register },
    Not { dst: Register, src: Register },
    BitNot { dst: Register, src: Register },
    Typeof { dst: Register, src: Register },

    // Control flow
    Jump { target: JumpTarget },
    JumpIfTrue { cond: Register, target: JumpTarget },
    JumpIfFalse { cond: Register, target: JumpTarget },
    JumpIfNullish { cond: Register, target: JumpTarget },

    // Variables
    GetVar { dst: Register, name: ConstantIndex },
    SetVar { name: ConstantIndex, src: Register },
    DeclareVar { name: ConstantIndex, init: Register, mutable: bool },

    // Objects/Arrays
    CreateObject { dst: Register },
    CreateArray { dst: Register, start: Register, count: u8 },
    GetProperty { dst: Register, obj: Register, key: Register },
    GetPropertyConst { dst: Register, obj: Register, key: ConstantIndex },
    SetProperty { obj: Register, key: Register, value: Register },
    SetPropertyConst { obj: Register, key: ConstantIndex, value: Register },

    // Functions
    Call { dst: Register, callee: Register, this: Register, args_start: Register, argc: u8 },
    Construct { dst: Register, callee: Register, args_start: Register, argc: u8 },
    Return { value: Register },
    CreateClosure { dst: Register, chunk_idx: ConstantIndex },

    // Exception handling
    Throw { value: Register },
    PushTry { catch_target: JumpTarget, finally_target: JumpTarget },
    PopTry,

    // Async/Generator (suspension points)
    Await { dst: Register, promise: Register },
    Yield { dst: Register, value: Register },
    YieldStar { dst: Register, iterable: Register },

    // Scope
    PushScope,
    PopScope,

    // Control
    Nop,
    Halt,
    Debugger,
}
```

### 1.3 Define BytecodeChunk structure

```rust
pub struct BytecodeChunk {
    pub code: Vec<Op>,
    pub constants: Vec<Constant>,
    pub source_map: Vec<(usize, Span)>,  // instruction index -> source location
    pub register_count: u8,
    pub function_info: Option<FunctionInfo>,
}

pub enum Constant {
    String(JsString),
    Number(f64),
    Chunk(Box<BytecodeChunk>),  // Nested function bytecode
}

pub struct FunctionInfo {
    pub name: Option<JsString>,
    pub param_count: usize,
    pub is_generator: bool,
    pub is_async: bool,
}
```

---

## Phase 2: Bytecode Compiler

### 2.1 Implement BytecodeBuilder helper

**File**: `src/compiler/builder.rs`

```rust
pub struct BytecodeBuilder {
    code: Vec<Op>,
    constants: Vec<Constant>,
    source_map: Vec<(usize, Span)>,
    registers: RegisterAllocator,
}

impl BytecodeBuilder {
    pub fn emit(&mut self, op: Op, span: Span) -> usize;
    pub fn emit_jump(&mut self, span: Span) -> JumpPlaceholder;
    pub fn patch_jump(&mut self, placeholder: JumpPlaceholder);
    pub fn add_constant(&mut self, c: Constant) -> ConstantIndex;
    pub fn alloc_register(&mut self) -> Register;
    pub fn free_register(&mut self, r: Register);
}
```

### 2.2 Compile expressions

**File**: `src/compiler/compile_expr.rs`

Compilation order (by priority):

1. **Literals** - `LoadConst`, `LoadBool`, `LoadNull`, `LoadUndefined`
2. **Identifiers** - `GetVar`
3. **Binary operators** - Compile left, compile right, emit `Add`/`Sub`/etc
4. **Logical operators** - Short-circuit with `JumpIfTrue`/`JumpIfFalse`
5. **Unary operators** - `Neg`, `Not`, `BitNot`, `Typeof`
6. **Member access** - `GetProperty`, `GetPropertyConst`
7. **Function calls** - `Call`, `Construct`
8. **Object/Array literals** - `CreateObject`/`CreateArray` + `SetProperty`
9. **Assignment** - `SetVar`, `SetProperty`
10. **Conditional (ternary)** - Jumps with condition
11. **Arrow/Function expressions** - `CreateClosure` with nested chunk

### 2.3 Compile statements

**File**: `src/compiler/compile_stmt.rs`

1. **Expression statements** - Compile expression, discard result
2. **Variable declarations** - `DeclareVar` with initializer
3. **If/else** - `JumpIfFalse` to else branch
4. **While/DoWhile** - Loop with `Jump` back
5. **For loops** - Init, test with `JumpIfFalse`, body, update, `Jump` back
6. **Return** - `Return` opcode
7. **Throw** - `Throw` opcode
8. **Try/catch/finally** - `PushTry`/`PopTry` with jump targets
9. **Function declarations** - `CreateClosure` + `DeclareVar`
10. **Class declarations** - Complex: constructor, methods, prototype setup

### 2.4 Compile patterns (destructuring)

**File**: `src/compiler/compile_pattern.rs`

Handle `Pattern` variants:
- `Pattern::Identifier` - Direct `DeclareVar`/`SetVar`
- `Pattern::Object` - `GetProperty` for each key, recurse
- `Pattern::Array` - `GetProperty` with index, recurse
- `Pattern::Rest` - Slice remaining elements
- `Pattern::Assignment` - Default value with `JumpIfNullish`

---

## Phase 3: Bytecode Virtual Machine

### 3.1 Define VM state

**File**: `src/interpreter/bytecode_vm.rs`

```rust
pub struct BytecodeVM {
    ip: usize,                           // Instruction pointer
    chunk: Rc<BytecodeChunk>,            // Current bytecode
    registers: Vec<Guarded>,             // Register file (GC-safe)
    call_stack: Vec<CallFrame>,          // Return addresses
    exception_handlers: Vec<TryHandler>, // Try/catch stack
}

struct CallFrame {
    return_ip: usize,
    return_chunk: Rc<BytecodeChunk>,
    registers_base: usize,
    return_register: Register,
}

struct TryHandler {
    catch_ip: usize,
    finally_ip: usize,
    registers_snapshot: usize,
}
```

### 3.2 Implement execution loop

Integrate with existing `Interpreter`:

```rust
impl Interpreter {
    pub fn run_bytecode(&mut self, chunk: Rc<BytecodeChunk>) -> Result<Guarded, JsError> {
        let mut vm = BytecodeVM::new(chunk);
        loop {
            self.check_timeout()?;
            let Some(op) = vm.fetch() else {
                return Ok(vm.get_result());
            };
            match op {
                Op::LoadConst { dst, idx } => { /* ... */ }
                Op::Add { dst, left, right } => {
                    let result = self.add_values(&vm.reg(left), &vm.reg(right))?;
                    vm.set_reg(dst, result);
                }
                Op::Call { dst, callee, this, args_start, argc } => {
                    // Delegate to existing call_function for full semantics
                    let result = self.call_function(callee_val, this_val, &args)?;
                    vm.set_reg(dst, result);
                }
                Op::Await { dst, promise } => {
                    // Save VM state, return suspension
                    return Err(JsError::AwaitSuspend {
                        vm_state: vm.save_state(),
                        promise: vm.reg(promise).clone(),
                    });
                }
                // ... handle all opcodes
            }
        }
    }
}
```

### 3.3 Suspension & resumption

For `await` and `yield`, save VM state similarly to current `SavedGeneratorExecution`:

```rust
pub struct SavedVMState {
    ip: usize,
    chunk: Rc<BytecodeChunk>,
    registers: Vec<Guarded>,
    call_stack: Vec<CallFrame>,
}
```

Resume by restoring state and continuing execution.

---

## Phase 4: Integration & Testing

### 4.1 Add feature flag

**File**: `src/interpreter/mod.rs`

```rust
impl Interpreter {
    pub fn eval(&mut self, source: &str) -> Result<RuntimeResult, JsError> {
        let program = Parser::new(source, &mut self.string_dict).parse_program()?;

        if self.use_bytecode {
            let chunk = Compiler::compile_program(&program, &mut self.string_dict)?;
            self.run_bytecode(chunk)
        } else {
            self.eval_with_stack(source)  // Existing frame-based
        }
    }
}
```

### 4.2 Differential testing

Run both implementations and compare results:

```rust
#[test]
fn test_bytecode_equivalence() {
    for source in TEST_CASES {
        let frame_result = eval_with_frames(source);
        let bytecode_result = eval_with_bytecode(source);
        assert_eq!(frame_result, bytecode_result);
    }
}
```

### 4.3 Run Test262 suite

```bash
USE_BYTECODE=0 ./target/release/test262-runner --strict-only language/
USE_BYTECODE=1 ./target/release/test262-runner --strict-only language/
# Compare pass rates
```

---

## Phase 5: Cleanup

### 5.1 Remove frame-based execution

Once bytecode is stable and passes all tests:
1. Remove `src/interpreter/stack.rs` - Frame-based execution stack
2. Remove `Frame` enum from `src/interpreter/mod.rs`
3. Remove AST interpreter logic from `src/interpreter/mod.rs`:
   - `evaluate_expression()` and all `evaluate_*` methods
   - `execute_statement()` and all `execute_*` methods
   - `step_expr()`, `step_stmt()` and related frame-stepping logic
   - Keep only: builtins, GC, environment management, `call_function` (delegates to bytecode)
4. Remove `use_bytecode` flag - bytecode becomes the only execution path
5. Update documentation

---

## Migration Order (Feature Tiers)

| Tier | Features | Risk |
|------|----------|------|
| 1 | Literals, binary/unary ops, variables, member access | Low |
| 2 | Control flow (if, while, for, switch) | Medium |
| 3 | Functions, closures, calls, constructors | Medium |
| 4 | Try/catch/finally, destructuring | Medium |
| 5 | Generators (yield, yield*) | High |
| 6 | Async/await | High |
| 7 | Classes, modules | High |

---

## Critical Files

| File | Action |
|------|--------|
| `src/compiler/mod.rs` | **NEW** - Compiler entry point |
| `src/compiler/bytecode.rs` | **NEW** - Op enum, BytecodeChunk |
| `src/compiler/builder.rs` | **NEW** - BytecodeBuilder |
| `src/compiler/compile_expr.rs` | **NEW** - Expression compilation |
| `src/compiler/compile_stmt.rs` | **NEW** - Statement compilation |
| `src/compiler/compile_pattern.rs` | **NEW** - Pattern compilation |
| `src/interpreter/bytecode_vm.rs` | **NEW** - VM execution loop |
| `src/interpreter/mod.rs` | **MODIFY then CLEANUP** - Add bytecode flag, then remove AST interpreter |
| `src/lib.rs` | **MODIFY** - Add compile API |
| `src/ast.rs` | READ-ONLY - Compilation source |
| `src/interpreter/stack.rs` | **DELETE (Phase 5)** - Frame-based execution |
| `src/value.rs` | READ-ONLY - Runtime values (reused) |
| `src/gc.rs` | READ-ONLY - GC system (reused) |

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| GC bugs in VM | Use `Guarded` for all registers; test with `GC_THRESHOLD=1` |
| Suspension complexity | Model after existing `SavedGeneratorExecution` pattern |
| Performance regression | Keep both implementations until bytecode matches or exceeds |
| Semantic differences | Differential testing + Test262 suite comparison |
| Register overflow | Track max registers during compilation; error if >255 |
