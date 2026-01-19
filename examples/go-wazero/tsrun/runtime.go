package tsrun

import (
	"context"
	_ "embed"
	"encoding/binary"
	"fmt"
	"math/rand"
	"os"
	"regexp"
	"sync"
	"time"

	"github.com/tetratelabs/wazero"
	"github.com/tetratelabs/wazero/api"
)

//go:embed tsrun.wasm
var wasmBytes []byte

// Runtime represents a tsrun WASM runtime instance.
type Runtime struct {
	runtime wazero.Runtime
	module  api.Module
	memory  api.Memory

	// Exported WASM functions
	fnNew            api.Function
	fnFree           api.Function
	fnPrepare        api.Function
	fnStep           api.Function
	fnRun            api.Function
	fnStepResultFree api.Function

	// Value functions
	fnValueFree     api.Function
	fnNumber        api.Function
	fnString        api.Function
	fnBoolean       api.Function
	fnNull          api.Function
	fnUndefined     api.Function
	fnObject        api.Function
	fnArray         api.Function
	fnGetType       api.Function
	fnGetNumber     api.Function
	fnGetString     api.Function
	fnGetBool       api.Function
	fnIsNull        api.Function
	fnIsUndefined   api.Function
	fnIsArray       api.Function
	fnIsFunction    api.Function
	fnGet           api.Function
	fnSet           api.Function
	fnDelete        api.Function
	fnHas           api.Function
	fnKeys          api.Function
	fnArrayLength   api.Function
	fnArrayGet      api.Function
	fnArraySet      api.Function
	fnArrayPush     api.Function
	fnJSONStringify api.Function
	fnJSONParse     api.Function
	fnFreeString    api.Function
	fnFreeStrings   api.Function

	// Module functions
	fnProvideModule api.Function
	fnGetImports    api.Function

	// Order functions
	fnCreatePendingOrder  api.Function
	fnFulfillOrders       api.Function
	fnCreateOrderPromise  api.Function
	fnResolvePromise      api.Function
	fnRejectPromise       api.Function

	// Native function support
	fnNativeFunction api.Function

	// Memory allocation
	fnAlloc   api.Function
	fnDealloc api.Function

	// Console callback
	consoleCallback func(level ConsoleLevel, message string)
	consoleMu       sync.Mutex

	// Regex handles
	regexHandles   map[uint32]*regexp.Regexp
	nextRegexHandle uint32
	regexMu        sync.Mutex
}

// ConsoleOption sets a console callback function.
func ConsoleOption(callback func(level ConsoleLevel, message string)) func(*Runtime) {
	return func(r *Runtime) {
		r.consoleCallback = callback
	}
}

// New creates a new tsrun runtime.
func New(ctx context.Context, opts ...func(*Runtime)) (*Runtime, error) {
	r := &Runtime{
		regexHandles:    make(map[uint32]*regexp.Regexp),
		nextRegexHandle: 1,
	}

	// Apply options
	for _, opt := range opts {
		opt(r)
	}

	// Create wazero runtime
	r.runtime = wazero.NewRuntime(ctx)

	// Define host imports before instantiating WASM
	if _, err := r.defineHostImports(ctx); err != nil {
		r.runtime.Close(ctx)
		return nil, fmt.Errorf("failed to define host imports: %w", err)
	}

	// Instantiate the WASM module
	module, err := r.runtime.Instantiate(ctx, wasmBytes)
	if err != nil {
		r.runtime.Close(ctx)
		return nil, fmt.Errorf("failed to instantiate WASM module: %w", err)
	}
	r.module = module
	r.memory = module.Memory()

	// Get exported functions
	if err := r.getExportedFunctions(); err != nil {
		r.runtime.Close(ctx)
		return nil, fmt.Errorf("failed to get exported functions: %w", err)
	}

	return r, nil
}

// Close releases resources used by the runtime.
func (r *Runtime) Close(ctx context.Context) error {
	if r.runtime != nil {
		return r.runtime.Close(ctx)
	}
	return nil
}

