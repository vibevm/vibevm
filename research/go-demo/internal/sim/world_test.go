package sim

import (
	"context"
	"fmt"
	"testing"

	"reconcile-demo/internal/cells/naiveplanner"
	"reconcile-demo/internal/seams"
)

// Convergence (#req-converge): one step reaches desired; the second
// step is the empty fixed point.
//
//spec:verifies spec://go-demo/PROP-001#req-converge r=1
func TestReconcileConvergesInOneStepAndHoldsTheFixedPoint(t *testing.T) {
	world := NewWorld(seams.State{"api": 2, "cache": 1})
	desired := seams.State{"api": 3, "db": 1}
	planner := naiveplanner.New()
	ctx := context.Background()

	first, err := world.Step(ctx, planner, desired)
	if err != nil {
		t.Fatal(err)
	}
	if first.Converged {
		t.Fatalf("a divergent world must not report convergence on the mutating step")
	}
	if len(first.Applied) != 3 { // create db, update api, delete cache
		t.Fatalf("want 3 actions, got %v", first.Applied)
	}

	second, err := world.Step(ctx, planner, desired)
	if err != nil {
		t.Fatal(err)
	}
	if !second.Converged || len(second.Applied) != 0 {
		t.Fatalf("the second step must be the empty fixed point, got %v", second.Applied)
	}

	snapshot, err := world.Snapshot(ctx)
	if err != nil {
		t.Fatal(err)
	}
	if len(snapshot) != len(desired) || snapshot["api"] != 3 || snapshot["db"] != 1 {
		t.Fatalf("the world must equal desired, got %v", snapshot)
	}
}

// Snapshots are copies: mutating the world after a snapshot must not
// alias caller memory (the Clone contract at the seam).
func TestSnapshotDoesNotAlias(t *testing.T) {
	world := NewWorld(seams.State{"api": 1})
	ctx := context.Background()
	before, err := world.Snapshot(ctx)
	if err != nil {
		t.Fatal(err)
	}
	if err := world.Apply(ctx, seams.Action{Op: seams.OpDelete, ID: "api"}); err != nil {
		t.Fatal(err)
	}
	if before["api"] != 1 {
		t.Fatalf("the earlier snapshot changed under the caller")
	}
}

// An unknown op is an invariant violation — the panic channel, and the
// message cites the REQ (Class F).
func TestUnknownOpPanicsWithTheReq(t *testing.T) {
	defer func() {
		r := recover()
		if r == nil {
			t.Fatalf("an unknown op must panic (the op set is closed)")
		}
		text := fmt.Sprint(r)
		if want := "spec://go-demo/PROP-001#req-errors"; !contains(text, want) {
			t.Fatalf("the panic must cite %s, said: %s", want, text)
		}
	}()
	world := NewWorld(seams.State{})
	_ = world.Apply(context.Background(), seams.Action{Op: seams.ActionOp(99), ID: "x"})
}

func contains(haystack, needle string) bool {
	return len(haystack) >= len(needle) && (haystack == needle ||
		len(haystack) > len(needle) && (haystack[:len(needle)] == needle ||
			contains(haystack[1:], needle)))
}

// The steppable model's canonical use, executed (scaffold G + H): a
// reader RUNS the convergence instead of simulating it mentally.
func ExampleWorld_Step() {
	world := NewWorld(seams.State{"api": 2})
	desired := seams.State{"api": 3, "db": 1}
	result, err := world.Step(context.Background(), naiveplanner.New(), desired)
	if err != nil {
		fmt.Println(err)
		return
	}
	for _, a := range result.Applied {
		fmt.Printf("%s %s -> %d\n", a.Op, a.ID, a.To)
	}
	fmt.Println("converged:", result.Converged)
	// Output:
	// create db -> 1
	// update api -> 3
	// converged: false
}
