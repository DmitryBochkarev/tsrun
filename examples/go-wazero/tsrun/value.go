package tsrun

import (
	"context"
	"fmt"
	"math"
)

// Value represents a JavaScript value handle.
type Value struct {
	ctx    *Context
	handle uint32 // Pointer to TsRunValue
}

// Handle returns the raw WASM handle for this value.
func (v *Value) Handle() uint32 {
	return v.handle
}

// Free releases the value resources.
func (v *Value) Free(ctx context.Context) error {
	if v.handle == 0 || v.ctx.rt.fnValueFree == nil {
		return nil
	}
	_, err := v.ctx.rt.fnValueFree.Call(ctx, uint64(v.handle))
	v.handle = 0
	return err
}

// Type returns the JavaScript type of the value.
func (v *Value) Type(ctx context.Context) (ValueType, error) {
	if v.handle == 0 || v.ctx.rt.fnGetType == nil {
		return TypeUndefined, nil
	}

	results, err := v.ctx.rt.fnGetType.Call(ctx, uint64(v.handle))
	if err != nil {
		return TypeUndefined, err
	}

	return ValueType(results[0]), nil
}

// AsNumber returns the value as a number, or an error if not a number.
func (v *Value) AsNumber(ctx context.Context) (float64, error) {
	if v.handle == 0 || v.ctx.rt.fnGetNumber == nil {
		return 0, fmt.Errorf("value is nil or function not available")
	}

	results, err := v.ctx.rt.fnGetNumber.Call(ctx, uint64(v.handle))
	if err != nil {
		return 0, err
	}

	// WASM returns f64 as uint64 bit representation
	return math.Float64frombits(results[0]), nil
}

// AsString returns the value as a string, or an error if not a string.
func (v *Value) AsString(ctx context.Context) (string, error) {
	if v.handle == 0 || v.ctx.rt.fnGetString == nil {
		return "", fmt.Errorf("value is nil or function not available")
	}

	// tsrun_get_string(val: *const TsRunValue) -> *const c_char
	// Returns null if not a string
	results, err := v.ctx.rt.fnGetString.Call(ctx, uint64(v.handle))
	if err != nil {
		return "", err
	}

	strPtr := uint32(results[0])
	if strPtr == 0 {
		return "", nil
	}

	str := v.ctx.rt.readString(strPtr)

	// Free the allocated string
	if v.ctx.rt.fnFreeString != nil {
		v.ctx.rt.fnFreeString.Call(ctx, uint64(strPtr))
	}

	return str, nil
}

// AsBool returns the value as a boolean, or an error if not a boolean.
func (v *Value) AsBool(ctx context.Context) (bool, error) {
	if v.handle == 0 || v.ctx.rt.fnGetBool == nil {
		return false, fmt.Errorf("value is nil or function not available")
	}

	results, err := v.ctx.rt.fnGetBool.Call(ctx, uint64(v.handle))
	if err != nil {
		return false, err
	}

	// Returns { value: bool, valid: bool }
	return results[0] != 0, nil
}

// IsNull returns true if the value is null.
func (v *Value) IsNull(ctx context.Context) bool {
	if v.handle == 0 || v.ctx.rt.fnIsNull == nil {
		return false
	}

	results, _ := v.ctx.rt.fnIsNull.Call(ctx, uint64(v.handle))
	return len(results) > 0 && results[0] != 0
}

// IsUndefined returns true if the value is undefined.
func (v *Value) IsUndefined(ctx context.Context) bool {
	if v.handle == 0 || v.ctx.rt.fnIsUndefined == nil {
		return true
	}

	results, _ := v.ctx.rt.fnIsUndefined.Call(ctx, uint64(v.handle))
	return len(results) > 0 && results[0] != 0
}

// IsArray returns true if the value is an array.
func (v *Value) IsArray(ctx context.Context) bool {
	if v.handle == 0 || v.ctx.rt.fnIsArray == nil {
		return false
	}

	results, _ := v.ctx.rt.fnIsArray.Call(ctx, uint64(v.handle))
	return len(results) > 0 && results[0] != 0
}

// IsFunction returns true if the value is a function.
func (v *Value) IsFunction(ctx context.Context) bool {
	if v.handle == 0 || v.ctx.rt.fnIsFunction == nil {
		return false
	}

	results, _ := v.ctx.rt.fnIsFunction.Call(ctx, uint64(v.handle))
	return len(results) > 0 && results[0] != 0
}

