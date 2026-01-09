// Native functions example: Register Go callbacks as JavaScript functions.
//
// Note: Full native callback support requires implementing the callback
// trampoline mechanism. This example demonstrates the concept.
package main

import (
	"context"
	"fmt"
	"log"

	"github.com/example/tsrun-go/tsrun"
)

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

	// For now, demonstrate using the built-in functionality
	// Full native callback registration would require:
	// 1. A callback registry in Go
	// 2. A trampoline mechanism to route WASM calls back to Go
	// 3. Proper argument marshaling

	// Instead, let's show a simple example using built-in features
	code := `
		// This example shows what native function usage would look like
		// once the callback system is fully implemented.
		//
		// In a full implementation, you could register Go functions like:
		//   interp.RegisterFunction("myGoFunc", func(args) { ... })
		//
		// And call them from TypeScript:
		//   const result = myGoFunc(1, 2, 3);

		// For now, let's demonstrate the interpreter's capabilities
		function factorial(n: number): number {
			if (n <= 1) return 1;
			return n * factorial(n - 1);
		}

		function fibonacci(n: number): number {
			if (n <= 1) return n;
			return fibonacci(n - 1) + fibonacci(n - 2);
		}

		console.log("Factorial of 10:", factorial(10));
		console.log("Fibonacci of 20:", fibonacci(20));

		// Use Date.now() which calls our host_time_now import
		const start = Date.now();
		let sum = 0;
		for (let i = 0; i < 100000; i++) {
			sum += i;
		}
		const end = Date.now();
		console.log("Sum of 0..99999:", sum);
		console.log("Time taken:", end - start, "ms");

		// Use Math.random() which calls our host_random import
		const randoms = [];
		for (let i = 0; i < 5; i++) {
			randoms.push(Math.random());
		}
		console.log("Random numbers:", randoms.join(", "));

		sum
	`

	fmt.Println("=== Running TypeScript with host functions ===")
	fmt.Println()

	// Prepare the code
	if err := interp.Prepare(ctx, code, "/main.ts"); err != nil {
		log.Fatalf("Prepare error: %v", err)
	}

	// Run to completion
	result, err := interp.Run(ctx)
	if err != nil {
		log.Fatalf("Run error: %v", err)
	}

	fmt.Println()
	fmt.Println("=== Execution completed ===")

	switch result.Status {
	case tsrun.StatusComplete:
		if result.Value != nil {
			typ, _ := result.Value.Type(ctx)
			if typ == tsrun.TypeNumber {
				num, _ := result.Value.AsNumber(ctx)
				fmt.Printf("Result: %.0f\n", num)
			}
			result.Value.Free(ctx)
		}

	case tsrun.StatusError:
		log.Fatalf("Error: %s", result.Error)

	default:
		fmt.Printf("Status: %s\n", result.Status)
	}
}
