# Order System Refactor: Concurrent Async Execution

## Executive Summary

This document describes a refactor to the order system that:
1. Allows `__order__()` to return **any value** (including unresolved Promises)
2. Supports **multiple concurrent async contexts** waiting on different Promises
3. Properly resumes execution when Promises are resolved

---

## 1. Motivation: Concurrent Async Operations

### 1.1 Desired Behavior

```javascript
// fetch() internally calls __order__() which suspends,
// host returns an UNRESOLVED Promise, VM continues with that Promise

let req1 = fetch("https://api.example.com/users");   // req1 = Promise (unresolved)
let req2 = fetch("https://api.example.com/posts");   // req2 = Promise (unresolved)
let req3 = fetch("https://api.example.com/comments"); // req3 = Promise (unresolved)

async function wait_1_2() {
    let responses = await Promise.all([req1, req2]);  // Suspends this context
    console.log("Got users and posts:", responses);
}

async function wait_3() {
    let response = await req3;  // Suspends this context
    console.log("Got comments:", response);
}

wait_1_2();  // Starts async function, suspends at await
wait_3();    // Starts async function, suspends at await

// VM returns to host with:
// - 3 pending orders (for the fetches)
// - 2 suspended contexts (wait_1_2 and wait_3)
//
// Host can resolve promises in ANY order
// Runtime resumes the appropriate contexts when their dependencies resolve
```

### 1.2 Current Limitation

The current implementation only supports **one suspended VM state**. When any `await` hits a pending promise, the entire interpreter suspends.

### 1.3 Goals

1. `__order__()` suspends immediately, host provides value, VM resumes
2. If host returns unresolved Promise → store it, continue executing
3. Multiple async contexts can wait on different Promises
4. When a Promise resolves → resume contexts waiting on it
5. Host can resolve Promises via `fulfill_orders` or direct `resolve_promise` API

---

## 2. Two Types of Suspension

| Type | Trigger | Behavior |
|------|---------|----------|
| **Order Suspension** | `__order__()` called | VM stops immediately, returns to host, host provides value, VM resumes with that value |
| **Await Suspension** | `await pendingPromise` | This async context pauses, other contexts/code can continue, resumes when Promise resolves |

### 2.1 Order Suspension Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         ORDER SUSPENSION FLOW                               │
└─────────────────────────────────────────────────────────────────────────────┘

JavaScript                         Interpreter                          Host
    │                                  │                                  │
    │  let p = fetch(url)              │                                  │
    │  // fetch calls __order__()      │                                  │
    │─────────────────────────────────>│                                  │
    │                                  │                                  │
    │                     ┌────────────┴────────────┐                     │
    │                     │ order_syscall():        │                     │
    │                     │ 1. Generate OrderId     │                     │
    │                     │ 2. Create PendingOrder  │                     │
    │                     │ 3. SUSPEND IMMEDIATELY  │                     │
    │                     └────────────┬────────────┘                     │
    │                                  │                                  │
    │                                  │  Suspended { orders: [O1] }      │
    │                                  │─────────────────────────────────>│
    │                                  │                                  │
    │                                  │         ┌────────────────────────┴─┐
    │                                  │         │ Host decides response:   │
    │                                  │         │ - Concrete value, OR     │
    │                                  │         │ - Unresolved Promise     │
    │                                  │         └────────────────────────┬─┘
    │                                  │                                  │
    │                                  │  fulfill_orders([{               │
    │                                  │    id: O1,                       │
    │                                  │    result: Promise (unresolved)  │
    │                                  │  }])                             │
    │                                  │<─────────────────────────────────│
    │                                  │                                  │
    │                     ┌────────────┴────────────┐                     │
    │                     │ continue_eval():        │                     │
    │                     │ Inject Promise into     │                     │
    │                     │ resume register         │                     │
    │                     │ VM CONTINUES            │                     │
    │                     └────────────┬────────────┘                     │
    │                                  │                                  │
    │<─────────────────────────────────│                                  │
    │  p = Promise (unresolved)        │                                  │
    │  // execution continues...       │                                  │
```

### 2.2 Await Suspension Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         AWAIT SUSPENSION FLOW                               │
└─────────────────────────────────────────────────────────────────────────────┘

JavaScript                         Interpreter                          Host
    │                                  │                                  │
    │  // Inside async function:       │                                  │
    │  let result = await p;           │                                  │
    │  // p is pending Promise         │                                  │
    │─────────────────────────────────>│                                  │
    │                                  │                                  │
    │                     ┌────────────┴────────────┐                     │
    │                     │ Op::Await:              │                     │
    │                     │ Promise is PENDING      │                     │
    │                     │ 1. Save async context   │                     │
    │                     │ 2. Add to WaitGraph     │                     │
    │                     │ 3. Return from async fn │                     │
    │                     └────────────┬────────────┘                     │
    │                                  │                                  │
    │  // Caller continues executing   │                                  │
    │  // or if no more code...        │                                  │
    │                                  │                                  │
    │                                  │  Suspended {                     │
    │                                  │    contexts: [ctx1],             │
    │                                  │    waiting_on: [P]               │
    │                                  │  }                               │
    │                                  │─────────────────────────────────>│
    │                                  │                                  │
    │                                  │         ┌────────────────────────┴─┐
    │                                  │         │ Host resolves Promise P  │
    │                                  │         │ (e.g., network response) │
    │                                  │         └────────────────────────┬─┘
    │                                  │                                  │
    │                                  │  resolve_promise(P, value)       │
    │                                  │<─────────────────────────────────│
    │                                  │                                  │
    │                     ┌────────────┴────────────┐                     │
    │                     │ continue_eval():        │                     │
    │                     │ 1. P is now fulfilled   │                     │
    │                     │ 2. Find ctx1 in graph   │                     │
    │                     │ 3. Resume ctx1          │                     │
    │                     └────────────┬────────────┘                     │
    │                                  │                                  │
    │  result = resolved value         │                                  │
    │  // async function continues     │                                  │
```

