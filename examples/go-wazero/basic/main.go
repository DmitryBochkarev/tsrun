// Basic example: Run TypeScript code synchronously and get the result.
package main

import (
	"context"
	"fmt"
	"log"

	"github.com/example/tsrun-go/tsrun"
)

func main() {
	ctx := context.Background()

	// Create runtime with console callback
	rt, err := tsrun.New(ctx, tsrun.ConsoleOption(func(level tsrun.ConsoleLevel, message string) {
		switch level {
		case tsrun.ConsoleLevelWarn, tsrun.ConsoleLevelError:
			fmt.Printf("[%s] %s\n", level, message)
		default:
			fmt.Printf("%s\n", message)
		}
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

	// TypeScript code with enum and typed config
	code := `
		enum Status { Active = 1, Inactive = 0 }

		interface Config {
			status: Status;
			name: string;
			values: number[];
		}

		const config: Config = {
			status: Status.Active,
			name: "example",
			values: [1, 2, 3, 4, 5]
		};

		console.log("Config name:", config.name);
		console.log("Status:", config.status);
		console.log("Sum of values:", config.values.reduce((a, b) => a + b, 0));

		// Return the status value
		config.status
	`

	fmt.Println("=== Running TypeScript code ===")
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
	fmt.Printf("Status: %s\n", result.Status)

	switch result.Status {
	case tsrun.StatusComplete:
		if result.Value != nil {
			// Try to get the value as a number
			typ, _ := result.Value.Type(ctx)
			fmt.Printf("Result type: %s\n", typ)

			if typ == tsrun.TypeNumber {
				num, _ := result.Value.AsNumber(ctx)
				fmt.Printf("Result value: %.0f\n", num)
			}
			result.Value.Free(ctx)
		} else {
			fmt.Println("Result: undefined")
		}

	case tsrun.StatusError:
		fmt.Printf("Error: %s\n", result.Error)

	default:
		fmt.Printf("Unexpected status: %s\n", result.Status)
	}
}
