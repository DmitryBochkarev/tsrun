// Async orders example: Handle async operations via the order system.
//
// This example demonstrates how to use the order system for async operations
// like HTTP fetching, timers, or any custom async operation.
//
// Key pattern: Spawn goroutines for async work that run in the background.
// The interpreter continues running and suspends when awaiting unresolved
// Promises. We poll for completed async results and resolve Promises as
// they complete, enabling true parallelism with Promise.all.
package main

import (
	"context"
	"fmt"
	"log"
	"math/rand"
	"strings"
	"time"

	"github.com/example/tsrun-go/tsrun"
)

// asyncResult holds the result of completed async work.
type asyncResult struct {
	orderID      uint64
	promise      *tsrun.Value
	jsonResponse string
	errorMsg     string
}

func main() {
	ctx := context.Background()

	// Create runtime
	rt, err := tsrun.New(ctx, tsrun.ConsoleOption(func(level tsrun.ConsoleLevel, message string) {
		fmt.Println(message)
	}))
	if err != nil {
		log.Fatalf("Failed to create runtime: %v", err)
	}
	defer rt.Close(ctx)

	// Create interpreter context
	interp, err := rt.NewContext(ctx)
	if err != nil {
		log.Fatalf("Failed to create context: %v", err)
	}
	defer interp.Free(ctx)

	// TypeScript code that uses async operations via orders
	code := `
		import { order } from "tsrun:host";

		// Helper to create a delay
		function delay(ms: number): Promise<void> {
			return order({ type: "delay", ms });
		}

		// Helper to fetch data (simulated)
		function fetchData(url: string): Promise<any> {
			return order({ type: "fetch", url });
		}

		async function main() {
			console.log("Starting async operations...");

			// Test sequential operations
			console.log("Waiting 100ms...");
			await delay(100);
			console.log("Delay complete!");

			// Test parallel operations with Promise.all
			console.log("Fetching data in parallel...");
			const startTime = Date.now();

			const [data1, data2, data3] = await Promise.all([
				fetchData("https://api.example.com/users"),
				fetchData("https://api.example.com/products"),
				fetchData("https://api.example.com/orders"),
			]);

			const elapsed = Date.now() - startTime;
			console.log("All fetches completed in", elapsed, "ms");
			console.log("Users:", JSON.stringify(data1));
			console.log("Products:", JSON.stringify(data2));
			console.log("Orders:", JSON.stringify(data3));

			return data1.value + data2.value + data3.value;
		}

		main()
	`

	fmt.Println("=== Running async TypeScript code ===")
	fmt.Println()

	// Prepare the code
	if err := interp.Prepare(ctx, code, "/main.ts"); err != nil {
		log.Fatalf("Prepare error: %v", err)
	}

	// Channel for receiving completed async work (buffered for non-blocking sends)
	resultChan := make(chan asyncResult, 100)

	// Track number of pending goroutines
	pendingCount := 0

	// Execution loop
	for {
		// First, resolve any completed promises (non-blocking check)
		resolvedAny := false
	drainResults:
		for {
			select {
			case res := <-resultChan:
				pendingCount--
				resolvePromise(ctx, interp, res)
				resolvedAny = true
			default:
				break drainResults
			}
		}

		// Run the interpreter
		result, err := interp.Run(ctx)
		if err != nil {
			log.Fatalf("Run error: %v", err)
		}

		switch result.Status {
		case tsrun.StatusSuspended:
			// Handle any new pending orders
			if len(result.PendingOrders) > 0 {
				for _, order := range result.PendingOrders {
					// Create a Promise for this order
					promise, err := interp.CreateOrderPromise(ctx, order.ID)
					if err != nil {
						log.Fatalf("Failed to create promise for order %d: %v", order.ID, err)
					}

					// Extract order info (WASM calls on main goroutine)
					info := extractOrderInfo(ctx, order.Payload)

					// Log promise creation (similar to wasm-playground)
					fmt.Printf("[Order %d] Creating Promise for %s, will resolve in %dms...\n", order.ID, info.orderType, info.delayMs)

					// Fulfill order immediately with its Promise
					if err := interp.FulfillOrders(ctx, []tsrun.OrderResponse{{
						ID:    order.ID,
						Value: promise,
					}}); err != nil {
						log.Fatalf("FulfillOrders error: %v", err)
					}

					// Spawn goroutine for async work
					pendingCount++
					go func(id uint64, prom *tsrun.Value, oi orderInfo) {
						jsonResp, errMsg := doAsyncWork(oi)
						resultChan <- asyncResult{
							orderID:      id,
							promise:      prom,
							jsonResponse: jsonResp,
							errorMsg:     errMsg,
						}
					}(order.ID, promise, info)
				}
				continue // Continue to process more orders or run interpreter
			}

			// No new orders but still suspended - waiting for promises to resolve
			if pendingCount > 0 {
				// Block until at least one result is ready
				res := <-resultChan
				pendingCount--
				resolvePromise(ctx, interp, res)
				continue
			}

			// No pending orders and no pending goroutines - shouldn't happen
			log.Fatalf("Suspended with no pending work")

		case tsrun.StatusComplete:
			// Drain any remaining results (shouldn't be any)
			close(resultChan)
			for res := range resultChan {
				resolvePromise(ctx, interp, res)
			}

			fmt.Println()
			fmt.Println("=== Execution completed ===")
			if result.Value != nil {
				typ, _ := result.Value.Type(ctx)
				fmt.Printf("Result type: %s\n", typ)
				if typ == tsrun.TypeNumber {
					num, _ := result.Value.AsNumber(ctx)
					fmt.Printf("Result value: %.0f\n", num)
				}
				result.Value.Free(ctx)
			}
			return

		case tsrun.StatusError:
			log.Fatalf("Error: %s", result.Error)

		case tsrun.StatusNeedImports:
			log.Fatalf("Need imports (tsrun:host should be built-in): %v", result.ImportRequests)

		default:
			// If we resolved promises, might need to continue
			if resolvedAny {
				continue
			}
			fmt.Printf("Status: %s\n", result.Status)
			return
		}
	}
}