---

## 3. Data Structures

### 3.1 Context and Wait Graph

```rust
/// Unique identifier for a suspended async context
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContextId(pub u64);

/// Unique identifier for tracking Promises in the wait graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PromiseId(pub u64);

/// A suspended async execution context
#[derive(Debug)]
pub struct SuspendedContext {
    /// Unique identifier for this context
    pub id: ContextId,
    /// Saved VM state (registers, call frames, etc.)
    pub state: SavedVmState,
    /// The Promise this context is waiting on
    pub waiting_on: Gc<JsObject>,
    /// Promise ID for quick lookup
    pub waiting_on_id: PromiseId,
    /// Register to store the resolved value when resumed
    pub resume_register: Register,
}

/// Tracks all suspended contexts and their Promise dependencies
#[derive(Debug, Default)]
pub struct WaitGraph {
    /// All suspended contexts, indexed by ContextId
    pub contexts: FxHashMap<ContextId, SuspendedContext>,

    /// Promise → contexts waiting on it
    /// When a Promise resolves, we look up all waiters here
    pub promise_waiters: FxHashMap<PromiseId, Vec<ContextId>>,

    /// Contexts ready to resume (their Promise has resolved)
    pub ready_queue: VecDeque<ContextId>,
}

impl WaitGraph {
    /// Add a suspended context to the graph
    pub fn add_context(&mut self, ctx: SuspendedContext) {
        let ctx_id = ctx.id;
        let promise_id = ctx.waiting_on_id;

        self.promise_waiters
            .entry(promise_id)
            .or_default()
            .push(ctx_id);

        self.contexts.insert(ctx_id, ctx);
    }

    /// Called when a Promise is resolved - moves waiters to ready queue
    pub fn promise_resolved(&mut self, promise_id: PromiseId) {
        if let Some(waiter_ids) = self.promise_waiters.remove(&promise_id) {
            for ctx_id in waiter_ids {
                self.ready_queue.push_back(ctx_id);
            }
        }
    }

    /// Take the next ready context for execution
    pub fn take_ready(&mut self) -> Option<SuspendedContext> {
        while let Some(ctx_id) = self.ready_queue.pop_front() {
            if let Some(ctx) = self.contexts.remove(&ctx_id) {
                return Some(ctx);
            }
            // Context was cancelled/removed, skip it
        }
        None
    }

    /// Check if any contexts are waiting
    pub fn has_waiting_contexts(&self) -> bool {
        !self.contexts.is_empty()
    }

    /// Check if any contexts are ready to resume
    pub fn has_ready_contexts(&self) -> bool {
        !self.ready_queue.is_empty()
    }
}
```

### 3.2 Order Suspension

```rust
/// Order suspension - VM is blocked waiting for host to provide a value
pub struct OrderSuspension {
    /// The order ID we're waiting for
    pub order_id: OrderId,
    /// Saved VM state for resumption
    pub state: SavedVmState,
    /// Register to store the response value
    pub resume_register: Register,
}
```

### 3.3 Updated Interpreter Fields

```rust
pub struct Interpreter {
    // ... existing fields ...

    // ═══════════════════════════════════════════════════════════════════
    // Order System
    // ═══════════════════════════════════════════════════════════════════

    /// Counter for generating unique order IDs
    pub(crate) next_order_id: u64,

    /// Pending orders waiting for host fulfillment
    pub(crate) pending_orders: Vec<Order>,

    /// Order responses from host (consumed on resume)
    pub(crate) order_responses: FxHashMap<OrderId, Result<RuntimeValue, JsError>>,

    /// Current order suspension (only one at a time - VM is blocked)
    pub(crate) suspended_for_order: Option<OrderSuspension>,

    /// Cancelled order IDs
    pub(crate) cancelled_orders: Vec<OrderId>,

    // ═══════════════════════════════════════════════════════════════════
    // Async Context Management
    // ═══════════════════════════════════════════════════════════════════

    /// Counter for generating unique context IDs
    pub(crate) next_context_id: u64,

    /// Counter for generating unique promise IDs (for wait graph tracking)
    pub(crate) next_promise_id: u64,

    /// Graph of suspended async contexts and their Promise dependencies
    pub(crate) wait_graph: WaitGraph,

    /// Map from Promise object to PromiseId (for quick lookup)
    pub(crate) promise_ids: FxHashMap<GcId, PromiseId>,

    // ═══════════════════════════════════════════════════════════════════
    // REMOVED
    // ═══════════════════════════════════════════════════════════════════

    // DELETE: order_callbacks - no longer needed
    // DELETE: suspended_vm_state - replaced by wait_graph
}
```

### 3.4 New ExoticObject Variant

```rust
pub enum ExoticObject {
    // ... existing variants ...

    /// Pending order marker - triggers immediate suspension when awaited
    PendingOrder { id: u64 },
}
```

---

## 4. VM Changes

### 4.1 New OpResult Variants

```rust
enum OpResult {
    // ... existing variants ...

    /// Suspend for order fulfillment (immediate, blocks entire VM)
    SuspendForOrder {
        order_id: OrderId,
        resume_register: Register,
    },

    /// Suspend this async context (other code can continue)
    SuspendAsyncContext {
        waiting_on: Gc<JsObject>,
        resume_register: Register,
    },
}
```

### 4.2 New VmResult Variants

```rust
pub enum VmResult {
    // ... existing variants ...

    /// Suspended waiting for order fulfillment
    SuspendForOrder(OrderSuspension),

    /// Async context suspended, waiting on Promise
    SuspendAsyncContext {
        context_id: ContextId,
        waiting_on: Gc<JsObject>,
        state: SavedVmState,
        resume_register: Register,
    },
}
```

