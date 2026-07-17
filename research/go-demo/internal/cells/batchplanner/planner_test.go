package batchplanner

import (
	"context"
	"encoding/binary"
	"fmt"
	"testing"

	"reconcile-demo/internal/cells/naiveplanner"
	"reconcile-demo/internal/seams"
)

// NOTE (go-cell-isolation): this differential oracle imports the
// SIBLING cell it replaces — exactly what the isolation rule flags.
// The finding is real, deliberate, and frozen in the demo's conform
// baseline as the replacement window's recorded debt: the oracle dies
// with naiveplanner once the replacement is ratified (scaffold D's
// "keep old reachable until green" step, pinned by the ratchet
// instead of hidden from it).

// The differential fuzz oracle (scaffold D, the replacement protocol):
// identical generated worlds through BOTH planners, byte-equal plans
// demanded. The seed corpus below covers the hard shapes; `go test`
// runs seeds deterministically, `-fuzz` explores locally.
//
//spec:verifies spec://go-demo/PROP-001#req-planner-seam r=1
func FuzzPlannersAgree(f *testing.F) {
	f.Add([]byte{}, []byte{})
	f.Add([]byte{1, 1}, []byte{})
	f.Add([]byte{}, []byte{2, 9})
	f.Add([]byte{1, 1, 2, 2, 3, 3}, []byte{1, 9, 3, 3, 4, 4})
	f.Fuzz(func(t *testing.T, rawDesired, rawActual []byte) {
		desired := decodeState(rawDesired)
		actual := decodeState(rawActual)
		ctx := context.Background()
		newPlan, newErr := New().Plan(ctx, desired, actual)
		oldPlan, oldErr := naiveplanner.New().Plan(ctx, desired, actual)
		if (newErr == nil) != (oldErr == nil) {
			t.Fatalf("error disagreement: new=%v old=%v", newErr, oldErr)
		}
		if newErr != nil {
			return
		}
		if len(newPlan) != len(oldPlan) {
			t.Fatalf("plan lengths differ: new=%v old=%v", newPlan, oldPlan)
		}
		for i := range newPlan {
			if newPlan[i] != oldPlan[i] {
				t.Fatalf("action %d differs: new=%+v old=%+v", i, newPlan[i], oldPlan[i])
			}
		}
	})
}

// decodeState reads (id, revision) byte pairs — the fuzz encoding of a
// small world.
func decodeState(raw []byte) seams.State {
	out := seams.State{}
	for i := 0; i+1 < len(raw); i += 2 {
		id := seams.ResourceID(fmt.Sprintf("r%d", raw[i]%16))
		out[id] = seams.Revision(binary.LittleEndian.Uint16([]byte{raw[i+1], 0}))
	}
	return out
}

// Determinism is part of the contract (#req-plan-order): identical
// inputs, byte-identical plans, twice.
//
//spec:verifies spec://go-demo/PROP-001#req-plan-order r=1
func TestPlanIsDeterministic(t *testing.T) {
	desired := seams.State{"a": 1, "b": 2, "c": 3, "d": 4}
	actual := seams.State{"b": 9, "e": 1}
	ctx := context.Background()
	first, err := New().Plan(ctx, desired, actual)
	if err != nil {
		t.Fatal(err)
	}
	second, err := New().Plan(ctx, desired, actual)
	if err != nil {
		t.Fatal(err)
	}
	if len(first) != len(second) {
		t.Fatalf("lengths differ")
	}
	for i := range first {
		if first[i] != second[i] {
			t.Fatalf("action %d differs across runs", i)
		}
	}
}

// The canonical use, executed (scaffold G).
func ExampleNew() {
	plan, err := New().Plan(
		context.Background(),
		seams.State{"api": 1, "web": 1},
		seams.State{"cache": 1},
	)
	if err != nil {
		fmt.Println(err)
		return
	}
	for _, a := range plan {
		fmt.Printf("%s %s\n", a.Op, a.ID)
	}
	// Output:
	// create api
	// create web
	// delete cache
}
