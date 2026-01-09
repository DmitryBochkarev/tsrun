package tsrun

import (
	"context"
	"fmt"
)

// Context represents a tsrun interpreter context.
type Context struct {
	rt     *Runtime
	handle uint32 // Pointer to TsRunContext
}

// NewContext creates a new interpreter context.
func (r *Runtime) NewContext(ctx context.Context) (*Context, error) {
	results, err := r.fnNew.Call(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to create context: %w", err)
	}

	handle := uint32(results[0])
	if handle == 0 {
		return nil, fmt.Errorf("context creation returned null")
	}

	return &Context{
		rt:     r,
		handle: handle,
	}, nil
}

// Free releases the context resources.
func (c *Context) Free(ctx context.Context) error {
	if c.handle == 0 {
		return nil
	}
	_, err := c.rt.fnFree.Call(ctx, uint64(c.handle))
	c.handle = 0
	return err
}

// Prepare compiles code for execution.
// path is optional (use "" for anonymous scripts).
func (c *Context) Prepare(ctx context.Context, code string, path string) error {
	// Allocate code string
	codePtr, err := c.rt.allocString(ctx, code)
	if err != nil {
		return fmt.Errorf("failed to allocate code: %w", err)
	}
	defer c.rt.deallocString(ctx, codePtr, uint32(len(code)+1))

	// Allocate path string if provided
	var pathPtr uint32
	if path != "" {
		pathPtr, err = c.rt.allocString(ctx, path)
		if err != nil {
			return fmt.Errorf("failed to allocate path: %w", err)
		}
		defer c.rt.deallocString(ctx, pathPtr, uint32(len(path)+1))
	}

	// Allocate space for TsRunResult struct (sret convention)
	// TsRunResult: { ok: bool (4 bytes padded), error: *const c_char (4 bytes) } = 8 bytes
	const resultSize = 8
	resultPtr, err := c.rt.allocResult(ctx, resultSize)
	if err != nil {
		return fmt.Errorf("failed to allocate result: %w", err)
	}
	defer c.rt.deallocResult(ctx, resultPtr, resultSize)

	// Call tsrun_prepare with sret pointer as first argument
	_, err = c.rt.fnPrepare.Call(ctx, uint64(resultPtr), uint64(c.handle), uint64(codePtr), uint64(pathPtr))
	if err != nil {
		return fmt.Errorf("prepare call failed: %w", err)
	}

	// Read TsRunResult from memory
	// offset 0: ok (i32, but actually bool)
	// offset 4: error (*const c_char)
	okVal, _ := c.rt.memory.ReadUint32Le(resultPtr)
	errorPtr, _ := c.rt.memory.ReadUint32Le(resultPtr + 4)

	if okVal == 0 {
		errMsg := c.rt.readString(errorPtr)
		return fmt.Errorf("prepare error: %s", errMsg)
	}

	return nil
}

// Step executes one instruction.
func (c *Context) Step(ctx context.Context) (*StepResult, error) {
	// Allocate space for TsRunStepResult struct (sret convention)
	// TsRunStepResult layout (wasm32):
	// - status: i32 (4 bytes)
	// - value: *mut TsRunValue (4 bytes)
	// - imports: *mut TsRunImportRequest (4 bytes)
	// - import_count: usize (4 bytes)
	// - pending_orders: *mut TsRunOrder (4 bytes)
	// - pending_count: usize (4 bytes)
	// - cancelled_orders: *mut u64 (4 bytes)
	// - cancelled_count: usize (4 bytes)
	// - error: *const c_char (4 bytes)
	// Total: 36 bytes
	const resultSize = 36
	resultPtr, err := c.rt.allocResult(ctx, resultSize)
	if err != nil {
		return nil, fmt.Errorf("failed to allocate step result: %w", err)
	}

	_, err = c.rt.fnStep.Call(ctx, uint64(resultPtr), uint64(c.handle))
	if err != nil {
		c.rt.deallocResult(ctx, resultPtr, resultSize)
		return nil, fmt.Errorf("step call failed: %w", err)
	}

	return c.parseStepResultFromPtr(ctx, resultPtr, resultSize)
}