### 4.3 Updated Op::Await Handler

```rust
Op::Await { dst, promise } => {
    use crate::value::{ExoticObject, PromiseStatus};

    let promise_val = self.get_reg(promise);

    if let JsValue::Object(obj) = &promise_val {
        let obj_ref = obj.borrow();

        // Check for pending order marker → immediate suspension
        if let ExoticObject::PendingOrder { id } = &obj_ref.exotic {
            drop(obj_ref);
            return Ok(OpResult::SuspendForOrder {
                order_id: OrderId(*id),
                resume_register: dst,
            });
        }

        // Check for Promise
        if let ExoticObject::Promise(state) = &obj_ref.exotic {
            let state_ref = state.borrow();
            match state_ref.status {
                PromiseStatus::Fulfilled => {
                    // Use resolved value immediately
                    let result = state_ref.result.clone().unwrap_or(JsValue::Undefined);
                    drop(state_ref);
                    drop(obj_ref);
                    self.set_reg(dst, result);
                    return Ok(OpResult::Continue);
                }
                PromiseStatus::Rejected => {
                    // Throw the rejection reason
                    let reason = state_ref.result.clone().unwrap_or(JsValue::Undefined);
                    drop(state_ref);
                    drop(obj_ref);
                    return Err(JsError::thrown(Guarded::from_value(reason, &interp.heap)));
                }
                PromiseStatus::Pending => {
                    // Suspend this async context
                    drop(state_ref);
                    drop(obj_ref);
                    return Ok(OpResult::SuspendAsyncContext {
                        waiting_on: obj.clone(),
                        resume_register: dst,
                    });
                }
            }
        }
    }

    // Not a promise or pending order - use value directly (await 42 === 42)
    self.set_reg(dst, promise_val.clone());
    Ok(OpResult::Continue)
}
```

### 4.4 Updated run() Loop

```rust
// In the main run loop, handle new OpResult variants:

Ok(OpResult::SuspendForOrder { order_id, resume_register }) => {
    return VmResult::SuspendForOrder(OrderSuspension {
        order_id,
        state: self.save_state(interp),
        resume_register,
    });
}

Ok(OpResult::SuspendAsyncContext { waiting_on, resume_register }) => {
    let context_id = ContextId(interp.next_context_id);
    interp.next_context_id += 1;

    return VmResult::SuspendAsyncContext {
        context_id,
        waiting_on,
        state: self.save_state(interp),
        resume_register,
    };
}
```

---

## 5. Interpreter Changes

### 5.1 Simplified order_syscall

```rust
fn order_syscall(
    interp: &mut Interpreter,
    _this: JsValue,
    args: &[JsValue],
) -> Result<Guarded, JsError> {
    let payload = args.first().cloned().unwrap_or(JsValue::Undefined);

    // Generate unique order ID
    let id = OrderId(interp.next_order_id);
    interp.next_order_id += 1;

    // Create payload RuntimeValue with guard if object
    let payload_rv = if let JsValue::Object(ref obj) = payload {
        let guard = interp.heap.create_guard();
        guard.guard(obj.clone());
        RuntimeValue::with_guard(payload, guard)
    } else {
        RuntimeValue::unguarded(payload)
    };

    // Record the pending order
    interp.pending_orders.push(Order { id, payload: payload_rv });

    // Create and return PendingOrder marker
    // When this is awaited, it triggers SuspendForOrder
    let marker_guard = interp.heap.create_guard();
    let marker = marker_guard.alloc();
    marker.borrow_mut().exotic = ExoticObject::PendingOrder { id: id.0 };

    Ok(Guarded::with_guard(JsValue::Object(marker), marker_guard))
}
```

### 5.2 Updated fulfill_orders

```rust
pub fn fulfill_orders(&mut self, responses: Vec<OrderResponse>) -> Result<(), JsError> {
    for response in responses {
        self.order_responses.insert(response.id, response.result);
    }
    Ok(())
}
```

### 5.3 New resolve_promise API

```rust
/// Resolve a Promise and wake any waiting contexts
///
/// This can be called by the host to resolve Promises that were
/// returned from orders (e.g., when a network request completes).
pub fn resolve_promise(
    &mut self,
    promise: Gc<JsObject>,
    value: JsValue,
) -> Result<(), JsError> {
    // 1. Update the Promise state
    {
        let mut obj_ref = promise.borrow_mut();
        if let ExoticObject::Promise(state) = &obj_ref.exotic {
            let mut state_ref = state.borrow_mut();
            if state_ref.status != PromiseStatus::Pending {
                return Ok(()); // Already settled
            }
            state_ref.status = PromiseStatus::Fulfilled;
            state_ref.result = Some(value.clone());
        } else {
            return Err(JsError::type_error("Not a promise"));
        }
    }

    // 2. Find waiting contexts and mark them ready
    let gc_id = promise.id(); // Get GC object ID
    if let Some(promise_id) = self.promise_ids.get(&gc_id) {
        self.wait_graph.promise_resolved(*promise_id);
    }

    // 3. Call any .then() handlers (existing mechanism)
    self.run_promise_handlers(promise)?;

    Ok(())
}

/// Reject a Promise and wake any waiting contexts
pub fn reject_promise(
    &mut self,
    promise: Gc<JsObject>,
    reason: JsValue,
) -> Result<(), JsError> {
    // Similar to resolve_promise but sets PromiseStatus::Rejected
    // ...
}
```

### 5.4 Updated run_vm_to_completion

