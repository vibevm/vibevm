package naiveplanner

import (
	"context"
	"errors"
	"fmt"
	"testing"
	"testing/quick"

	"reconcile-demo/internal/seams"
)

// The declared test matrix (GUIDE §11): a named, bounded case table —
// never an implicit 2^n.
//
//spec:verifies spec://go-demo/PROP-001#req-plan-total r=1
func TestPlanMatrix(t *testing.T) {
	cases := []struct {
		name    string
		desired seams.State
		actual  seams.State
		want    []seams.Action
	}{
		{
			name:    "empty worlds plan nothing",
			desired: seams.State{},
			actual:  seams.State{},
			want:    []seams.Action{},
		},
		{
			name:    "create missing",
			desired: seams.State{"api": 1},
			actual:  seams.State{},
			want:    []seams.Action{{Op: seams.OpCreate, ID: "api", To: 1}},
		},
		{
			name:    "update stale",
			desired: seams.State{"api": 2},
			actual:  seams.State{"api": 1},
			want:    []seams.Action{{Op: seams.OpUpdate, ID: "api", To: 2}},
		},
		{
			name:    "delete extra",
			desired: seams.State{},
			actual:  seams.State{"cache": 1},
			want:    []seams.Action{{Op: seams.OpDelete, ID: "cache"}},
		},
		{
			name:    "classes ordered creates-updates-deletes, ids sorted",
			desired: seams.State{"b": 1, "a": 2, "c": 3},
			actual:  seams.State{"c": 1, "z": 9},
			want: []seams.Action{
				{Op: seams.OpCreate, ID: "a", To: 2},
				{Op: seams.OpCreate, ID: "b", To: 1},
				{Op: seams.OpUpdate, ID: "c", To: 3},
				{Op: seams.OpDelete, ID: "z"},
			},
		},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			got, err := New().Plan(context.Background(), tc.desired, tc.actual)
			if err != nil {
				t.Fatalf("plan: %v", err)
			}
			if len(got) != len(tc.want) {
				t.Fatalf("plan length: got %v, want %v", got, tc.want)
			}
			for i := range got {
				if got[i] != tc.want[i] {
					t.Fatalf("action %d: got %+v, want %+v", i, got[i], tc.want[i])
				}
			}
		})
	}
}

// The seam's one expected failure is a value in the closed set, not a
// panic and not prose.
//
//spec:verifies spec://go-demo/PROP-001#req-errors r=1
func TestNilStateIsTheClosedFailure(t *testing.T) {
	_, err := New().Plan(context.Background(), nil, seams.State{})
	var pe *seams.PlanError
	if !errors.As(err, &pe) {
		t.Fatalf("want *seams.PlanError, got %T (%v)", err, err)
	}
	if pe.Code != seams.ErrNilState {
		t.Fatalf("want ErrNilState, got %v", pe.Code)
	}
}

// The #req-plan-total property, quick-checked: applying the plan to
// actual yields desired exactly (scaffold C's behavioral backing).
//
//spec:verifies spec://go-demo/PROP-001#req-plan-total r=1
func TestPlanIsTotalProperty(t *testing.T) {
	property := func(rawDesired, rawActual map[uint8]uint8) bool {
		desired := stateOf(rawDesired)
		actual := stateOf(rawActual)
		plan, err := New().Plan(context.Background(), desired, actual)
		if err != nil {
			return false
		}
		applied := seams.Clone(actual)
		for _, a := range plan {
			switch a.Op {
			case seams.OpCreate, seams.OpUpdate:
				applied[a.ID] = a.To
			case seams.OpDelete:
				delete(applied, a.ID)
			}
		}
		if len(applied) != len(desired) {
			return false
		}
		for id, rev := range desired {
			if applied[id] != rev {
				return false
			}
		}
		return true
	}
	if err := quick.Check(property, nil); err != nil {
		t.Fatal(err)
	}
}

func stateOf(raw map[uint8]uint8) seams.State {
	out := seams.State{}
	for k, v := range raw {
		out[seams.ResourceID(fmt.Sprintf("r%d", k))] = seams.Revision(v)
	}
	return out
}

// The canonical use, executed (scaffold G): construct via New, plan a
// one-divergence world, read the ordered plan.
func ExampleNew() {
	plan, err := New().Plan(
		context.Background(),
		seams.State{"api": 2},
		seams.State{"api": 1},
	)
	if err != nil {
		fmt.Println(err)
		return
	}
	for _, a := range plan {
		fmt.Printf("%s %s -> %d\n", a.Op, a.ID, a.To)
	}
	// Output: update api -> 2
}