// Run executes until completion, needing imports, or suspension.
func (c *Context) Run(ctx context.Context) (*StepResult, error) {
	// Same struct size as Step
	const resultSize = 36
	resultPtr, err := c.rt.allocResult(ctx, resultSize)
	if err != nil {
		return nil, fmt.Errorf("failed to allocate run result: %w", err)
	}

	_, err = c.rt.fnRun.Call(ctx, uint64(resultPtr), uint64(c.handle))
	if err != nil {
		c.rt.deallocResult(ctx, resultPtr, resultSize)
		return nil, fmt.Errorf("run call failed: %w", err)
	}

	return c.parseStepResultFromPtr(ctx, resultPtr, resultSize)
}

// parseStepResultFromPtr parses the TsRunStepResult structure from a memory pointer.
func (c *Context) parseStepResultFromPtr(ctx context.Context, resultPtr uint32, resultSize uint32) (*StepResult, error) {
	// TsRunStepResult layout (wasm32):
	// offset 0: status (i32)
	// offset 4: value (i32 pointer)
	// offset 8: imports (i32 pointer)
	// offset 12: import_count (i32)
	// offset 16: pending_orders (i32 pointer)
	// offset 20: pending_count (i32)
	// offset 24: cancelled_orders (i32 pointer)
	// offset 28: cancelled_count (i32)
	// offset 32: error (i32 pointer)

	statusVal, _ := c.rt.memory.ReadUint32Le(resultPtr)
	valuePtr, _ := c.rt.memory.ReadUint32Le(resultPtr + 4)
	importsPtr, _ := c.rt.memory.ReadUint32Le(resultPtr + 8)
	importCount, _ := c.rt.memory.ReadUint32Le(resultPtr + 12)
	pendingPtr, _ := c.rt.memory.ReadUint32Le(resultPtr + 16)
	pendingCount, _ := c.rt.memory.ReadUint32Le(resultPtr + 20)
	cancelledPtr, _ := c.rt.memory.ReadUint32Le(resultPtr + 24)
	cancelledCount, _ := c.rt.memory.ReadUint32Le(resultPtr + 28)
	errorPtr, _ := c.rt.memory.ReadUint32Le(resultPtr + 32)

	result := &StepResult{
		Status: StepStatus(statusVal),
	}

	// Parse based on status
	switch result.Status {
	case StatusComplete:
		if valuePtr != 0 {
			result.Value = &Value{ctx: c, handle: valuePtr}
		}

	case StatusError:
		if errorPtr != 0 {
			result.Error = c.rt.readString(errorPtr)
		}

	case StatusNeedImports:
		result.ImportRequests = c.parseImportRequests(importsPtr, importCount)

	case StatusSuspended:
		result.PendingOrders = c.parsePendingOrders(pendingPtr, pendingCount)
		result.CancelledOrders = c.parseCancelledOrders(cancelledPtr, cancelledCount)
	}

	// Free the step result structure's internal arrays (but not the value)
	if c.rt.fnStepResultFree != nil {
		c.rt.fnStepResultFree.Call(ctx, uint64(resultPtr))
	}

	// Free the result struct memory we allocated
	c.rt.deallocResult(ctx, resultPtr, resultSize)

	return result, nil
}