// Get retrieves a property from an object.
func (v *Value) Get(ctx context.Context, key string) (*Value, error) {
	if v.handle == 0 || v.ctx.rt.fnGet == nil {
		return nil, fmt.Errorf("value is nil or function not available")
	}

	keyPtr, err := v.ctx.rt.allocString(ctx, key)
	if err != nil {
		return nil, err
	}
	defer v.ctx.rt.deallocString(ctx, keyPtr, uint32(len(key)+1))

	// TsRunValueResult: { value: *TsRunValue (4 bytes), error: *c_char (4 bytes) } = 8 bytes
	const resultSize = 8
	resultPtr, err := v.ctx.rt.allocResult(ctx, resultSize)
	if err != nil {
		return nil, fmt.Errorf("failed to allocate result: %w", err)
	}
	defer v.ctx.rt.deallocResult(ctx, resultPtr, resultSize)

	// Call with sret convention: (sret, ctx, obj, key)
	_, err = v.ctx.rt.fnGet.Call(ctx, uint64(resultPtr), uint64(v.ctx.handle), uint64(v.handle), uint64(keyPtr))
	if err != nil {
		return nil, err
	}

	// Read TsRunValueResult from memory
	valuePtr, _ := v.ctx.rt.memory.ReadUint32Le(resultPtr)
	errorPtr, _ := v.ctx.rt.memory.ReadUint32Le(resultPtr + 4)

	if errorPtr != 0 {
		return nil, fmt.Errorf("get error: %s", v.ctx.rt.readString(errorPtr))
	}

	if valuePtr == 0 {
		return nil, nil
	}

	return &Value{ctx: v.ctx, handle: valuePtr}, nil
}

// Set sets a property on an object.
func (v *Value) Set(ctx context.Context, key string, value *Value) error {
	if v.handle == 0 || v.ctx.rt.fnSet == nil {
		return fmt.Errorf("value is nil or function not available")
	}

	keyPtr, err := v.ctx.rt.allocString(ctx, key)
	if err != nil {
		return err
	}
	defer v.ctx.rt.deallocString(ctx, keyPtr, uint32(len(key)+1))

	valueHandle := uint32(0)
	if value != nil {
		valueHandle = value.handle
	}

	// TsRunResult: { ok: bool (4 bytes), error: *c_char (4 bytes) } = 8 bytes
	const resultSize = 8
	resultPtr, err := v.ctx.rt.allocResult(ctx, resultSize)
	if err != nil {
		return fmt.Errorf("failed to allocate result: %w", err)
	}
	defer v.ctx.rt.deallocResult(ctx, resultPtr, resultSize)

	// Call with sret convention: (sret, ctx, obj, key, val)
	_, err = v.ctx.rt.fnSet.Call(ctx, uint64(resultPtr), uint64(v.ctx.handle), uint64(v.handle), uint64(keyPtr), uint64(valueHandle))
	if err != nil {
		return err
	}

	// Read TsRunResult from memory
	okVal, _ := v.ctx.rt.memory.ReadUint32Le(resultPtr)
	errorPtr, _ := v.ctx.rt.memory.ReadUint32Le(resultPtr + 4)

	if okVal == 0 {
		return fmt.Errorf("set error: %s", v.ctx.rt.readString(errorPtr))
	}

	return nil
}

// Context value creation methods

// Number creates a number value.
func (c *Context) Number(ctx context.Context, n float64) (*Value, error) {
	if c.rt.fnNumber == nil {
		return nil, fmt.Errorf("number function not available")
	}

	results, err := c.rt.fnNumber.Call(ctx, uint64(c.handle), uint64(n))
	if err != nil {
		return nil, err
	}

	valuePtr := uint32(results[0])
	if valuePtr == 0 {
		return nil, fmt.Errorf("failed to create number")
	}

	return &Value{ctx: c, handle: valuePtr}, nil
}

// String creates a string value.
func (c *Context) String(ctx context.Context, s string) (*Value, error) {
	if c.rt.fnString == nil {
		return nil, fmt.Errorf("string function not available")
	}

	strPtr, err := c.rt.allocString(ctx, s)
	if err != nil {
		return nil, err
	}
	defer c.rt.deallocString(ctx, strPtr, uint32(len(s)+1))

	results, err := c.rt.fnString.Call(ctx, uint64(c.handle), uint64(strPtr))
	if err != nil {
		return nil, err
	}

	valuePtr := uint32(results[0])
	if valuePtr == 0 {
		return nil, fmt.Errorf("failed to create string")
	}

	return &Value{ctx: c, handle: valuePtr}, nil
}

// Boolean creates a boolean value.
func (c *Context) Boolean(ctx context.Context, b bool) (*Value, error) {
	if c.rt.fnBoolean == nil {
		return nil, fmt.Errorf("boolean function not available")
	}

	var bVal uint64
	if b {
		bVal = 1
	}

	results, err := c.rt.fnBoolean.Call(ctx, uint64(c.handle), bVal)
	if err != nil {
		return nil, err
	}

	valuePtr := uint32(results[0])
	if valuePtr == 0 {
		return nil, fmt.Errorf("failed to create boolean")
	}

	return &Value{ctx: c, handle: valuePtr}, nil
}

