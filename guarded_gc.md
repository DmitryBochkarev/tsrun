# GuardedGc: Protecting Temporary Objects from GC Collection

## Problem

When GC threshold is set to 1 (collecting on every allocation), temporary objects allocated during Rust code execution can be collected before they're stored in a rooted structure. This causes tests to fail with corrupted or missing data.

Example of the problem:
```rust
// BROKEN: obj may be collected during evaluate() calls
let obj = self.create_object();  // Allocates, may trigger GC
let value = self.evaluate(&expr)?;  // Allocates, triggers GC - obj gets unlinked!
obj.borrow_mut().set_property(key, value);  // obj is now empty/corrupted
```

## Solution: GuardedGc

`GuardedGc<T>` is a wrapper that temporarily protects a GC object from collection. It registers itself in the space's guards collection (treated as roots during mark phase) and automatically removes itself when dropped.

## API

### Allocating with Protection

```rust
// Allocate new object with protection
let guarded = self.gc_space.alloc_guarded(MyObject::new());

// Or use interpreter helpers
let guarded_obj = self.create_object_guarded();
let guarded_arr = self.create_array_guarded(elements);
```

### Creating Guard for Existing Object

```rust
let gc = self.create_object();  // Unprotected
let guarded = self.gc_space.guard(&gc);  // Now protected
```

### Accessing the Value

```rust
// Via Deref (GuardedGc implements Deref<Target = Gc<T>>)
guarded.borrow().some_field;
guarded.borrow_mut().some_field = value;

// Explicit access
guarded.as_gc().borrow();
```

### Extracting the Inner Gc

```rust
// Option 1: take() - removes protection, returns Gc
let gc: Gc<T> = guarded.take();

// Option 2: From conversion - same as take()
let gc: Gc<T> = guarded.into();

// Option 3: Clone inner Gc (guard still active)
let gc = guarded.as_gc().clone();
drop(guarded);  // Protection removed here
```

### Drop Behavior

When `GuardedGc` is dropped:
1. It removes itself from the space's guards list
2. The object becomes eligible for GC (if not otherwise rooted)

```rust
{
    let guarded = space.alloc_guarded(obj);
    // Protected here
}  // Guard dropped, object can now be collected
```

## Patterns to Fix

### Pattern 1: Object/Array Construction with Property Evaluation

**Before (broken with threshold=1):**
```rust
let result = self.create_object();
self.gc_space.add_root(&result);

for prop in &properties {
    let value = self.evaluate(&prop.value)?;  // May trigger GC
    result.borrow_mut().set_property(key, value);
}

self.gc_space.remove_root(&result);
Ok(JsValue::Object(result))
```

**After (using GuardedGc):**
```rust
let result = self.create_object_guarded();

for prop in &properties {
    let value = self.evaluate(&prop.value)?;  // GC won't collect result
    result.borrow_mut().set_property(key, value);
}

Ok(JsValue::Object(result.take()))
```

### Pattern 2: Creating Multiple Related Objects

**Before (broken):**
```rust
let parent = self.create_object();
let child = self.create_object();  // GC may collect parent here!
parent.borrow_mut().set_property(key, JsValue::Object(child));
```

**After:**
```rust
let parent = self.create_object_guarded();
let child = self.create_object();  // parent is protected
parent.borrow_mut().set_property(key, JsValue::Object(child));
let parent_gc = parent.take();
```

### Pattern 3: Array with Evaluated Elements

**Before (broken):**
```rust
let arr = self.create_array(vec![]);
self.gc_space.add_root(&arr);

for expr in expressions {
    let elem = self.evaluate(expr)?;
    // push to arr
}

self.gc_space.remove_root(&arr);
```

**After:**
```rust
let arr = self.create_array_guarded(vec![]);

for expr in expressions {
    let elem = self.evaluate(expr)?;
    // push to arr
}

drop(arr);  // or arr.take() if you need the Gc
```

### Pattern 4: Constructor Call with New Object

**Before (broken):**
```rust
let new_obj = self.create_object();
self.gc_space.add_root(&new_obj);

// Set prototype, initialize fields...
let result = self.call_function(ctor, JsValue::Object(new_obj.clone()), &args)?;

self.gc_space.remove_root(&new_obj);
```

**After:**
```rust
let new_obj = self.create_object_guarded();

// Set prototype, initialize fields...
let result = self.call_function(ctor, JsValue::Object(new_obj.as_gc().clone()), &args)?;

let new_obj_gc = new_obj.take();
```

### Pattern 5: Builtin Function Creating Result Object