// parseImportRequests parses an array of TsRunImportRequest structs.
func (c *Context) parseImportRequests(ptr uint32, count uint32) []ImportRequest {
	if ptr == 0 || count == 0 {
		return nil
	}

	// TsRunImportRequest layout (wasm32):
	// offset 0: specifier (i32 pointer to C string)
	// offset 4: resolved_path (i32 pointer to C string)
	// offset 8: importer (i32 pointer to C string, may be null)
	const structSize = 12

	requests := make([]ImportRequest, count)
	for i := uint32(0); i < count; i++ {
		offset := ptr + i*structSize
		specifierPtr, _ := c.rt.memory.ReadUint32Le(offset)
		resolvedPtr, _ := c.rt.memory.ReadUint32Le(offset + 4)
		importerPtr, _ := c.rt.memory.ReadUint32Le(offset + 8)

		requests[i] = ImportRequest{
			Specifier:    c.rt.readString(specifierPtr),
			ResolvedPath: c.rt.readString(resolvedPtr),
			Importer:     c.rt.readString(importerPtr),
		}
	}
	return requests
}

// parsePendingOrders parses an array of TsRunOrder structs.
func (c *Context) parsePendingOrders(ptr uint32, count uint32) []Order {
	if ptr == 0 || count == 0 {
		return nil
	}

	// TsRunOrder layout (wasm32):
	// offset 0: id (u64)
	// offset 8: payload (i32 pointer to TsRunValue)
	const structSize = 12 // 8 + 4 on wasm32

	orders := make([]Order, count)
	for i := uint32(0); i < count; i++ {
		offset := ptr + i*structSize
		id, _ := c.rt.memory.ReadUint64Le(offset)
		payloadPtr, _ := c.rt.memory.ReadUint32Le(offset + 8)

		var payload *Value
		if payloadPtr != 0 {
			payload = &Value{ctx: c, handle: payloadPtr}
		}

		orders[i] = Order{
			ID:      id,
			Payload: payload,
		}
	}
	return orders
}

// parseCancelledOrders parses an array of u64 order IDs.
func (c *Context) parseCancelledOrders(ptr uint32, count uint32) []uint64 {
	if ptr == 0 || count == 0 {
		return nil
	}

	ids := make([]uint64, count)
	for i := uint32(0); i < count; i++ {
		ids[i], _ = c.rt.memory.ReadUint64Le(ptr + i*8)
	}
	return ids
}

// ProvideModule provides source code for a requested module.
func (c *Context) ProvideModule(ctx context.Context, path string, source string) error {
	if c.rt.fnProvideModule == nil {
		return fmt.Errorf("provide_module not available")
	}

	pathPtr, err := c.rt.allocString(ctx, path)
	if err != nil {
		return err
	}
	defer c.rt.deallocString(ctx, pathPtr, uint32(len(path)+1))

	sourcePtr, err := c.rt.allocString(ctx, source)
	if err != nil {
		return err
	}
	defer c.rt.deallocString(ctx, sourcePtr, uint32(len(source)+1))

	// Allocate space for TsRunResult struct (sret convention)
	const resultSize = 8
	resultPtr, err := c.rt.allocResult(ctx, resultSize)
	if err != nil {
		return err
	}
	defer c.rt.deallocResult(ctx, resultPtr, resultSize)

	_, err = c.rt.fnProvideModule.Call(ctx, uint64(resultPtr), uint64(c.handle), uint64(pathPtr), uint64(sourcePtr))
	if err != nil {
		return err
	}

	// Read TsRunResult from memory
	okVal, _ := c.rt.memory.ReadUint32Le(resultPtr)
	errorPtr, _ := c.rt.memory.ReadUint32Le(resultPtr + 4)

	if okVal == 0 {
		return fmt.Errorf("provide_module error: %s", c.rt.readString(errorPtr))
	}

	return nil
}