// Null creates a null value.
func (c *Context) Null(ctx context.Context) (*Value, error) {
	if c.rt.fnNull == nil {
		return nil, fmt.Errorf("null function not available")
	}

	results, err := c.rt.fnNull.Call(ctx, uint64(c.handle))
	if err != nil {
		return nil, err
	}

	valuePtr := uint32(results[0])
	if valuePtr == 0 {
		return nil, fmt.Errorf("failed to create null")
	}

	return &Value{ctx: c, handle: valuePtr}, nil
}

// Undefined creates an undefined value.
func (c *Context) Undefined(ctx context.Context) (*Value, error) {
	if c.rt.fnUndefined == nil {
		return nil, fmt.Errorf("undefined function not available")
	}

	results, err := c.rt.fnUndefined.Call(ctx, uint64(c.handle))
	if err != nil {
		return nil, err
	}

	valuePtr := uint32(results[0])
	if valuePtr == 0 {
		return nil, fmt.Errorf("failed to create undefined")
	}

	return &Value{ctx: c, handle: valuePtr}, nil
}

// Object creates an empty object.
func (c *Context) Object(ctx context.Context) (*Value, error) {
	if c.rt.fnObject == nil {
		return nil, fmt.Errorf("object function not available")
	}

	results, err := c.rt.fnObject.Call(ctx, uint64(c.handle))
	if err != nil {
		return nil, err
	}

	valuePtr := uint32(results[0])
	if valuePtr == 0 {
		return nil, fmt.Errorf("failed to create object")
	}

	return &Value{ctx: c, handle: valuePtr}, nil
}

// Array creates an empty array.
func (c *Context) Array(ctx context.Context) (*Value, error) {
	if c.rt.fnArray == nil {
		return nil, fmt.Errorf("array function not available")
	}

	results, err := c.rt.fnArray.Call(ctx, uint64(c.handle))
	if err != nil {
		return nil, err
	}

	valuePtr := uint32(results[0])
	if valuePtr == 0 {
		return nil, fmt.Errorf("failed to create array")
	}

	return &Value{ctx: c, handle: valuePtr}, nil
}

// JSONStringify converts a value to JSON string.
func (c *Context) JSONStringify(ctx context.Context, value *Value) (string, error) {
	if c.rt.fnJSONStringify == nil {
		return "", fmt.Errorf("json_stringify function not available")
	}

	results, err := c.rt.fnJSONStringify.Call(ctx, uint64(c.handle), uint64(value.handle))
	if err != nil {
		return "", err
	}

	strPtr := uint32(results[0])
	errorPtr := uint32(results[1])

	if errorPtr != 0 {
		return "", fmt.Errorf("json_stringify error: %s", c.rt.readString(errorPtr))
	}

	if strPtr == 0 {
		return "", nil
	}

	str := c.rt.readString(strPtr)

	// Free the allocated string
	if c.rt.fnFreeString != nil {
		c.rt.fnFreeString.Call(ctx, uint64(strPtr))
	}

	return str, nil
}

// JSONParse parses a JSON string into a value.
func (c *Context) JSONParse(ctx context.Context, json string) (*Value, error) {
	if c.rt.fnJSONParse == nil {
		return nil, fmt.Errorf("json_parse function not available")
	}

	jsonPtr, err := c.rt.allocString(ctx, json)
	if err != nil {
		return nil, err
	}
	defer c.rt.deallocString(ctx, jsonPtr, uint32(len(json)+1))

	// TsRunValueResult: { value: *TsRunValue (4 bytes), error: *c_char (4 bytes) } = 8 bytes
	const resultSize = 8
	resultPtr, err := c.rt.allocResult(ctx, resultSize)
	if err != nil {
		return nil, fmt.Errorf("failed to allocate result: %w", err)
	}
	defer c.rt.deallocResult(ctx, resultPtr, resultSize)

	// Call with sret convention: (sret, ctx, json)
	_, err = c.rt.fnJSONParse.Call(ctx, uint64(resultPtr), uint64(c.handle), uint64(jsonPtr))
	if err != nil {
		return nil, err
	}

	// Read TsRunValueResult from memory
	valuePtr, _ := c.rt.memory.ReadUint32Le(resultPtr)
	errorPtr, _ := c.rt.memory.ReadUint32Le(resultPtr + 4)

	if errorPtr != 0 {
		return nil, fmt.Errorf("json_parse error: %s", c.rt.readString(errorPtr))
	}

	if valuePtr == 0 {
		return nil, fmt.Errorf("json_parse returned null")
	}

	return &Value{ctx: c, handle: valuePtr}, nil
}