// defineHostImports sets up the tsrun_host module with host functions.
func (r *Runtime) defineHostImports(ctx context.Context) (api.Module, error) {
	return r.runtime.NewHostModuleBuilder("tsrun_host").
		NewFunctionBuilder().
		WithFunc(r.hostTimeNow).
		Export("host_time_now").
		NewFunctionBuilder().
		WithFunc(r.hostTimeStartTimer).
		Export("host_time_start_timer").
		NewFunctionBuilder().
		WithFunc(r.hostTimeElapsed).
		Export("host_time_elapsed").
		NewFunctionBuilder().
		WithFunc(r.hostRandom).
		Export("host_random").
		NewFunctionBuilder().
		WithFunc(r.hostConsoleWrite).
		Export("host_console_write").
		NewFunctionBuilder().
		WithFunc(r.hostConsoleClear).
		Export("host_console_clear").
		// Regex host functions
		NewFunctionBuilder().
		WithFunc(r.hostRegexCompile).
		Export("host_regex_compile").
		NewFunctionBuilder().
		WithFunc(r.hostRegexFree).
		Export("host_regex_free").
		NewFunctionBuilder().
		WithFunc(r.hostRegexTest).
		Export("host_regex_test").
		NewFunctionBuilder().
		WithFunc(r.hostRegexExec).
		Export("host_regex_exec").
		NewFunctionBuilder().
		WithFunc(r.hostFreeCaptures).
		Export("host_free_captures").
		NewFunctionBuilder().
		WithFunc(r.hostRegexReplace).
		Export("host_regex_replace").
		NewFunctionBuilder().
		WithFunc(r.hostRegexSplit).
		Export("host_regex_split").
		NewFunctionBuilder().
		WithFunc(r.hostFreeSplitResult).
		Export("host_free_split_result").
		NewFunctionBuilder().
		WithFunc(r.hostFreeString).
		Export("host_free_string").
		Instantiate(ctx)
}

// Host function implementations

func (r *Runtime) hostTimeNow(ctx context.Context) int64 {
	return time.Now().UnixMilli()
}

func (r *Runtime) hostTimeStartTimer(ctx context.Context) uint64 {
	return uint64(time.Now().UnixNano())
}

func (r *Runtime) hostTimeElapsed(ctx context.Context, start uint64) uint64 {
	elapsed := time.Now().UnixNano() - int64(start)
	return uint64(elapsed / 1_000_000) // Convert to milliseconds
}

func (r *Runtime) hostRandom(ctx context.Context) float64 {
	return rand.Float64()
}

func (r *Runtime) hostConsoleWrite(ctx context.Context, m api.Module, level uint32, ptr uint32, length uint32) {
	data, ok := m.Memory().Read(ptr, length)
	if !ok {
		return
	}
	message := string(data)

	r.consoleMu.Lock()
	callback := r.consoleCallback
	r.consoleMu.Unlock()

	if callback != nil {
		callback(ConsoleLevel(level), message)
	} else {
		// Default: print to stdout/stderr
		switch ConsoleLevel(level) {
		case ConsoleLevelWarn, ConsoleLevelError:
			fmt.Fprintln(os.Stderr, message)
		default:
			fmt.Println(message)
		}
	}
}

func (r *Runtime) hostConsoleClear(ctx context.Context) {
	// ANSI escape code to clear screen
	fmt.Print("\033[2J\033[H")
}

// Regex host functions

func (r *Runtime) hostRegexCompile(ctx context.Context, m api.Module, patternPtr, patternLen, flagsPtr, flagsLen, errorPtrOut, errorLenOut uint32) uint32 {
	pattern, ok := m.Memory().Read(patternPtr, patternLen)
	if !ok {
		return 0
	}
	flags, ok := m.Memory().Read(flagsPtr, flagsLen)
	if !ok {
		return 0
	}

	// Convert JS regex flags to Go regex
	// Go doesn't support all JS flags, we'll handle 'i' (case insensitive)
	goPattern := string(pattern)
	flagStr := string(flags)
	if len(flagStr) > 0 {
		prefix := "(?"
		if contains(flagStr, 'i') {
			prefix += "i"
		}
		if contains(flagStr, 'm') {
			prefix += "m"
		}
		if contains(flagStr, 's') {
			prefix += "s"
		}
		if prefix != "(?" {
			goPattern = prefix + ")" + goPattern
		}
	}

	re, err := regexp.Compile(goPattern)
	if err != nil {
		// Write error message to WASM memory
		errMsg := []byte(err.Error())
		errPtr, _ := r.fnAlloc.Call(ctx, uint64(len(errMsg)))
		if errPtr[0] != 0 {
			m.Memory().Write(uint32(errPtr[0]), errMsg)
			m.Memory().WriteUint32Le(errorPtrOut, uint32(errPtr[0]))
			m.Memory().WriteUint32Le(errorLenOut, uint32(len(errMsg)))
		}
		return 0
	}

	r.regexMu.Lock()
	handle := r.nextRegexHandle
	r.nextRegexHandle++
	r.regexHandles[handle] = re
	r.regexMu.Unlock()

	return handle
}

