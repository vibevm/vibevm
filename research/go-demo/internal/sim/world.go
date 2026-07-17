package sim

import (
	"context"
	"fmt"

	"reconcile-demo/internal/seams"
)

// World is the in-memory model: the actual state plus the plumbing to
// drive one reconcile step. It implements the Store seam, so cells
// accept it directly — no mocking framework anywhere (GUIDE §4-H).
type World struct {
	state seams.State
}

var _ seams.Store = (*World)(nil) // silent conformance made loud

// NewWorld starts a world at the given actual state (copied).
func NewWorld(actual seams.State) *World {
	return &World{state: seams.Clone(actual)}
}

// Snapshot returns a copy of the current state (the Store seam).
func (w *World) Snapshot(ctx context.Context) (seams.State, error) {
	_ = ctx
	return seams.Clone(w.state), nil
}

// Apply performs one action (the Store seam). Unknown ops panic: the
// op set is closed (#req-errors), so an unknown value is an invariant
// violation, not an expected failure.
func (w *World) Apply(ctx context.Context, action seams.Action) error {
	_ = ctx
	switch action.Op {
	case seams.OpCreate, seams.OpUpdate:
		w.state[action.ID] = action.To
	case seams.OpDelete:
		delete(w.state, action.ID)
	default:
		panic(fmt.Sprintf(
			"sim: unknown ActionOp %d — the op set is closed (spec://go-demo/PROP-001#req-errors)",
			action.Op,
		))
	}
	return nil
}

// StepResult is one reconcile step's transcript.
type StepResult struct {
	Applied []seams.Action
	// Converged is true when the step's plan was empty — the fixed
	// point (#req-converge).
	Converged bool
}

// Step plans desired-vs-actual through the given planner and applies
// the whole plan — one reconcile turn of the loop.
//
//spec:implements spec://go-demo/PROP-001#req-converge r=1
func (w *World) Step(
	ctx context.Context,
	planner seams.Planner,
	desired seams.State,
) (StepResult, error) {
	actual, err := w.Snapshot(ctx)
	if err != nil {
		return StepResult{}, err
	}
	plan, err := planner.Plan(ctx, desired, actual)
	if err != nil {
		return StepResult{}, err
	}
	for _, action := range plan {
		if err := w.Apply(ctx, action); err != nil {
			return StepResult{}, err
		}
	}
	return StepResult{Applied: plan, Converged: len(plan) == 0}, nil
}

// FixedClock is the injected time capability for deterministic
// transcripts.
type FixedClock struct {
	// Nanos is returned verbatim by UnixNano.
	Nanos int64
}

var _ seams.Clock = (*FixedClock)(nil)

// UnixNano returns the fixed stamp.
func (c *FixedClock) UnixNano() int64 { return c.Nanos }
