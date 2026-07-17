// The composition root: the one place ambient state (env, stdout) is
// legal. Reads the planner flag, wires the world, runs the loop to
// convergence, prints the transcript.
package main

import (
	"context"
	"fmt"
	"os"

	"reconcile-demo/internal/registry"
	"reconcile-demo/internal/seams"
	"reconcile-demo/internal/sim"
)

func main() {
	cfg := registry.Config{Planner: registry.PlannerNaive}
	if os.Getenv("PLANNER") == "batch" { // provenance: env
		cfg.Planner = registry.PlannerBatch
	}
	planner := registry.Planner(cfg)

	desired := seams.State{"api": 3, "db": 1, "web": 2}
	world := sim.NewWorld(seams.State{"api": 2, "cache": 1})

	ctx := context.Background()
	for turn := 1; ; turn++ {
		result, err := world.Step(ctx, planner, desired)
		if err != nil {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}
		fmt.Printf("turn %d: %d action(s)\n", turn, len(result.Applied))
		for _, a := range result.Applied {
			fmt.Printf("  %s %s -> %d\n", a.Op, a.ID, a.To)
		}
		if result.Converged {
			fmt.Println("converged")
			return
		}
	}
}