```rust
fn run_vm_to_completion(
    &mut self,
    mut vm: BytecodeVM,
) -> Result<RuntimeResult, JsError> {
    loop {
        let result = vm.run(self);

        match result {
            VmResult::Complete(guarded) => {
                // Check if there's more work to do
                return self.check_completion_or_continue(guarded);
            }

            VmResult::SuspendForOrder(order_suspension) => {
                // Immediate suspension - return to host
                self.suspended_for_order = Some(order_suspension);
                return self.return_suspended();
            }

            VmResult::SuspendAsyncContext { context_id, waiting_on, state, resume_register } => {
                // Register this context in the wait graph
                let promise_id = self.get_or_create_promise_id(&waiting_on);

                self.wait_graph.add_context(SuspendedContext {
                    id: context_id,
                    state,
                    waiting_on,
                    waiting_on_id: promise_id,
                    resume_register,
                });

                // Try to continue with other ready contexts
                if let Some(ready_ctx) = self.wait_graph.take_ready() {
                    vm = self.restore_context(ready_ctx)?;
                    continue;
                }

                // No more work - return to host
                return self.return_suspended();
            }

            VmResult::Error(err) => {
                return Err(self.materialize_thrown_error(err));
            }

            // ... handle Yield, YieldStar ...
        }
    }
}

fn return_suspended(&mut self) -> Result<RuntimeResult, JsError> {
    let pending = std::mem::take(&mut self.pending_orders);
    let cancelled = std::mem::take(&mut self.cancelled_orders);
    Ok(RuntimeResult::Suspended { pending, cancelled })
}
```

### 5.5 Updated continue_eval

```rust
pub fn continue_eval(&mut self) -> Result<RuntimeResult, JsError> {
    loop {
        // ═══════════════════════════════════════════════════════════════
        // 1. Resume from order suspension if we have a response
        // ═══════════════════════════════════════════════════════════════
        if let Some(order_susp) = self.suspended_for_order.take() {
            if let Some(result) = self.order_responses.remove(&order_susp.order_id) {
                match result {
                    Ok(runtime_value) => {
                        let value = runtime_value.value().clone();
                        let vm = self.restore_order_suspension(order_susp, value)?;
                        return self.run_vm_to_completion(vm);
                    }
                    Err(error) => {
                        return self.inject_order_error(order_susp, error);
                    }
                }
            } else {
                // Order not yet fulfilled - re-suspend
                self.suspended_for_order = Some(order_susp);
                return self.return_suspended();
            }
        }

        // ═══════════════════════════════════════════════════════════════
        // 2. Resume any ready async contexts
        // ═══════════════════════════════════════════════════════════════
        if let Some(ready_ctx) = self.wait_graph.take_ready() {
            // Get the resolved value from the Promise
            let resolved_value = self.get_promise_result(&ready_ctx.waiting_on)?;

            let vm = self.restore_context_with_value(ready_ctx, resolved_value)?;
            return self.run_vm_to_completion(vm);
        }

        // ═══════════════════════════════════════════════════════════════
        // 3. No work to do - check completion status
        // ═══════════════════════════════════════════════════════════════
        if self.wait_graph.has_waiting_contexts() {
            // Contexts are waiting for Promises to resolve
            return self.return_suspended();
        }

        // All done
        return Ok(RuntimeResult::Complete(RuntimeValue::unguarded(JsValue::Undefined)));
    }
}

fn get_promise_result(&self, promise: &Gc<JsObject>) -> Result<JsValue, JsError> {
    let obj_ref = promise.borrow();
    if let ExoticObject::Promise(state) = &obj_ref.exotic {
        let state_ref = state.borrow();
        match state_ref.status {
            PromiseStatus::Fulfilled => {
                Ok(state_ref.result.clone().unwrap_or(JsValue::Undefined))
            }
            PromiseStatus::Rejected => {
                let reason = state_ref.result.clone().unwrap_or(JsValue::Undefined);
                Err(JsError::thrown(Guarded::unguarded(reason)))
            }
            PromiseStatus::Pending => {
                Err(JsError::internal_error("Promise still pending"))
            }
        }
    } else {
        Err(JsError::internal_error("Not a promise"))
    }
}
```

---

## 6. Public API

### 6.1 Runtime Methods

```rust
impl Runtime {
    /// Fulfill orders with responses from the host
    ///
    /// Order responses can be:
    /// - Concrete values (immediately usable)
    /// - Unresolved Promises (will be awaited when the code reaches them)
    pub fn fulfill_orders(
        &mut self,
        responses: Vec<OrderResponse>,
    ) -> Result<RuntimeResult, JsError> {
        self.interpreter.fulfill_orders(responses)?;
        self.continue_eval()
    }

    /// Resolve a Promise that was returned from an order
    ///
    /// Use this when an async operation completes (e.g., network request finished).
    /// Any async contexts waiting on this Promise will be woken up.
    pub fn resolve_promise(
        &mut self,
        promise: Gc<JsObject>,
        value: RuntimeValue,
    ) -> Result<RuntimeResult, JsError> {
        self.interpreter.resolve_promise(promise, value.value().clone())?;
        self.continue_eval()
    }

    /// Reject a Promise that was returned from an order
    pub fn reject_promise(
        &mut self,
        promise: Gc<JsObject>,
        reason: RuntimeValue,
    ) -> Result<RuntimeResult, JsError> {
        self.interpreter.reject_promise(promise, reason.value().clone())?;
        self.continue_eval()
    }

    /// Create an unresolved Promise that can be returned from fulfill_orders
    ///
    /// Later, call resolve_promise or reject_promise to settle it.
    pub fn create_promise(&mut self) -> (Gc<JsObject>, Guard<JsObject>) {
        let guard = self.interpreter.heap.create_guard();
        let promise = create_promise(&mut self.interpreter, &guard);
        (promise, guard)
    }
}
```

### 6.2 Updated RuntimeResult

