// Module loading example: Handle ES module imports with a virtual filesystem.
package main

import (
	"context"
	"fmt"
	"log"

	"github.com/example/tsrun-go/tsrun"
)

// Virtual filesystem containing our modules
// Note: Paths must match the resolved paths from the interpreter
// For relative imports like "./lib/math.ts" from "/main.ts", the
// resolved path is "lib/math.ts" (no leading slash)
var modules = map[string]string{
	"lib/math.ts": `
		export function add(a: number, b: number): number {
			return a + b;
		}

		export function multiply(a: number, b: number): number {
			return a * b;
		}

		export const PI = 3.14159;
	`,
	"lib/utils.ts": `
		export function greet(name: string): string {
			return "Hello, " + name + "!";
		}

		export function repeat(s: string, n: number): string {
			let result = "";
			for (let i = 0; i < n; i++) {
				result += s;
			}
			return result;
		}
	`,
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

	// Main module that imports from our virtual filesystem
	code := `
		import { add, multiply, PI } from "./lib/math.ts";
		import { greet, repeat } from "./lib/utils.ts";

		console.log(greet("World"));
		console.log("PI is:", PI);
		console.log("3 + 4 =", add(3, 4));
		console.log("5 * 6 =", multiply(5, 6));
		console.log(repeat("Go! ", 3));

		add(10, 20) + multiply(2, 3)
	`

	fmt.Println("=== Running TypeScript with ES modules ===")
	fmt.Println()

	// Prepare the code
	if err := interp.Prepare(ctx, code, "/main.ts"); err != nil {
		log.Fatalf("Prepare error: %v", err)
	}

	// Track provided modules to avoid infinite loops
	providedModules := make(map[string]bool)
	maxIterations := 100

	// Execution loop with module loading
	for i := 0; i < maxIterations; i++ {
		result, err := interp.Run(ctx)
		if err != nil {
			log.Fatalf("Run error: %v", err)
		}

		switch result.Status {
		case tsrun.StatusNeedImports:
			if len(result.ImportRequests) == 0 {
				log.Fatalf("NeedImports status but no import requests")
			}
			fmt.Printf("Loading %d module(s)...\n", len(result.ImportRequests))
			for _, req := range result.ImportRequests {
				// Use the resolved path directly from the interpreter
				resolvedPath := req.ResolvedPath

				// Skip if already provided
				if providedModules[resolvedPath] {
					fmt.Printf("  - Already provided: %s (skipping)\n", resolvedPath)
					continue
				}

				source, ok := modules[resolvedPath]
				if !ok {
					log.Fatalf("Module not found: %s (from %s)", req.Specifier, req.Importer)
				}

				fmt.Printf("  - Providing: %s\n", resolvedPath)
				if err := interp.ProvideModule(ctx, resolvedPath, source); err != nil {
					log.Fatalf("Failed to provide module %s: %v", resolvedPath, err)
				}
				providedModules[resolvedPath] = true
			}
			continue // Continue execution

		case tsrun.StatusComplete:
			fmt.Println()
			fmt.Println("=== Execution completed ===")
			if result.Value != nil {
				typ, _ := result.Value.Type(ctx)
				if typ == tsrun.TypeNumber {
					num, _ := result.Value.AsNumber(ctx)
					fmt.Printf("Result: %.0f\n", num)
				}
				result.Value.Free(ctx)
			}
			return

		case tsrun.StatusError:
			log.Fatalf("Error: %s", result.Error)

		default:
			fmt.Printf("Unexpected status: %s\n", result.Status)
			return
		}
	}
}