// FulfillOrders fulfills pending orders with responses.
func (c *Context) FulfillOrders(ctx context.Context, responses []OrderResponse) error {
	if c.rt.fnFulfillOrders == nil {
		return fmt.Errorf("fulfill_orders not available")
	}

	if len(responses) == 0 {
		return nil
	}

	// TsRunOrderResponse layout (wasm32):
	// offset 0: id (u64, 8 bytes)
	// offset 8: value (*mut TsRunValue, 4 bytes)
	// offset 12: error (*const c_char, 4 bytes)
	// Total: 16 bytes per response
	const responseSize = 16

	// Allocate array for all responses
	arraySize := uint32(len(responses) * responseSize)
	arrayPtr, err := c.rt.allocResult(ctx, arraySize)
	if err != nil {
		return fmt.Errorf("failed to allocate responses array: %w", err)
	}
	defer c.rt.deallocResult(ctx, arrayPtr, arraySize)

	// Track error strings we allocate so we can free them
	var errorPtrs []uint32

	// Write each response to the array
	for i, resp := range responses {
		offset := arrayPtr + uint32(i*responseSize)

		// Write id (u64 at offset 0)
		c.rt.memory.WriteUint64Le(offset, resp.ID)

		// Write value pointer (i32 at offset 8)
		var valueHandle uint32
		if resp.Value != nil {
			valueHandle = resp.Value.handle
		}
		c.rt.memory.WriteUint32Le(offset+8, valueHandle)

		// Write error pointer (i32 at offset 12)
		var errorPtr uint32
		if resp.Error != "" {
			errorPtr, err = c.rt.allocString(ctx, resp.Error)
			if err != nil {
				// Clean up any error strings we already allocated
				for _, ptr := range errorPtrs {
					c.rt.fnDealloc.Call(ctx, uint64(ptr), uint64(1)) // Size doesn't matter for cleanup
				}
				return fmt.Errorf("failed to allocate error string: %w", err)
			}
			errorPtrs = append(errorPtrs, errorPtr)
		}
		c.rt.memory.WriteUint32Le(offset+12, errorPtr)
	}

	// Allocate space for TsRunResult (sret convention)
	const resultSize = 8
	resultPtr, err := c.rt.allocResult(ctx, resultSize)
	if err != nil {
		// Clean up error strings
		for _, ptr := range errorPtrs {
			c.rt.fnDealloc.Call(ctx, uint64(ptr), uint64(1))
		}
		return fmt.Errorf("failed to allocate result: %w", err)
	}
	defer c.rt.deallocResult(ctx, resultPtr, resultSize)

	// Call tsrun_fulfill_orders(sret, ctx, responses, count)
	_, err = c.rt.fnFulfillOrders.Call(ctx,
		uint64(resultPtr),
		uint64(c.handle),
		uint64(arrayPtr),
		uint64(len(responses)))

	// Clean up error strings (after call, since Rust reads them during the call)
	for _, ptr := range errorPtrs {
		c.rt.fnDealloc.Call(ctx, uint64(ptr), uint64(1))
	}

	if err != nil {
		return fmt.Errorf("fulfill_orders call failed: %w", err)
	}

	// Read TsRunResult from memory
	okVal, _ := c.rt.memory.ReadUint32Le(resultPtr)
	errorPtr, _ := c.rt.memory.ReadUint32Le(resultPtr + 4)

	if okVal == 0 {
		return fmt.Errorf("fulfill_orders error: %s", c.rt.readString(errorPtr))
	}

	return nil
}

// CreateOrderPromise creates a promise for deferred order fulfillment.
// The returned promise can be used as the order response value, and then
// resolved later using ResolvePromise.
func (c *Context) CreateOrderPromise(ctx context.Context, orderID uint64) (*Value, error) {
	if c.rt.fnCreateOrderPromise == nil {
		return nil, fmt.Errorf("create_order_promise not available")
	}

	// tsrun_create_order_promise returns TsRunValueResult (sret convention)
	// TsRunValueResult: { value: *mut TsRunValue (4 bytes), error: *const c_char (4 bytes) } = 8 bytes
	const resultSize = 8
	resultPtr, err := c.rt.allocResult(ctx, resultSize)
	if err != nil {
		return nil, fmt.Errorf("failed to allocate result: %w", err)
	}
	defer c.rt.deallocResult(ctx, resultPtr, resultSize)

	// Call tsrun_create_order_promise(sret, ctx, order_id)
	_, err = c.rt.fnCreateOrderPromise.Call(ctx, uint64(resultPtr), uint64(c.handle), orderID)
	if err != nil {
		return nil, fmt.Errorf("create_order_promise call failed: %w", err)
	}

	// Read TsRunValueResult from memory
	valuePtr, _ := c.rt.memory.ReadUint32Le(resultPtr)
	errorPtr, _ := c.rt.memory.ReadUint32Le(resultPtr + 4)

	if valuePtr == 0 {
		errMsg := c.rt.readString(errorPtr)
		return nil, fmt.Errorf("create_order_promise error: %s", errMsg)
	}

	return &Value{ctx: c, handle: valuePtr}, nil
}

