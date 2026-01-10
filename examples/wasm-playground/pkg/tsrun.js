// tsrun Raw WASM API Wrapper
//
// This module provides a JavaScript wrapper around the raw WASM FFI exports.
// It implements the host functions and provides a TsRunner-like API.

// Step status constants (matching TsRunStepStatus in Rust)
export const STEP_CONTINUE = 0;
export const STEP_COMPLETE = 1;
export const STEP_NEED_IMPORTS = 2;
export const STEP_SUSPENDED = 3;
export const STEP_DONE = 4;
export const STEP_ERROR = 5;

// Value type constants (matching TsRunType in Rust)
export const TYPE_UNDEFINED = 0;
export const TYPE_NULL = 1;
export const TYPE_BOOLEAN = 2;
export const TYPE_NUMBER = 3;
export const TYPE_STRING = 4;
export const TYPE_OBJECT = 5;
export const TYPE_SYMBOL = 6;

// Console level constants
export const CONSOLE_LOG = 0;
export const CONSOLE_INFO = 1;
export const CONSOLE_DEBUG = 2;
export const CONSOLE_WARN = 3;
export const CONSOLE_ERROR = 4;

// Private symbols for internal state
const _wasm = Symbol('wasm');
const _memory = Symbol('memory');
const _context = Symbol('context');
const _consoleBuffer = Symbol('consoleBuffer');
const _pendingOrders = Symbol('pendingOrders');
const _importRequests = Symbol('importRequests');

/**
 * Initialize the tsrun WASM module.
 * @param {string|URL|Request} [wasmPath] - Path to the WASM file (defaults to 'tsrun.wasm')
 * @returns {Promise<typeof TsRunner>} - The TsRunner class ready to use
 */