**Before (broken):**
```rust
pub fn array_map(interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let result = interp.create_array(vec![]);
    // result may be collected during callback calls!

    for (i, elem) in elements.iter().enumerate() {
        let mapped = interp.call_function(callback, this_arg, vec![elem.clone(), ...])?;
        // result might be corrupted here
        result.borrow_mut().set_property(PropertyKey::Index(i as u32), mapped);
    }

    Ok(JsValue::Object(result))
}
```

**After:**
```rust
pub fn array_map(interp: &mut Interpreter, this: JsValue, args: Vec<JsValue>) -> Result<JsValue, JsError> {
    let result = interp.create_array_guarded(vec![]);

    for (i, elem) in elements.iter().enumerate() {
        let mapped = interp.call_function(callback, this_arg, vec![elem.clone(), ...])?;
        result.borrow_mut().set_property(PropertyKey::Index(i as u32), mapped);
    }

    Ok(JsValue::Object(result.take()))
}
```

### Pattern 6: Multiple Guards for Same Object

When you need to protect an object that's already been created:

```rust
let gc = some_existing_gc.clone();
let guard = self.gc_space.guard(&gc);

// Do allocating work...
let value = self.evaluate(expr)?;

// gc is protected by guard
drop(guard);  // Now gc can be collected if unreachable
```

## Systematic Fix Process

When enabling `threshold=1` and tests fail:

### Step 1: Identify the Failure Point

Run with `RUST_BACKTRACE=1` and look for:
- `NaN` results (numeric properties from unlinked objects)
- Empty arrays/objects
- Missing properties
- Panics in `borrow()` or `borrow_mut()`

### Step 2: Find the Allocation Pattern

Look for code between allocation and storage:
```rust
let obj = create_*();     // <-- Allocation
// ... any code that may allocate ...
something.set(obj);       // <-- Storage
```

The "any code that may allocate" includes:
- `evaluate()` calls
- `call_function()` calls
- Other `create_*()` calls
- Builtin methods that allocate (most of them)

### Step 3: Apply the Fix

1. Change `create_*()` to `create_*_guarded()` or wrap with `guard()`
2. Use `take()` or `into()` when storing/returning the result
3. Or just let the guard drop if the object is already stored elsewhere

### Step 4: Verify

Run the specific test with threshold=1:
```bash
cargo test test_name -- --nocapture
```

## Files Most Likely to Need Fixes

Based on allocation patterns, check these in order:

1. **`interpreter/mod.rs`** - Expression evaluation, statement execution
   - `evaluate()` method - object/array literals
   - `call_function()` - function environments
   - `call_constructor_internal()` - new objects
   - Loop statements - iteration environments

2. **`interpreter/builtins/array.rs`** - Array methods that create new arrays
   - `array_map`, `array_filter`, `array_slice`, `array_concat`
   - `array_flat`, `array_flat_map`
   - `array_from`, `array_of`

3. **`interpreter/builtins/object.rs`** - Object methods
   - `object_assign`, `object_create`
   - `object_keys`, `object_values`, `object_entries`
   - `object_from_entries`

4. **`interpreter/builtins/string.rs`** - String methods returning arrays
   - `string_split`, `string_match`

5. **`interpreter/builtins/json.rs`** - JSON parsing
   - `json_parse` - creates nested objects/arrays

6. **`interpreter/builtins/regexp.rs`** - RegExp methods
   - `regexp_exec` - creates result array

7. **`interpreter/builtins/map.rs`** and **`set.rs`**
   - Iterator result objects

8. **`interpreter/builtins/promise.rs`**
   - Promise result handling

## Quick Reference

| Operation | Unprotected | Protected |
|-----------|-------------|-----------|
| Create object | `create_object()` | `create_object_guarded()` |
| Create array | `create_array(elems)` | `create_array_guarded(elems)` |
| Protect existing | - | `gc_space.guard(&gc)` |
| Get inner Gc | - | `guarded.take()` or `guarded.into()` |
| Access (keep guard) | - | `guarded.as_gc().clone()` |
| Borrow | `gc.borrow()` | `guarded.borrow()` |
| Borrow mut | `gc.borrow_mut()` | `guarded.borrow_mut()` |

## Testing Strategy

1. First, run all tests with default threshold to ensure no regressions
2. Then enable threshold=1 and run tests one module at a time:
   ```bash
   # Set threshold in test setup or use a helper
   runtime.set_gc_threshold(1);
   ```
3. Fix failures systematically using the patterns above
4. Re-run full test suite with both default and threshold=1

## Notes

- Guards are cheap (just a HashMap entry with Rc clone)
- Multiple guards can protect the same object
- Guards don't prevent the object from being modified
- Guards only prevent GC from unlinking the object's properties
- Permanent roots (prototypes, global) should NOT use GuardedGc - keep using `add_root()`
