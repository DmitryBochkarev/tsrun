package tsrun

import (
	"context"
	_ "embed"
	"fmt"
	"math/rand"
	"os"
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
}

// ConsoleOption sets a console callback function.
func ConsoleOption(callback func(level ConsoleLevel, message string)) func(*Runtime) {
	return func(r *Runtime) {
		r.consoleCallback = callback
	}
}

// New creates a new tsrun runtime.
func New(ctx context.Context, opts ...func(*Runtime)) (*Runtime, error) {
	r := &Runtime{}

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