export async function init(wasmPath = 'tsrun.wasm') {
    // Performance timer state (per-instance would be better but simplified here)
    const timerStart = performance.now();

    // Console buffer for current TsRunner instance (set during instantiation)
    let activeConsoleBuffer = null;

    // Host imports that the WASM module expects
    const hostImports = {
        tsrun_host: {
            // Get current time in milliseconds since Unix epoch
            host_time_now() {
                return BigInt(Date.now());
            },

            // Start a performance timer (returns opaque handle)
            host_time_start_timer() {
                return BigInt(Math.floor(performance.now() * 1e6)); // Convert to nanoseconds as u64
            },

            // Get elapsed milliseconds since timer start
            host_time_elapsed(start) {
                const startMs = Number(start) / 1e6;
                const elapsed = performance.now() - startMs;
                return BigInt(Math.floor(elapsed));
            },

            // Generate random float in [0, 1)
            host_random() {
                return Math.random();
            },

            // Write to console
            host_console_write(level, ptr, len) {
                if (!wasmInstance) return;

                const memory = wasmInstance.exports.memory;
                const bytes = new Uint8Array(memory.buffer, ptr, len);
                const message = textDecoder.decode(bytes);

                const levelName = ['log', 'info', 'debug', 'warn', 'error'][level] || 'log';

                // Always log to browser console for debugging
                console[levelName]('[WASM]', message);

                // Also buffer for result if buffer is active
                if (activeConsoleBuffer) {
                    activeConsoleBuffer.push({ level: levelName, message });
                }
            },

            // Clear console
            host_console_clear() {
                if (activeConsoleBuffer) {
                    activeConsoleBuffer.push({ level: 'clear', message: '--- Console cleared ---' });
                }
            }
        }
    };

    // Load and instantiate the WASM module
    const wasmResponse = await fetch(wasmPath);
    const wasmBytes = await wasmResponse.arrayBuffer();
    const { instance } = await WebAssembly.instantiate(wasmBytes, hostImports);

    const wasmInstance = instance;
    const textEncoder = new TextEncoder();
    const textDecoder = new TextDecoder();

    /**
     * Allocate memory in WASM and write a string to it.
     * @param {string} str - The string to allocate
     * @returns {{ptr: number, len: number}} - Pointer and length (excluding null terminator)
     */
    function allocString(str) {
        const bytes = textEncoder.encode(str);
        const len = bytes.length;
        const ptr = wasmInstance.exports.tsrun_alloc(len + 1); // +1 for null terminator
        if (ptr === 0) throw new Error('Failed to allocate memory for string');

        const memory = new Uint8Array(wasmInstance.exports.memory.buffer);
        memory.set(bytes, ptr);
        memory[ptr + len] = 0; // null terminator

        return { ptr, len };
    }

    /**
     * Deallocate memory in WASM.
     * @param {number} ptr - Pointer to free
     * @param {number} size - Size of allocation
     */
    function deallocString(ptr, size) {
        if (ptr !== 0 && size > 0) {
            wasmInstance.exports.tsrun_dealloc(ptr, size);
        }
    }

    /**
     * Read a null-terminated string from WASM memory.
     * @param {number} ptr - Pointer to string
     * @returns {string}
     */
    function readString(ptr) {
        if (ptr === 0) return '';

        const memory = new Uint8Array(wasmInstance.exports.memory.buffer);
        let end = ptr;
        while (memory[end] !== 0) end++;

        return textDecoder.decode(memory.slice(ptr, end));
    }

    /**
     * Read a DataView from WASM memory.
     * @param {number} ptr - Start pointer
     * @param {number} len - Length in bytes
     * @returns {DataView}
     */
    function getDataView(ptr, len) {
        return new DataView(wasmInstance.exports.memory.buffer, ptr, len);
    }

    /**
     * TsRunner class - wraps the raw WASM FFI.
     */
    class TsRunner {
        constructor() {
            this[_wasm] = wasmInstance;
            this[_memory] = wasmInstance.exports.memory;
            this[_consoleBuffer] = [];
            this[_pendingOrders] = [];
            this[_importRequests] = [];

            // Create interpreter context using WASM-specific constructor
            this[_context] = wasmInstance.exports.tsrun_wasm_new();
            if (this[_context] === 0) {
                throw new Error('Failed to create tsrun context');
            }
        }

        /**
         * Free the context and release resources.
         */
        free() {
            if (this[_context] !== 0) {
                this[_wasm].exports.tsrun_free(this[_context]);
                this[_context] = 0;
            }
        }

        /**
         * Prepare code for execution.
         * @param {string} code - TypeScript/JavaScript source code
         * @param {string} [filename] - Optional filename for error messages
         * @returns {{status: number, error?: string, console_output: Array}}
         */
        prepare(code, filename = 'script.ts') {
            // Set active console buffer for host callbacks
            activeConsoleBuffer = this[_consoleBuffer];
            this[_consoleBuffer] = [];
            this[_pendingOrders] = [];
            this[_importRequests] = [];

            try {
                const codeAlloc = allocString(code);
                const filenameAlloc = filename ? allocString(filename) : { ptr: 0, len: 0 };

                // Allocate result struct: TsRunResult = { ok: i32, error: i32 } = 8 bytes
                const resultPtr = this[_wasm].exports.tsrun_alloc(8);
                if (resultPtr === 0) throw new Error('Failed to allocate result memory');

                try {
                    // Call tsrun_prepare(sret, ctx, code, path)
                    this[_wasm].exports.tsrun_prepare(resultPtr, this[_context], codeAlloc.ptr, filenameAlloc.ptr);

                    // Parse result
                    const view = getDataView(resultPtr, 8);
                    const ok = view.getUint32(0, true);
                    const errorPtr = view.getUint32(4, true);

                    if (ok === 0) {
                        const error = readString(errorPtr);
                        return {
                            status: STEP_ERROR,
                            error: `Parse error: ${error}`,
                            console_output: this[_consoleBuffer].splice(0)
                        };
                    }

                    return {
                        status: STEP_CONTINUE,
                        console_output: this[_consoleBuffer].splice(0)
                    };
                } finally {
                    this[_wasm].exports.tsrun_dealloc(resultPtr, 8);
                    deallocString(codeAlloc.ptr, codeAlloc.len + 1);
                    if (filenameAlloc.ptr) deallocString(filenameAlloc.ptr, filenameAlloc.len + 1);
                }
            } finally {
                activeConsoleBuffer = null;
            }
        }

        /**
         * Execute one step.
         * @returns {{status: number, value_handle?: number, error?: string, console_output: Array}}
         */
        step() {
            activeConsoleBuffer = this[_consoleBuffer];

            try {
                // Allocate TsRunStepResult: 36 bytes
                const resultPtr = this[_wasm].exports.tsrun_alloc(36);
                if (resultPtr === 0) throw new Error('Failed to allocate step result memory');

                try {
                    this[_wasm].exports.tsrun_step(resultPtr, this[_context]);
                    return this._parseStepResult(resultPtr);
                } finally {
                    // Free internal arrays but not value
                    this[_wasm].exports.tsrun_step_result_free(resultPtr);
                    this[_wasm].exports.tsrun_dealloc(resultPtr, 36);
                }
            } finally {
                activeConsoleBuffer = null;
            }
        }

        /**
         * Execute until completion, needing imports, or suspension.
         * @returns {{status: number, value_handle?: number, error?: string, console_output: Array}}
         */
        run() {
            activeConsoleBuffer = this[_consoleBuffer];

            try {
                const resultPtr = this[_wasm].exports.tsrun_alloc(36);
                if (resultPtr === 0) throw new Error('Failed to allocate run result memory');

                try {
                    this[_wasm].exports.tsrun_run(resultPtr, this[_context]);
                    return this._parseStepResult(resultPtr);
                } finally {
                    this[_wasm].exports.tsrun_step_result_free(resultPtr);
                    this[_wasm].exports.tsrun_dealloc(resultPtr, 36);
                }
            } finally {
                activeConsoleBuffer = null;
            }
        }

        /**
         * Parse TsRunStepResult from memory.
         * @private
         */
        _parseStepResult(resultPtr) {
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

            const view = getDataView(resultPtr, 36);
            const status = view.getUint32(0, true);
            const valuePtr = view.getUint32(4, true);
            const importsPtr = view.getUint32(8, true);
            const importCount = view.getUint32(12, true);
            const pendingPtr = view.getUint32(16, true);
            const pendingCount = view.getUint32(20, true);
            const cancelledPtr = view.getUint32(24, true);
            const cancelledCount = view.getUint32(28, true);
            const errorPtr = view.getUint32(32, true);

            const result = {
                status,
                value_handle: 0,
                console_output: this[_consoleBuffer].splice(0)
            };

            switch (status) {
                case STEP_COMPLETE:
                    result.value_handle = valuePtr;
                    break;

                case STEP_ERROR:
                    result.error = readString(errorPtr);
                    break;

                case STEP_NEED_IMPORTS:
                    this[_importRequests] = this._parseImportRequests(importsPtr, importCount);
                    break;

                case STEP_SUSPENDED:
                    this[_pendingOrders] = this._parsePendingOrders(pendingPtr, pendingCount);
                    break;
            }

            return result;
        }

        /**
         * Parse import requests from memory.
         * @private
         */
        _parseImportRequests(ptr, count) {
            if (ptr === 0 || count === 0) return [];

            // TsRunImportRequest: { specifier: i32, resolved_path: i32, importer: i32 } = 12 bytes
            const requests = [];
            for (let i = 0; i < count; i++) {
                const view = getDataView(ptr + i * 12, 12);
                const specifierPtr = view.getUint32(0, true);
                const resolvedPtr = view.getUint32(4, true);
                const importerPtr = view.getUint32(8, true);

                requests.push({
                    specifier: readString(specifierPtr),
                    resolved_path: readString(resolvedPtr),
                    importer: readString(importerPtr)
                });
            }
            return requests;
        }

        /**
         * Parse pending orders from memory.
         * @private
         */
        _parsePendingOrders(ptr, count) {
            if (ptr === 0 || count === 0) return [];

            // TsRunOrder: { id: u64, payload: i32 } = 12 bytes (8 + 4 on wasm32)
            const orders = [];
            for (let i = 0; i < count; i++) {
                const view = getDataView(ptr + i * 12, 12);
                const id = view.getBigUint64(0, true);
                const payloadPtr = view.getUint32(8, true);

                orders.push({
                    id: Number(id),
                    payload_handle: payloadPtr
                });
            }
            return orders;
        }

        // ═══════════════════════════════════════════════════════════════════════════════
        // Order/Import API
        // ═══════════════════════════════════════════════════════════════════════════════

        /**
         * Get pending order IDs (after STEP_SUSPENDED).
         * @returns {number[]}
         */
        get_pending_order_ids() {
            return this[_pendingOrders].map(o => o.id);
        }

        /**
         * Get the payload handle for a pending order.
         * @param {number} orderId
         * @returns {number}
         */
        get_order_payload(orderId) {
            const order = this[_pendingOrders].find(o => o.id === orderId);
            return order ? order.payload_handle : 0;
        }

        /**
         * Get import request specifiers (after STEP_NEED_IMPORTS).
         * @returns {string[]}
         */
        get_import_requests() {
            return this[_importRequests].map(r => r.specifier);
        }

        /**
         * Fulfill orders with values.
         * @param {Array<{id: number, value?: number, error?: string}>} responses
         */
        fulfill_orders(responses) {
            // Build array of TsRunOrderResponse structs
            // TsRunOrderResponse: { id: u64, value: i32, error: i32 } = 16 bytes
            const count = responses.length;
            if (count === 0) return;

            const arrayPtr = this[_wasm].exports.tsrun_alloc(count * 16);
            if (arrayPtr === 0) throw new Error('Failed to allocate order responses');

            const allocatedErrors = [];

            try {
                for (let i = 0; i < count; i++) {
                    const resp = responses[i];
                    const offset = arrayPtr + i * 16;

                    // Get fresh DataView after any potential memory growth
                    const memory = new DataView(this[_wasm].exports.memory.buffer);

                    // Write id as u64
                    memory.setBigUint64(offset, BigInt(resp.id), true);

                    // Write value pointer
                    memory.setUint32(offset + 8, resp.value || 0, true);

                    // Write error pointer
                    let errorPtr = 0;
                    if (resp.error) {
                        const alloc = allocString(resp.error);
                        errorPtr = alloc.ptr;
                        allocatedErrors.push(alloc);
                    }
                    // Get fresh DataView after potential memory growth from allocString
                    new DataView(this[_wasm].exports.memory.buffer).setUint32(offset + 12, errorPtr, true);
                }

                // Allocate result struct for sret
                const resultPtr = this[_wasm].exports.tsrun_alloc(8);
                if (resultPtr === 0) throw new Error('Failed to allocate result');

                try {
                    this[_wasm].exports.tsrun_fulfill_orders(resultPtr, this[_context], arrayPtr, count);

                    // Check result
                    const view = getDataView(resultPtr, 8);
                    const ok = view.getUint32(0, true);
                    if (ok === 0) {
                        const errPtr = view.getUint32(4, true);
                        throw new Error(`fulfill_orders failed: ${readString(errPtr)}`);
                    }
                } finally {
                    this[_wasm].exports.tsrun_dealloc(resultPtr, 8);
                }
            } finally {
                this[_wasm].exports.tsrun_dealloc(arrayPtr, count * 16);
                for (const alloc of allocatedErrors) {
                    deallocString(alloc.ptr, alloc.len + 1);
                }
            }

            this[_pendingOrders] = [];
        }

        /**
         * Set result for a single order (queued until commit_fulfillments).
         * @param {number} orderId
         * @param {number} resultHandle
         */
        set_order_result(orderId, resultHandle) {
            this._queuedFulfillments = this._queuedFulfillments || [];
            this._queuedFulfillments.push({ id: orderId, value: resultHandle });
        }

        /**
         * Set error for a single order (queued until commit_fulfillments).
         * @param {number} orderId
         * @param {string} errorMsg
         */
        set_order_error(orderId, errorMsg) {
            this._queuedFulfillments = this._queuedFulfillments || [];
            this._queuedFulfillments.push({ id: orderId, error: errorMsg });
        }

        /**
         * Commit queued fulfillments.
         */
        commit_fulfillments() {
            if (!this._queuedFulfillments || this._queuedFulfillments.length === 0) return;
            this.fulfill_orders(this._queuedFulfillments);
            this._queuedFulfillments = [];
        }

        /**
         * Provide a module source.
         * @param {string} path - Module path
         * @param {string} source - Module source code
         */
        provide_module(path, source) {
            const pathAlloc = allocString(path);
            const sourceAlloc = allocString(source);
            const resultPtr = this[_wasm].exports.tsrun_alloc(8);

            try {
                this[_wasm].exports.tsrun_provide_module(resultPtr, this[_context], pathAlloc.ptr, sourceAlloc.ptr);

                const view = getDataView(resultPtr, 8);
                const ok = view.getUint32(0, true);
                if (ok === 0) {
                    const errPtr = view.getUint32(4, true);
                    throw new Error(`provide_module failed: ${readString(errPtr)}`);
                }
            } finally {
                this[_wasm].exports.tsrun_dealloc(resultPtr, 8);
                deallocString(pathAlloc.ptr, pathAlloc.len + 1);
                deallocString(sourceAlloc.ptr, sourceAlloc.len + 1);
            }
        }

        // ═══════════════════════════════════════════════════════════════════════════════
        // Promise API
        // ═══════════════════════════════════════════════════════════════════════════════

        /**
         * Create an unresolved Promise.
         * @returns {number} Promise handle
         */
        create_promise() {
            // Allocate result struct: TsRunValueResult = { value: i32, error: i32 } = 8 bytes
            const resultPtr = this[_wasm].exports.tsrun_alloc(8);
            if (resultPtr === 0) throw new Error('Failed to allocate result');

            try {
                this[_wasm].exports.tsrun_create_order_promise(resultPtr, this[_context], BigInt(0));

                const view = getDataView(resultPtr, 8);
                const valuePtr = view.getUint32(0, true);
                const errorPtr = view.getUint32(4, true);

                if (valuePtr === 0 && errorPtr !== 0) {
                    throw new Error(`create_promise failed: ${readString(errorPtr)}`);
                }

                return valuePtr;
            } finally {
                this[_wasm].exports.tsrun_dealloc(resultPtr, 8);
            }
        }

        /**
         * Resolve a Promise with a value.
         * @param {number} promiseHandle
         * @param {number} valueHandle
         */
        resolve_promise(promiseHandle, valueHandle) {
            const resultPtr = this[_wasm].exports.tsrun_alloc(8);
            if (resultPtr === 0) throw new Error('Failed to allocate result');

            try {
                this[_wasm].exports.tsrun_resolve_promise(resultPtr, this[_context], promiseHandle, valueHandle);

                const view = getDataView(resultPtr, 8);
                const ok = view.getUint32(0, true);
                if (ok === 0) {
                    const errPtr = view.getUint32(4, true);
                    throw new Error(`resolve_promise failed: ${readString(errPtr)}`);
                }
            } finally {
                this[_wasm].exports.tsrun_dealloc(resultPtr, 8);
            }
        }

        /**
         * Reject a Promise with an error.
         * @param {number} promiseHandle
         * @param {string} errorMsg
         */
        reject_promise(promiseHandle, errorMsg) {
            const errorAlloc = allocString(errorMsg);
            const resultPtr = this[_wasm].exports.tsrun_alloc(8);

            try {
                this[_wasm].exports.tsrun_reject_promise(resultPtr, this[_context], promiseHandle, errorAlloc.ptr);

                const view = getDataView(resultPtr, 8);
                const ok = view.getUint32(0, true);
                if (ok === 0) {
                    const errPtr = view.getUint32(4, true);
                    throw new Error(`reject_promise failed: ${readString(errPtr)}`);
                }
            } finally {
                this[_wasm].exports.tsrun_dealloc(resultPtr, 8);
                deallocString(errorAlloc.ptr, errorAlloc.len + 1);
            }
        }

        // ═══════════════════════════════════════════════════════════════════════════════
        // Value Creation
        // ═══════════════════════════════════════════════════════════════════════════════

        /**
         * Create a number value.
         * @param {number} n
         * @returns {number} Value handle
         */
        create_number(n) {
            return this[_wasm].exports.tsrun_number(this[_context], n);
        }

        /**
         * Create a string value.
         * @param {string} s
         * @returns {number} Value handle
         */
        create_string(s) {
            const alloc = allocString(s);
            try {
                return this[_wasm].exports.tsrun_string(this[_context], alloc.ptr);
            } finally {
                deallocString(alloc.ptr, alloc.len + 1);
            }
        }

        /**
         * Create a boolean value.
         * @param {boolean} b
         * @returns {number} Value handle
         */
        create_bool(b) {
            return this[_wasm].exports.tsrun_boolean(this[_context], b ? 1 : 0);
        }

        /**
         * Create null value.
         * @returns {number} Value handle
         */
        create_null() {
            return this[_wasm].exports.tsrun_null(this[_context]);
        }

        /**
         * Create undefined value.
         * @returns {number} Value handle
         */
        create_undefined() {
            return this[_wasm].exports.tsrun_undefined(this[_context]);
        }

        /**
         * Create an empty object.
         * @returns {number} Value handle
         */
        create_object() {
            const resultPtr = this[_wasm].exports.tsrun_alloc(8);
            if (resultPtr === 0) throw new Error('Failed to allocate result');

            try {
                this[_wasm].exports.tsrun_object_new(resultPtr, this[_context]);
                const view = getDataView(resultPtr, 8);
                return view.getUint32(0, true);
            } finally {
                this[_wasm].exports.tsrun_dealloc(resultPtr, 8);
            }
        }

        /**
         * Create an empty array.
         * @returns {number} Value handle
         */
        create_array() {
            const resultPtr = this[_wasm].exports.tsrun_alloc(8);
            if (resultPtr === 0) throw new Error('Failed to allocate result');

            try {
                this[_wasm].exports.tsrun_array_new(resultPtr, this[_context]);
                const view = getDataView(resultPtr, 8);
                return view.getUint32(0, true);
            } finally {
                this[_wasm].exports.tsrun_dealloc(resultPtr, 8);
            }
        }

        // ═══════════════════════════════════════════════════════════════════════════════
        // Value Inspection
        // ═══════════════════════════════════════════════════════════════════════════════

        /**
         * Get the type of a value.
         * @param {number} handle
         * @returns {string}
         */
        get_value_type(handle) {
            if (handle === 0) return 'undefined';
            const type = this[_wasm].exports.tsrun_typeof(handle);
            return ['undefined', 'null', 'boolean', 'number', 'string', 'object', 'symbol'][type] || 'undefined';
        }

        /**
         * Get value as number.
         * @param {number} handle
         * @returns {number}
         */
        value_as_number(handle) {
            if (handle === 0) return NaN;
            return this[_wasm].exports.tsrun_get_number(handle);
        }

        /**
         * Get value as string.
         * @param {number} handle
         * @returns {string|undefined}
         */
        value_as_string(handle) {
            if (handle === 0) return undefined;
            const ptr = this[_wasm].exports.tsrun_get_string(handle);
            if (ptr === 0) return undefined;
            return readString(ptr);
        }

        /**
         * Get value as boolean.
         * @param {number} handle
         * @returns {boolean|undefined}
         */
        value_as_bool(handle) {
            if (handle === 0) return undefined;
            if (!this[_wasm].exports.tsrun_is_boolean(handle)) return undefined;
            return this[_wasm].exports.tsrun_get_bool(handle) !== 0;
        }

        /**
         * Check if value is null.
         * @param {number} handle
         * @returns {boolean}
         */
        value_is_null(handle) {
            return handle !== 0 && this[_wasm].exports.tsrun_is_null(handle) !== 0;
        }

        /**
         * Check if value is undefined.
         * @param {number} handle
         * @returns {boolean}
         */
        value_is_undefined(handle) {
            return handle === 0 || this[_wasm].exports.tsrun_is_undefined(handle) !== 0;
        }

        /**
         * Check if value is an array.
         * @param {number} handle
         * @returns {boolean}
         */
        value_is_array(handle) {
            return handle !== 0 && this[_wasm].exports.tsrun_is_array(handle) !== 0;
        }

        /**
         * Check if value is a function.
         * @param {number} handle
         * @returns {boolean}
         */
        value_is_function(handle) {
            return handle !== 0 && this[_wasm].exports.tsrun_is_function(handle) !== 0;
        }

        // ═══════════════════════════════════════════════════════════════════════════════
        // Object Operations
        // ═══════════════════════════════════════════════════════════════════════════════

        /**
         * Get a property from an object.
         * @param {number} objHandle
         * @param {string} key
         * @returns {number} Value handle
         */
        get_property(objHandle, key) {
            const keyAlloc = allocString(key);
            const resultPtr = this[_wasm].exports.tsrun_alloc(8);

            try {
                this[_wasm].exports.tsrun_get(resultPtr, this[_context], objHandle, keyAlloc.ptr);
                const view = getDataView(resultPtr, 8);
                return view.getUint32(0, true);
            } finally {
                this[_wasm].exports.tsrun_dealloc(resultPtr, 8);
                deallocString(keyAlloc.ptr, keyAlloc.len + 1);
            }
        }

        /**
         * Set a property on an object.
         * @param {number} objHandle
         * @param {string} key
         * @param {number} valueHandle
         * @returns {boolean}
         */
        set_property(objHandle, key, valueHandle) {
            const keyAlloc = allocString(key);
            const resultPtr = this[_wasm].exports.tsrun_alloc(8);

            try {
                this[_wasm].exports.tsrun_set(resultPtr, this[_context], objHandle, keyAlloc.ptr, valueHandle);
                const view = getDataView(resultPtr, 8);
                return view.getUint32(0, true) !== 0;
            } finally {
                this[_wasm].exports.tsrun_dealloc(resultPtr, 8);
                deallocString(keyAlloc.ptr, keyAlloc.len + 1);
            }
        }

        /**
         * Get all property keys of an object.
         * @param {number} objHandle
         * @returns {string[]}
         */
        get_keys(objHandle) {
            // tsrun_keys returns pointer to array of C strings + count via out params
            // For simplicity, use JSON stringify then parse to get keys
            const json = this.json_stringify(objHandle);
            if (!json) return [];
            try {
                const obj = JSON.parse(json);
                return Object.keys(obj);
            } catch {
                return [];
            }
        }

        // ═══════════════════════════════════════════════════════════════════════════════
        // Array Operations
        // ═══════════════════════════════════════════════════════════════════════════════

        /**
         * Get array length.
         * @param {number} arrHandle
         * @returns {number}
         */
        array_length(arrHandle) {
            if (arrHandle === 0) return 0;
            return this[_wasm].exports.tsrun_array_len(arrHandle);
        }

        /**
         * Get array element by index.
         * @param {number} arrHandle
         * @param {number} index
         * @returns {number} Value handle
         */
        get_index(arrHandle, index) {
            const resultPtr = this[_wasm].exports.tsrun_alloc(8);

            try {
                this[_wasm].exports.tsrun_array_get(resultPtr, this[_context], arrHandle, index);
                const view = getDataView(resultPtr, 8);
                return view.getUint32(0, true);
            } finally {
                this[_wasm].exports.tsrun_dealloc(resultPtr, 8);
            }
        }

        /**
         * Push a value onto an array.
         * @param {number} arrHandle
         * @param {number} valueHandle
         * @returns {boolean}
         */
        push(arrHandle, valueHandle) {
            const resultPtr = this[_wasm].exports.tsrun_alloc(8);

            try {
                this[_wasm].exports.tsrun_array_push(resultPtr, this[_context], arrHandle, valueHandle);
                const view = getDataView(resultPtr, 8);
                return view.getUint32(0, true) !== 0;
            } finally {
                this[_wasm].exports.tsrun_dealloc(resultPtr, 8);
            }
        }

        // ═══════════════════════════════════════════════════════════════════════════════
        // JSON Operations
        // ═══════════════════════════════════════════════════════════════════════════════

        /**
         * Parse JSON string into a value.
         * @param {string} json
         * @returns {number} Value handle
         */
        json_parse(json) {
            const jsonAlloc = allocString(json);
            const resultPtr = this[_wasm].exports.tsrun_alloc(8);

            try {
                this[_wasm].exports.tsrun_json_parse(resultPtr, this[_context], jsonAlloc.ptr);
                const view = getDataView(resultPtr, 8);
                return view.getUint32(0, true);
            } finally {
                this[_wasm].exports.tsrun_dealloc(resultPtr, 8);
                deallocString(jsonAlloc.ptr, jsonAlloc.len + 1);
            }
        }

        /**
         * Stringify a value to JSON.
         * @param {number} handle
         * @returns {string|null}
         */
        json_stringify(handle) {
            const ptr = this[_wasm].exports.tsrun_json_stringify(this[_context], handle);
            if (ptr === 0) return null;
            const result = readString(ptr);
            this[_wasm].exports.tsrun_free_string(ptr);
            return result;
        }

        // ═══════════════════════════════════════════════════════════════════════════════
        // Value Memory Management
        // ═══════════════════════════════════════════════════════════════════════════════

        /**
         * Free a value handle.
         * @param {number} handle
         */
        release_handle(handle) {
            if (handle !== 0) {
                this[_wasm].exports.tsrun_value_free(handle);
            }
        }
    }

    return TsRunner;
}

// Default export for convenience
export default init;