func contains(s string, c byte) bool {
	for i := 0; i < len(s); i++ {
		if s[i] == c {
			return true
		}
	}
	return false
}

func (r *Runtime) hostRegexFree(ctx context.Context, handle uint32) {
	r.regexMu.Lock()
	delete(r.regexHandles, handle)
	r.regexMu.Unlock()
}

func (r *Runtime) hostRegexTest(ctx context.Context, m api.Module, handle, inputPtr, inputLen uint32) int32 {
	r.regexMu.Lock()
	re, ok := r.regexHandles[handle]
	r.regexMu.Unlock()
	if !ok {
		return 0
	}

	input, ok := m.Memory().Read(inputPtr, inputLen)
	if !ok {
		return 0
	}

	if re.Match(input) {
		return 1
	}
	return 0
}

func (r *Runtime) hostRegexExec(ctx context.Context, m api.Module, handle, inputPtr, inputLen, startPos, matchStartOut, matchEndOut, capturesPtrOut, capturesCountOut uint32) int32 {
	r.regexMu.Lock()
	re, ok := r.regexHandles[handle]
	r.regexMu.Unlock()
	if !ok {
		return 0
	}

	input, ok := m.Memory().Read(inputPtr, inputLen)
	if !ok {
		return 0
	}

	// Search from startPos
	searchInput := input[startPos:]
	loc := re.FindSubmatchIndex(searchInput)
	if loc == nil {
		return 0
	}

	// Adjust positions by startPos
	matchStart := uint32(loc[0]) + startPos
	matchEnd := uint32(loc[1]) + startPos

	// Allocate captures array (pairs of i32)
	capturesCount := uint32(len(loc) / 2)
	capturesBytes := capturesCount * 2 * 4
	capturesPtr, _ := r.fnAlloc.Call(ctx, uint64(capturesBytes))
	if capturesPtr[0] == 0 {
		return 0
	}

	// Write captures
	for i := uint32(0); i < capturesCount; i++ {
		start := loc[i*2]
		end := loc[i*2+1]
		offset := uint32(capturesPtr[0]) + i*8
		if start >= 0 && end >= 0 {
			m.Memory().WriteUint32Le(offset, uint32(start)+startPos)
			m.Memory().WriteUint32Le(offset+4, uint32(end)+startPos)
		} else {
			// Non-participating group
			buf := make([]byte, 4)
			binary.LittleEndian.PutUint32(buf, 0xFFFFFFFF) // -1 as i32
			m.Memory().Write(offset, buf)
			m.Memory().Write(offset+4, buf)
		}
	}

	// Write output params
	m.Memory().WriteUint32Le(matchStartOut, matchStart)
	m.Memory().WriteUint32Le(matchEndOut, matchEnd)
	m.Memory().WriteUint32Le(capturesPtrOut, uint32(capturesPtr[0]))
	m.Memory().WriteUint32Le(capturesCountOut, capturesCount)

	return 1
}

func (r *Runtime) hostFreeCaptures(ctx context.Context, ptr, count uint32) {
	if ptr != 0 && count > 0 {
		r.fnDealloc.Call(ctx, uint64(ptr), uint64(count*2*4))
	}
}