```rust
pub enum RuntimeResult {
    /// Execution completed with a final value
    Complete(RuntimeValue),

    /// Need these modules before execution can continue
    NeedImports(Vec<ImportRequest>),

    /// Execution suspended
    Suspended {
        /// Orders waiting for host fulfillment
        pending: Vec<Order>,
        /// Orders that were cancelled
        cancelled: Vec<OrderId>,
        // Note: waiting_contexts count could be added if useful
    },
}
```

---

## 7. Complete Flow Example

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    COMPLETE CONCURRENT EXECUTION FLOW                       │
└─────────────────────────────────────────────────────────────────────────────┘

JavaScript                         Interpreter                          Host
    │                                  │                                  │
    │ let r1 = fetch(url1);            │                                  │
    │ // __order__() suspends          │                                  │
    │─────────────────────────────────>│                                  │
    │                                  │ Suspended { orders: [O1] }       │
    │                                  │─────────────────────────────────>│
    │                                  │                                  │
    │                                  │ fulfill_orders([{O1: Promise P1}])
    │                                  │<─────────────────────────────────│
    │                                  │                                  │
    │ r1 = P1 (unresolved)             │ continue_eval()                  │
    │<─────────────────────────────────│                                  │
    │                                  │                                  │
    │ let r2 = fetch(url2);            │                                  │
    │─────────────────────────────────>│                                  │
    │                                  │ Suspended { orders: [O2] }       │
    │                                  │─────────────────────────────────>│
    │                                  │                                  │
    │                                  │ fulfill_orders([{O2: Promise P2}])
    │                                  │<─────────────────────────────────│
    │                                  │                                  │
    │ r2 = P2 (unresolved)             │                                  │
    │<─────────────────────────────────│                                  │
    │                                  │                                  │
    │ async function doWork() {        │                                  │
    │   let x = await r1;              │                                  │
    │   // P1 is pending               │                                  │
    │─────────────────────────────────>│                                  │
    │                                  │                                  │
    │                     ┌────────────┴────────────┐                     │
    │                     │ Op::Await on P1         │                     │
    │                     │ P1 is PENDING           │                     │
    │                     │ Save context C1         │                     │
    │                     │ Add to wait_graph       │                     │
    │                     │ Return from async fn    │                     │
    │                     └────────────┬────────────┘                     │
    │                                  │                                  │
    │ doWork();  // returns Promise    │                                  │
    │ let y = await r2;                │                                  │
    │ // P2 is pending                 │                                  │
    │─────────────────────────────────>│                                  │
    │                                  │                                  │
    │                     ┌────────────┴────────────┐                     │
    │                     │ Op::Await on P2         │                     │
    │                     │ P2 is PENDING           │                     │
    │                     │ Save context C2         │                     │
    │                     │ Add to wait_graph       │                     │
    │                     └────────────┬────────────┘                     │
    │                                  │                                  │
    │ // No more sync code             │                                  │
    │                                  │ Suspended {                      │
    │                                  │   wait_graph: [C1→P1, C2→P2]    │
    │                                  │ }                                │
    │                                  │─────────────────────────────────>│
    │                                  │                                  │
    │                                  │         ┌────────────────────────┴─┐
    │                                  │         │ Network: url2 responds   │
    │                                  │         └────────────────────────┬─┘
    │                                  │                                  │
    │                                  │ resolve_promise(P2, "data2")     │
    │                                  │<─────────────────────────────────│
    │                                  │                                  │
    │                     ┌────────────┴────────────┐                     │
    │                     │ P2 fulfilled            │                     │
    │                     │ C2 waiting on P2        │                     │
    │                     │ C2 → ready_queue        │                     │
    │                     │ Resume C2               │                     │
    │                     └────────────┬────────────┘                     │
    │                                  │                                  │
    │ y = "data2"                      │                                  │
    │ // C2 continues...               │                                  │
    │ // C1 still waiting              │                                  │
    │                                  │                                  │
    │                                  │ Suspended { C1→P1 }              │
    │                                  │─────────────────────────────────>│
    │                                  │                                  │
    │                                  │         ┌────────────────────────┴─┐
    │                                  │         │ Network: url1 responds   │
    │                                  │         └────────────────────────┬─┘
    │                                  │                                  │
    │                                  │ resolve_promise(P1, "data1")     │
    │                                  │<─────────────────────────────────│
    │                                  │                                  │
    │                     ┌────────────┴────────────┐                     │
    │                     │ P1 fulfilled            │                     │
    │                     │ C1 waiting on P1        │                     │
    │                     │ Resume C1               │                     │
    │                     └────────────┬────────────┘                     │
    │                                  │                                  │
    │ x = "data1"                      │                                  │
    │ // doWork() continues...         │                                  │
    │                                  │                                  │
    │ // All done                      │ Complete(result)                 │
    │<─────────────────────────────────│─────────────────────────────────>│
```

---

## 8. Summary of Changes

### 8.1 Files to Modify

| File | Changes |
|------|---------|
| `src/value.rs` | Add `ExoticObject::PendingOrder { id: u64 }` |
| `src/interpreter/mod.rs` | Add `WaitGraph`, `SuspendedContext`, `ContextId`, `PromiseId`; update `continue_eval`, `run_vm_to_completion`; add `resolve_promise`, `reject_promise` |
| `src/interpreter/builtins/internal.rs` | Simplify `order_syscall` |
| `src/interpreter/bytecode_vm.rs` | Add `OpResult::SuspendForOrder`, `OpResult::SuspendAsyncContext`; add `VmResult::SuspendForOrder`, `VmResult::SuspendAsyncContext`; update `Op::Await` handler |
| `src/lib.rs` | Add `Runtime::resolve_promise`, `Runtime::reject_promise`, `Runtime::create_promise` |

### 8.2 Removed

| Item | Reason |
|------|--------|
| `order_callbacks` field | No longer creating internal promises for orders |
| `suspended_vm_state` field | Replaced by `WaitGraph` for multiple contexts |

### 8.3 New Capabilities

- Multiple async contexts can be suspended simultaneously
- Host can return unresolved Promises from orders
- Host can resolve Promises at any time via `resolve_promise`
- Proper concurrent async/await semantics

---

## 9. Testing Strategy

### 9.1 Order Fulfillment - Basic Scenarios

```rust
#[test]
fn test_order_returns_concrete_value() {
    // Order returns a simple value (number, string, object)
    // VM resumes immediately with that value
    let code = r#"
        let time = await __order__({ type: "getTime" });
        time
    "#;
    // Host fulfills with: 1703654321
    // Result: 1703654321
}

#[test]
fn test_order_returns_unresolved_promise() {
    // Order returns an unresolved Promise
    // VM resumes with Promise, continues execution
    // Later await on that Promise suspends
    let code = r#"
        let promise = await __order__({ type: "fetch", url: "/api" });
        // promise is unresolved Promise here
        let data = await promise;
        data
    "#;
    // Host fulfills order with: unresolved Promise P
    // VM continues, hits "await promise", suspends in wait_graph
    // Host resolves P with "response data"
    // Result: "response data"
}

#[test]
fn test_order_returns_already_resolved_promise() {
    // Order returns a Promise that's already fulfilled
    // await should complete immediately
    let code = r#"
        let promise = await __order__({ type: "cached" });
        let data = await promise;
        data
    "#;
    // Host fulfills with: already-fulfilled Promise containing "cached data"
    // Result: "cached data" (no second suspension)
}

#[test]
fn test_order_error_response() {
    // Order returns an error
    // Should throw in JS
    let code = r#"
        try {
            await __order__({ type: "fail" });
        } catch (e) {
            "caught: " + e
        }
    "#;
    // Host fulfills with: Err(JsError::...)
    // Result: "caught: ..."
}
```

### 9.2 Multiple Orders - Sequential

```rust
#[test]
fn test_sequential_orders() {
    // Multiple orders executed one after another
    let code = r#"
        let a = await __order__({ step: 1 });
        let b = await __order__({ step: 2 });
        let c = await __order__({ step: 3 });
        [a, b, c]
    "#;
    // 3 suspend/resume cycles
    // Result: [val1, val2, val3]
}

#[test]
fn test_order_value_depends_on_previous() {
    // Each order uses result from previous
    let code = r#"
        let userId = await __order__({ type: "getCurrentUser" });
        let profile = await __order__({ type: "getProfile", userId });
        let posts = await __order__({ type: "getPosts", userId });
        { userId, profile, posts }
    "#;
}
```

### 9.3 Concurrent Async Contexts

```rust
#[test]
fn test_two_async_functions_different_promises() {
    // Two async functions waiting on different promises
    // Resolved in reverse order
    let code = r#"
        let p1 = await __order__({ id: 1 });  // Returns Promise P1
        let p2 = await __order__({ id: 2 });  // Returns Promise P2

        let results = [];

        async function waitFor1() {
            let r = await p1;
            results.push("got1: " + r);
        }

        async function waitFor2() {
            let r = await p2;
            results.push("got2: " + r);
        }

        waitFor1();
        waitFor2();

        // Both suspended, waiting on P1 and P2
        // Return to host
    "#;
    // Host resolves P2 first with "data2"
    // waitFor2 resumes, pushes "got2: data2"
    // Host resolves P1 with "data1"
    // waitFor1 resumes, pushes "got1: data1"
    // Result: results = ["got2: data2", "got1: data1"]
}

#[test]
fn test_multiple_contexts_same_promise() {
    // Multiple async functions waiting on the SAME promise
    let code = r#"
        let p = await __order__({ type: "shared" });  // Returns Promise P

        let results = [];

        async function waiter(name) {
            let r = await p;
            results.push(name + ": " + r);
        }

        waiter("A");
        waiter("B");
        waiter("C");

        // All 3 contexts waiting on P
    "#;
    // Host resolves P with "shared data"
    // All 3 contexts should resume
    // Result: results = ["A: shared data", "B: shared data", "C: shared data"]
}

#[test]
fn test_async_function_calls_another_async() {
    // Nested async function calls
    let code = r#"
        async function inner() {
            let p = await __order__({ type: "inner" });
            return await p;
        }

        async function outer() {
            let x = await inner();
            let p = await __order__({ type: "outer" });
            let y = await p;
            return x + y;
        }

        outer()
    "#;
    // Complex nesting of suspensions
}
```

### 9.4 Promise.all / Promise.race

```rust
#[test]
fn test_promise_all_with_order_promises() {
    // Promise.all waiting on multiple order-returned promises
    let code = r#"
        let p1 = await __order__({ url: "/users" });
        let p2 = await __order__({ url: "/posts" });
        let p3 = await __order__({ url: "/comments" });

        let results = await Promise.all([p1, p2, p3]);
        results
    "#;
    // After orders fulfilled with promises P1, P2, P3
    // Promise.all creates internal promise waiting on all 3
    // Host can resolve in any order
    // Result: [data1, data2, data3] when all resolved
}

#[test]
fn test_promise_all_partial_resolution() {
    // Promise.all where some promises resolve, then more work
    let code = r#"
        let p1 = await __order__({ id: 1 });
        let p2 = await __order__({ id: 2 });

        async function waitAll() {
            return await Promise.all([p1, p2]);
        }

        let allPromise = waitAll();

        // Other work while waiting
        let p3 = await __order__({ id: 3 });
        let immediate = await p3;

        let allResults = await allPromise;
        [immediate, allResults]
    "#;
    // P3 might resolve before P1/P2
    // Both contexts should work independently
}

#[test]
fn test_promise_race_with_order_promises() {
    // Promise.race - first resolved wins
    let code = r#"
        let p1 = await __order__({ url: "/slow" });
        let p2 = await __order__({ url: "/fast" });

        let winner = await Promise.race([p1, p2]);
        winner
    "#;
    // Host resolves p2 first
    // Result: p2's value (p1 still pending but ignored)
}

#[test]
fn test_promise_all_one_rejects() {
    // Promise.all where one promise rejects
    let code = r#"
        let p1 = await __order__({ id: 1 });
        let p2 = await __order__({ id: 2 });

        try {
            await Promise.all([p1, p2]);
        } catch (e) {
            "failed: " + e
        }
    "#;
    // Host resolves p1 with value
    // Host rejects p2 with error
    // Promise.all rejects, catch block runs
}
```

### 9.5 Order Cancellation

```rust
#[test]
fn test_cancel_pending_order() {
    // Order is cancelled before fulfillment
    let code = r#"
        let orderId = __getOrderId__();
        let p = __order__({ id: orderId });
        __cancelOrder__(orderId);
        // What happens when we await cancelled order?
    "#;
}

#[test]
fn test_cancel_order_in_promise_race() {
    // Losing promises in Promise.race could be cancelled
    let code = r#"
        let p1 = await __order__({ id: 1 });
        let p2 = await __order__({ id: 2 });

        let winner = await Promise.race([p1, p2]);
        // Loser's order could be cancelled
    "#;
}
```

### 9.6 Error Handling

```rust
#[test]
fn test_await_rejected_promise() {
    // Promise rejected - should throw
    let code = r#"
        let p = await __order__({ type: "will_fail" });
        try {
            await p;
        } catch (e) {
            "caught: " + e
        }
    "#;
    // Host fulfills with unresolved promise
    // Host rejects promise with error
    // Catch block should run
}

#[test]
fn test_unhandled_rejection_in_async_context() {
    // Async function throws, no one catches
    let code = r#"
        let p = await __order__({ type: "will_fail" });

        async function willFail() {
            await p;  // This will throw
        }

        willFail();  // Returns rejected promise

        // What happens to the error?
    "#;
}

#[test]
fn test_error_in_one_context_doesnt_affect_others() {
    // One async context errors, others continue
    let code = r#"
        let p1 = await __order__({ id: 1 });
        let p2 = await __order__({ id: 2 });

        let results = [];

        async function fails() {
            try {
                await p1;
            } catch (e) {
                results.push("error: " + e);
            }
        }

        async function succeeds() {
            let r = await p2;
            results.push("success: " + r);
        }

        fails();
        succeeds();
    "#;
    // Host rejects p1, resolves p2
    // Both async functions should complete
    // Result: results contains both entries
}

#[test]
fn test_try_catch_in_async_function() {
    // Error handling within suspended async context
    let code = r#"
        async function fetchWithRetry() {
            let p = await __order__({ type: "unstable" });
            try {
                return await p;
            } catch (e) {
                // Retry
                let p2 = await __order__({ type: "retry" });
                return await p2;
            }
        }

        fetchWithRetry()
    "#;
}
```

### 9.7 Mixed Sync/Async Code

```rust
#[test]
fn test_sync_code_after_async_start() {
    // Sync code continues after starting async work
    let code = r#"
        let p = await __order__({ type: "fetch" });

        let syncResult = "computed";

        async function asyncWork() {
            return await p;
        }

        let promise = asyncWork();  // Starts, suspends at await

        // This sync code should execute
        syncResult = syncResult + " more";

        // Now wait for async
        let asyncResult = await promise;

        syncResult + " | " + asyncResult
    "#;
}

#[test]
fn test_loop_with_awaits() {
    // Loop that awaits multiple times
    let code = r#"
        async function fetchAll(urls) {
            let results = [];
            for (let url of urls) {
                let p = await __order__({ url });
                let data = await p;
                results.push(data);
            }
            return results;
        }

        fetchAll(["/a", "/b", "/c"])
    "#;
    // Multiple order/resume cycles in a loop
}

#[test]
fn test_promise_in_object_property() {
    // Promise stored in object, awaited later
    let code = r#"
        let obj = {
            promise: await __order__({ type: "deferred" })
        };

        // Later...
        let result = await obj.promise;
        result
    "#;
}
```

### 9.8 Resolve/Reject Promise API

```rust
#[test]
fn test_host_creates_and_resolves_promise() {
    // Host creates promise via API, returns from order, resolves later
    let code = r#"
        let p = await __order__({ type: "manual" });
        await p
    "#;

    // Host:
    // 1. runtime.create_promise() -> (promise, guard)
    // 2. fulfill_orders([{ id, result: Ok(promise) }])
    // 3. Later: runtime.resolve_promise(promise, value)
    // Result: value
}

#[test]
fn test_resolve_promise_wakes_multiple_waiters() {
    // Multiple contexts waiting, one resolve wakes all
    let code = r#"
        let p = await __order__({ type: "shared" });

        let count = 0;

        async function waiter() {
            await p;
            count++;
        }

        waiter();
        waiter();
        waiter();

        // Wait for all
    "#;
    // Host resolves p once
    // All 3 waiters should resume
    // count should be 3
}

#[test]
fn test_reject_promise_api() {
    // Host rejects promise, waiters get error
    let code = r#"
        let p = await __order__({ type: "will_reject" });

        try {
            await p;
        } catch (e) {
            "rejected: " + e
        }
    "#;
    // Host: runtime.reject_promise(promise, error)
}

#[test]
fn test_resolve_already_settled_promise() {
    // Trying to resolve an already-resolved promise
    // Should be no-op
    let code = r#"
        let p = await __order__({ type: "test" });
        await p
    "#;
    // Host resolves p with "first"
    // Host tries to resolve p with "second" (should be ignored)
    // Result: "first"
}
```

### 9.9 GC Safety

```rust
#[test]
fn test_gc_during_suspension() {
    // Objects should survive GC while context is suspended
    let code = r#"
        async function test() {
            let bigObject = { data: new Array(1000).fill("x") };
            let p = await __order__({ type: "slow" });
            await p;
            return bigObject.data.length;
        }

        test()
    "#;
    // Force GC between order fulfillment and promise resolution
    // bigObject should still be accessible
}

#[test]
fn test_promise_survives_gc() {
    // Promise object survives across suspension
    let code = r#"
        let p = await __order__({ type: "fetch" });
        // Force GC here
        await p
    "#;
}

#[test]
fn test_multiple_contexts_gc_safety() {
    // Multiple suspended contexts, GC runs
    let code = r#"
        let p1 = await __order__({ id: 1 });
        let p2 = await __order__({ id: 2 });

        let data1 = { value: "one" };
        let data2 = { value: "two" };

        async function use1() {
            await p1;
            return data1.value;
        }

        async function use2() {
            await p2;
            return data2.value;
        }

        use1();
        use2();

        // Both contexts suspended, referencing data1/data2
        // GC should not collect these
    "#;
}
```

### 9.10 Edge Cases

```rust
#[test]
fn test_immediate_await_after_order() {
    // await immediately on order result (common pattern)
    let code = r#"
        let result = await (await __order__({ type: "fetch" }));
        result
    "#;
}

#[test]
fn test_order_in_promise_constructor() {
    // Order inside Promise executor (unusual but valid)
    let code = r#"
        let p = new Promise(async (resolve) => {
            let data = await __order__({ type: "inner" });
            resolve(data);
        });
        await p
    "#;
}

#[test]
fn test_deeply_nested_async() {
    // Deep nesting of async calls
    let code = r#"
        async function level1() {
            return await level2();
        }
        async function level2() {
            return await level3();
        }
        async function level3() {
            let p = await __order__({ type: "deep" });
            return await p;
        }

        level1()
    "#;
}

#[test]
fn test_order_returns_function() {
    // Order returns a function
    let code = r#"
        let fn = await __order__({ type: "getFunction" });
        fn(42)
    "#;
    // Host returns a native function or JS function
}

#[test]
fn test_order_returns_promise_that_resolves_to_promise() {
    // Promise chain: order -> Promise -> Promise -> value
    let code = r#"
        let p = await __order__({ type: "nested" });
        await p  // Should unwrap to final value
    "#;
    // Host returns Promise that resolves to another Promise
    // await should handle full chain
}

#[test]
fn test_zero_pending_orders_but_waiting_contexts() {
    // All orders fulfilled, but contexts still waiting on promises
    let code = r#"
        let p1 = await __order__({ id: 1 });
        let p2 = await __order__({ id: 2 });

        async function wait() {
            await p1;
            await p2;
        }

        wait();
    "#;
    // After fulfilling both orders with promises
    // pending_orders is empty
    // But wait_graph has contexts
    // Should correctly return Suspended
}

#[test]
fn test_no_orders_just_promise_waits() {
    // Promises created in JS, not from orders
    let code = r#"
        let resolve;
        let p = new Promise(r => { resolve = r; });

        async function waiter() {
            return await p;
        }

        waiter();

        // resolve is available but we can't call it from host
        // This tests the wait_graph without orders
    "#;
}

#[test]
fn test_order_during_promise_handler() {
    // Order called inside .then() handler
    let code = r#"
        let p = await __order__({ id: 1 });

        p.then(async (value) => {
            let p2 = await __order__({ id: 2 });
            return await p2;
        });
    "#;
}
```

### 9.11 Performance / Stress Tests

```rust
#[test]
fn test_many_concurrent_contexts() {
    // Many async contexts waiting simultaneously
    let code = r#"
        let promises = [];
        for (let i = 0; i < 100; i++) {
            promises.push(__order__({ id: i }));
        }

        // Wait for all orders to be fulfilled with promises
        let allPromises = await Promise.all(promises);

        // Now start many async contexts
        let results = [];
        for (let p of allPromises) {
            (async () => {
                let r = await p;
                results.push(r);
            })();
        }

        // Many contexts in wait_graph
    "#;
}

#[test]
fn test_rapid_order_fulfill_cycle() {
    // Quick succession of order/fulfill cycles
    let code = r#"
        let sum = 0;
        for (let i = 0; i < 50; i++) {
            sum += await __order__({ value: i });
        }
        sum
    "#;
    // 50 order/fulfill cycles
}
```

### 9.12 Existing Tests Must Pass

All existing tests in `tests/interpreter/orders.rs` must continue to pass:
- `test_order_returns_pending`
- `test_order_basic_fulfilled`
- `test_order_in_promise_all`
- `test_sequential_awaits`
- `test_order_cancellation`
- `test_order_with_gc_stress`
- etc.

### 9.13 Test Categories Summary

| Category | Tests | Key Behaviors |
|----------|-------|---------------|
| Order Fulfillment | 4 | Concrete values, unresolved promises, resolved promises, errors |
| Sequential Orders | 2 | Multiple orders in sequence |
| Concurrent Contexts | 3 | Multiple async fns, same promise, nested async |
| Promise.all/race | 4 | Concurrent waits, partial resolution, rejection |
| Cancellation | 2 | Cancel pending, cancel in race |
| Error Handling | 4 | Rejection, unhandled, isolation, try/catch |
| Mixed Sync/Async | 3 | Interleaved execution |
| Host Promise API | 4 | create/resolve/reject, multiple waiters |
| GC Safety | 3 | Objects survive suspension |
| Edge Cases | 8 | Nested awaits, unusual patterns |
| Performance | 2 | Many contexts, rapid cycles |
