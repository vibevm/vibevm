package registry

import (
	"fmt"

	"reconcile-demo/internal/cells/batchplanner"
	"reconcile-demo/internal/cells/naiveplanner"
)

// The registry's canonical use, executed (scaffold G): the flag
// selects the cell; the switch is the system's table of contents.
func ExamplePlanner() {
	naive := Planner(Config{Planner: PlannerNaive})
	batch := Planner(Config{Planner: PlannerBatch})
	_, isNaive := naive.(*naiveplanner.NaivePlanner)
	_, isBatch := batch.(*batchplanner.BatchPlanner)
	fmt.Println(isNaive, isBatch)
	// Output: true true
}