// resolvePromise resolves or rejects a promise based on the async result.
func resolvePromise(ctx context.Context, interp *tsrun.Context, res asyncResult) {
	if res.errorMsg != "" {
		fmt.Printf("[Order %d] Rejecting Promise with error: %s\n", res.orderID, res.errorMsg)
		if err := interp.RejectPromise(ctx, res.promise, res.errorMsg); err != nil {
			log.Printf("Failed to reject promise: %v", err)
		}
		return
	}

	if res.jsonResponse != "" {
		fmt.Printf("[Order %d] Resolving Promise with: %s\n", res.orderID, res.jsonResponse)
		val, err := interp.JSONParse(ctx, res.jsonResponse)
		if err != nil {
			if err := interp.RejectPromise(ctx, res.promise, fmt.Sprintf("failed to create response: %v", err)); err != nil {
				log.Printf("Failed to reject promise: %v", err)
			}
			return
		}
		if err := interp.ResolvePromise(ctx, res.promise, val); err != nil {
			log.Printf("Failed to resolve promise: %v", err)
		}
		val.Free(ctx)
		return
	}

	// Resolve with undefined (e.g., for delay)
	fmt.Printf("[Order %d] Resolving Promise (void)\n", res.orderID)
	val, _ := interp.Undefined(ctx)
	if err := interp.ResolvePromise(ctx, res.promise, val); err != nil {
		log.Printf("Failed to resolve promise: %v", err)
	}
	val.Free(ctx)
}

// orderInfo contains extracted order data for use in goroutines.
type orderInfo struct {
	orderType string
	delayMs   int // actual delay to use (calculated upfront)
	url       string
}

// extractOrderInfo extracts order info from payload and calculates actual delay.
func extractOrderInfo(ctx context.Context, payload *tsrun.Value) orderInfo {
	info := orderInfo{}
	if payload == nil {
		return info
	}

	orderType, _ := payload.Get(ctx, "type")
	if orderType != nil {
		info.orderType, _ = orderType.AsString(ctx)
		orderType.Free(ctx)
	}

	switch info.orderType {
	case "delay":
		msVal, _ := payload.Get(ctx, "ms")
		if msVal != nil {
			ms, _ := msVal.AsNumber(ctx)
			info.delayMs = int(ms)
			msVal.Free(ctx)
		}
	case "fetch":
		urlVal, _ := payload.Get(ctx, "url")
		if urlVal != nil {
			info.url, _ = urlVal.AsString(ctx)
			urlVal.Free(ctx)
		}
		// Calculate actual random delay upfront (50-200ms)
		info.delayMs = 50 + rand.Intn(150)
	}

	return info
}

// doAsyncWork performs async work in a goroutine (no WASM calls).
func doAsyncWork(info orderInfo) (jsonResponse string, errorMsg string) {
	// Use pre-calculated delay
	time.Sleep(time.Duration(info.delayMs) * time.Millisecond)

	switch info.orderType {
	case "delay":
		return "", ""

	case "fetch":
		switch {
		case strings.Contains(info.url, "users"):
			return `{"value": 10, "endpoint": "users", "count": 42}`, ""
		case strings.Contains(info.url, "products"):
			return `{"value": 20, "endpoint": "products", "count": 100}`, ""
		case strings.Contains(info.url, "orders"):
			return `{"value": 30, "endpoint": "orders", "count": 15}`, ""
		default:
			return `{"value": 1, "status": "ok"}`, ""
		}

	default:
		return "", fmt.Sprintf("unknown order type: %s", info.orderType)
	}
}