// ResolvePromise resolves a promise created with CreateOrderPromise.
func (c *Context) ResolvePromise(ctx context.Context, promise *Value, value *Value) error {
	if c.rt.fnResolvePromise == nil {
		return fmt.Errorf("resolve_promise not available")
	}

	var valueHandle uint32
	if value != nil {
		valueHandle = value.handle
	}

	// tsrun_resolve_promise returns TsRunResult (sret convention)
	// TsRunResult: { ok: bool (4 bytes), error: *const c_char (4 bytes) } = 8 bytes
	const resultSize = 8
	resultPtr, err := c.rt.allocResult(ctx, resultSize)
	if err != nil {
		return fmt.Errorf("failed to allocate result: %w", err)
	}
	defer c.rt.deallocResult(ctx, resultPtr, resultSize)

	// Call tsrun_resolve_promise(sret, ctx, promise, value)
	_, err = c.rt.fnResolvePromise.Call(ctx, uint64(resultPtr), uint64(c.handle), uint64(promise.handle), uint64(valueHandle))
	if err != nil {
		return fmt.Errorf("resolve_promise call failed: %w", err)
	}

	// Read TsRunResult from memory
	okVal, _ := c.rt.memory.ReadUint32Le(resultPtr)
	errorPtr, _ := c.rt.memory.ReadUint32Le(resultPtr + 4)

	if okVal == 0 {
		return fmt.Errorf("resolve_promise error: %s", c.rt.readString(errorPtr))
	}

	return nil
}

// RejectPromise rejects a promise created with CreateOrderPromise.
func (c *Context) RejectPromise(ctx context.Context, promise *Value, errorMsg string) error {
	if c.rt.fnRejectPromise == nil {
		return fmt.Errorf("reject_promise not available")
	}

	// Allocate error string
	errorPtr, err := c.rt.allocString(ctx, errorMsg)
	if err != nil {
		return fmt.Errorf("failed to allocate error string: %w", err)
	}
	defer c.rt.deallocString(ctx, errorPtr, uint32(len(errorMsg)+1))

	// tsrun_reject_promise returns TsRunResult (sret convention)
	const resultSize = 8
	resultPtr, err := c.rt.allocResult(ctx, resultSize)
	if err != nil {
		return fmt.Errorf("failed to allocate result: %w", err)
	}
	defer c.rt.deallocResult(ctx, resultPtr, resultSize)

	// Call tsrun_reject_promise(sret, ctx, promise, error)
	_, err = c.rt.fnRejectPromise.Call(ctx, uint64(resultPtr), uint64(c.handle), uint64(promise.handle), uint64(errorPtr))
	if err != nil {
		return fmt.Errorf("reject_promise call failed: %w", err)
	}

	// Read TsRunResult from memory
	okVal, _ := c.rt.memory.ReadUint32Le(resultPtr)
	errMsgPtr, _ := c.rt.memory.ReadUint32Le(resultPtr + 4)

	if okVal == 0 {
		return fmt.Errorf("reject_promise error: %s", c.rt.readString(errMsgPtr))
	}

	return nil
}
