package tsrun

import (
	"context"
	"fmt"
)

// allocString allocates a null-terminated string in WASM memory and returns the pointer.
// The caller is responsible for calling deallocString to free it.
// Note: Allocates len(s)+1 bytes for the null terminator.
func (r *Runtime) allocString(ctx context.Context, s string) (uint32, error) {
	if len(s) == 0 {
		return 0, nil
	}

	// Allocate space for string + null terminator
	allocSize := uint64(len(s) + 1)
	results, err := r.fnAlloc.Call(ctx, allocSize)
	if err != nil {
		return 0, fmt.Errorf("failed to allocate memory: %w", err)
	}
	ptr := uint32(results[0])
	if ptr == 0 {
		return 0, fmt.Errorf("memory allocation failed")
	}

	// Write string content
	if !r.memory.Write(ptr, []byte(s)) {
		// Try to free the allocated memory on failure
		r.fnDealloc.Call(ctx, uint64(ptr), allocSize)
		return 0, fmt.Errorf("failed to write string to memory")
	}

	// Write null terminator
	if !r.memory.WriteByte(ptr+uint32(len(s)), 0) {
		r.fnDealloc.Call(ctx, uint64(ptr), allocSize)
		return 0, fmt.Errorf("failed to write null terminator")
	}

	return ptr, nil
}

// deallocString frees a string allocated with allocString.
func (r *Runtime) deallocString(ctx context.Context, ptr uint32, size uint32) {
	if ptr == 0 || size == 0 {
		return
	}
	r.fnDealloc.Call(ctx, uint64(ptr), uint64(size))
}

// readString reads a null-terminated string from WASM memory.
func (r *Runtime) readString(ptr uint32) string {
	if ptr == 0 {
		return ""
	}

	// Read bytes until we hit a null terminator or memory end
	var buf []byte
	for i := uint32(0); ; i++ {
		b, ok := r.memory.ReadByte(ptr + i)
		if !ok || b == 0 {
			break
		}
		buf = append(buf, b)
	}
	return string(buf)
}

// readStringWithLen reads a string of known length from WASM memory.
func (r *Runtime) readStringWithLen(ptr uint32, length uint32) string {
	if ptr == 0 || length == 0 {
		return ""
	}

	data, ok := r.memory.Read(ptr, length)
	if !ok {
		return ""
	}
	return string(data)
}

// allocResult allocates memory for a result struct (used for sret convention).
func (r *Runtime) allocResult(ctx context.Context, size uint32) (uint32, error) {
	results, err := r.fnAlloc.Call(ctx, uint64(size))
	if err != nil {
		return 0, fmt.Errorf("failed to allocate result memory: %w", err)
	}
	ptr := uint32(results[0])
	if ptr == 0 {
		return 0, fmt.Errorf("result memory allocation failed")
	}
	return ptr, nil
}

// deallocResult frees memory allocated for a result struct.
func (r *Runtime) deallocResult(ctx context.Context, ptr uint32, size uint32) {
	if ptr == 0 {
		return
	}
	r.fnDealloc.Call(ctx, uint64(ptr), uint64(size))
}
