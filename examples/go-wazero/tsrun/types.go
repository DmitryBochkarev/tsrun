// Package tsrun provides Go bindings for the tsrun TypeScript interpreter via WASM.
package tsrun

// StepStatus represents the status of an execution step.
type StepStatus int

const (
	// StatusContinue indicates more instructions to execute.
	StatusContinue StepStatus = 0
	// StatusComplete indicates execution finished with a value.
	StatusComplete StepStatus = 1
	// StatusNeedImports indicates waiting for modules to be loaded.
	StatusNeedImports StepStatus = 2
	// StatusSuspended indicates waiting for order fulfillment.
	StatusSuspended StepStatus = 3
	// StatusDone indicates no active execution.
	StatusDone StepStatus = 4
	// StatusError indicates an execution error.
	StatusError StepStatus = 5
)

// String returns a string representation of the StepStatus.
func (s StepStatus) String() string {
	switch s {
	case StatusContinue:
		return "Continue"
	case StatusComplete:
		return "Complete"
	case StatusNeedImports:
		return "NeedImports"
	case StatusSuspended:
		return "Suspended"
	case StatusDone:
		return "Done"
	case StatusError:
		return "Error"
	default:
		return "Unknown"
	}
}

// ValueType represents the type of a JavaScript value.
type ValueType int

const (
	TypeUndefined ValueType = 0
	TypeNull      ValueType = 1
	TypeBoolean   ValueType = 2
	TypeNumber    ValueType = 3
	TypeString    ValueType = 4
	TypeObject    ValueType = 5
	TypeSymbol    ValueType = 6
)

// String returns a string representation of the ValueType.
func (t ValueType) String() string {
	switch t {
	case TypeUndefined:
		return "undefined"
	case TypeNull:
		return "null"
	case TypeBoolean:
		return "boolean"
	case TypeNumber:
		return "number"
	case TypeString:
		return "string"
	case TypeObject:
		return "object"
	case TypeSymbol:
		return "symbol"
	default:
		return "unknown"
	}
}

// ImportRequest represents a pending import request.
type ImportRequest struct {
	// Specifier is the original import specifier as written in source code.
	Specifier string
	// ResolvedPath is the resolved absolute path.
	ResolvedPath string
	// Importer is the module that requested this import (empty for main module).
	Importer string
}

// Order represents a pending order from TypeScript to the host.
type Order struct {
	// ID is the unique order ID.
	ID uint64
	// Payload is the order payload value.
	Payload *Value
}

// OrderResponse represents a response to an order.
type OrderResponse struct {
	// ID is the order ID this response is for.
	ID uint64
	// Value is the result value (nil if error).
	Value *Value
	// Error is the error message (empty if success).
	Error string
}

// StepResult represents the result of an execution step.
type StepResult struct {
	// Status is the execution status.
	Status StepStatus
	// Value is the result value (for StatusComplete).
	Value *Value
	// Error is the error message (for StatusError).
	Error string
	// ImportRequests contains pending import requests (for StatusNeedImports).
	ImportRequests []ImportRequest
	// PendingOrders contains orders waiting for fulfillment (for StatusSuspended).
	PendingOrders []Order
	// CancelledOrders contains cancelled order IDs (for StatusSuspended).
	CancelledOrders []uint64
}

// ConsoleLevel represents the log level for console output.
type ConsoleLevel int

const (
	ConsoleLevelLog   ConsoleLevel = 0
	ConsoleLevelInfo  ConsoleLevel = 1
	ConsoleLevelDebug ConsoleLevel = 2
	ConsoleLevelWarn  ConsoleLevel = 3
	ConsoleLevelError ConsoleLevel = 4
)