func (r *Runtime) hostRegexReplace(ctx context.Context, m api.Module, handle, inputPtr, inputLen, replPtr, replLen, global, resultPtrOut, resultLenOut uint32) int32 {
	r.regexMu.Lock()
	re, ok := r.regexHandles[handle]
	r.regexMu.Unlock()
	if !ok {
		return 0
	}

	input, ok := m.Memory().Read(inputPtr, inputLen)
	if !ok {
		return 0
	}
	repl, ok := m.Memory().Read(replPtr, replLen)
	if !ok {
		return 0
	}

	var result []byte
	if global != 0 {
		result = re.ReplaceAll(input, repl)
	} else {
		// Replace first match only
		loc := re.FindIndex(input)
		if loc == nil {
			result = input
		} else {
			result = append(result, input[:loc[0]]...)
			result = append(result, repl...)
			result = append(result, input[loc[1]:]...)
		}
	}

	// Allocate result string
	resultPtr, _ := r.fnAlloc.Call(ctx, uint64(len(result)))
	if resultPtr[0] == 0 {
		return 0
	}
	m.Memory().Write(uint32(resultPtr[0]), result)
	m.Memory().WriteUint32Le(resultPtrOut, uint32(resultPtr[0]))
	m.Memory().WriteUint32Le(resultLenOut, uint32(len(result)))

	return 1
}

func (r *Runtime) hostRegexSplit(ctx context.Context, m api.Module, handle, inputPtr, inputLen, partsPtrOut, partsCountOut uint32) int32 {
	r.regexMu.Lock()
	re, ok := r.regexHandles[handle]
	r.regexMu.Unlock()
	if !ok {
		return 0
	}

	input, ok := m.Memory().Read(inputPtr, inputLen)
	if !ok {
		return 0
	}

	parts := re.Split(string(input), -1)
	partsCount := uint32(len(parts))

	// Allocate array of (ptr, len) pairs
	arrayBytes := partsCount * 2 * 4
	arrayPtr, _ := r.fnAlloc.Call(ctx, uint64(arrayBytes))
	if arrayPtr[0] == 0 {
		return 0
	}

	// Allocate each string and write to array
	for i, part := range parts {
		partBytes := []byte(part)
		partLen := len(partBytes)
		if partLen == 0 {
			partLen = 1 // Allocate at least 1 byte
		}
		strPtr, _ := r.fnAlloc.Call(ctx, uint64(partLen))
		if strPtr[0] == 0 {
			return 0
		}
		if len(partBytes) > 0 {
			m.Memory().Write(uint32(strPtr[0]), partBytes)
		}
		offset := uint32(arrayPtr[0]) + uint32(i)*8
		m.Memory().WriteUint32Le(offset, uint32(strPtr[0]))
		m.Memory().WriteUint32Le(offset+4, uint32(len(partBytes)))
	}

	m.Memory().WriteUint32Le(partsPtrOut, uint32(arrayPtr[0]))
	m.Memory().WriteUint32Le(partsCountOut, partsCount)

	return 1
}

func (r *Runtime) hostFreeSplitResult(ctx context.Context, m api.Module, partsPtr, partsCount uint32) {
	if partsPtr == 0 || partsCount == 0 {
		return
	}

	// Free each string
	for i := uint32(0); i < partsCount; i++ {
		strPtr, _ := m.Memory().ReadUint32Le(partsPtr + i*8)
		strLen, _ := m.Memory().ReadUint32Le(partsPtr + i*8 + 4)
		if strPtr != 0 {
			size := strLen
			if size == 0 {
				size = 1
			}
			r.fnDealloc.Call(ctx, uint64(strPtr), uint64(size))
		}
	}

	// Free the array
	r.fnDealloc.Call(ctx, uint64(partsPtr), uint64(partsCount*2*4))
}

func (r *Runtime) hostFreeString(ctx context.Context, ptr, len uint32) {
	if ptr != 0 && len > 0 {
		r.fnDealloc.Call(ctx, uint64(ptr), uint64(len))
	}
}

// getExportedFunctions retrieves references to all exported WASM functions.
func (r *Runtime) getExportedFunctions() error {
	getFunc := func(name string) (api.Function, error) {
		fn := r.module.ExportedFunction(name)
		if fn == nil {
			return nil, fmt.Errorf("function %s not exported", name)
		}
		return fn, nil
	}

	var err error

	// Context lifecycle
	r.fnNew, err = getFunc("tsrun_wasm_new")
	if err != nil {
		return err
	}
	r.fnFree, err = getFunc("tsrun_free")
	if err != nil {
		return err
	}

	// Execution
	r.fnPrepare, err = getFunc("tsrun_prepare")
	if err != nil {
		return err
	}
	r.fnStep, err = getFunc("tsrun_step")
	if err != nil {
		return err
	}
	r.fnRun, err = getFunc("tsrun_run")
	if err != nil {
		return err
	}
	r.fnStepResultFree, err = getFunc("tsrun_step_result_free")
	if err != nil {
		return err
	}

	// Memory allocation
	r.fnAlloc, err = getFunc("tsrun_alloc")
	if err != nil {
		return err
	}
	r.fnDealloc, err = getFunc("tsrun_dealloc")
	if err != nil {
		return err
	}

	// Value functions (optional - may not all be present)
	r.fnValueFree = r.module.ExportedFunction("tsrun_value_free")
	r.fnNumber = r.module.ExportedFunction("tsrun_number")
	r.fnString = r.module.ExportedFunction("tsrun_string")
	r.fnBoolean = r.module.ExportedFunction("tsrun_boolean")
	r.fnNull = r.module.ExportedFunction("tsrun_null")
	r.fnUndefined = r.module.ExportedFunction("tsrun_undefined")
	r.fnObject = r.module.ExportedFunction("tsrun_object")
	r.fnArray = r.module.ExportedFunction("tsrun_array")
	r.fnGetType = r.module.ExportedFunction("tsrun_get_type")
	r.fnGetNumber = r.module.ExportedFunction("tsrun_get_number")
	r.fnGetString = r.module.ExportedFunction("tsrun_get_string")
	r.fnGetBool = r.module.ExportedFunction("tsrun_get_bool")
	r.fnIsNull = r.module.ExportedFunction("tsrun_is_null")
	r.fnIsUndefined = r.module.ExportedFunction("tsrun_is_undefined")
	r.fnIsArray = r.module.ExportedFunction("tsrun_is_array")
	r.fnIsFunction = r.module.ExportedFunction("tsrun_is_function")
	r.fnGet = r.module.ExportedFunction("tsrun_get")
	r.fnSet = r.module.ExportedFunction("tsrun_set")
	r.fnDelete = r.module.ExportedFunction("tsrun_delete")
	r.fnHas = r.module.ExportedFunction("tsrun_has")
	r.fnKeys = r.module.ExportedFunction("tsrun_keys")
	r.fnArrayLength = r.module.ExportedFunction("tsrun_array_length")
	r.fnArrayGet = r.module.ExportedFunction("tsrun_array_get")
	r.fnArraySet = r.module.ExportedFunction("tsrun_array_set")
	r.fnArrayPush = r.module.ExportedFunction("tsrun_array_push")
	r.fnJSONStringify = r.module.ExportedFunction("tsrun_json_stringify")
	r.fnJSONParse = r.module.ExportedFunction("tsrun_json_parse")
	r.fnFreeString = r.module.ExportedFunction("tsrun_free_string")
	r.fnFreeStrings = r.module.ExportedFunction("tsrun_free_strings")

	// Module functions
	r.fnProvideModule = r.module.ExportedFunction("tsrun_provide_module")
	r.fnGetImports = r.module.ExportedFunction("tsrun_get_imports")

	// Order functions
	r.fnCreatePendingOrder = r.module.ExportedFunction("tsrun_create_pending_order")
	r.fnFulfillOrders = r.module.ExportedFunction("tsrun_fulfill_orders")
	r.fnCreateOrderPromise = r.module.ExportedFunction("tsrun_create_order_promise")
	r.fnResolvePromise = r.module.ExportedFunction("tsrun_resolve_promise")
	r.fnRejectPromise = r.module.ExportedFunction("tsrun_reject_promise")

	// Native function support
	r.fnNativeFunction = r.module.ExportedFunction("tsrun_native_function")

	return nil
}

// SetConsoleCallback sets a callback for console output.
func (r *Runtime) SetConsoleCallback(callback func(level ConsoleLevel, message string)) {
	r.consoleMu.Lock()
	defer r.consoleMu.Unlock()
	r.consoleCallback = callback
}
