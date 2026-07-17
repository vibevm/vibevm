package naiveplanner

import (
	"context"
	"sort"

	"reconcile-demo/internal/seams"
)

// NaivePlanner diffs desired against actual in three obvious passes.
// Construct via New. The cell is pure — it needs no capabilities, and
// deliberately takes none (ambient state stays banned either way).
type NaivePlanner struct{}

var _ seams.Planner = (*NaivePlanner)(nil) // silent conformance made loud

// New is the blessed construction path.
func New() *NaivePlanner { return &NaivePlanner{} }

// Plan computes creates → updates → deletes, each class sorted by
// resource id (#req-plan-order), covering every divergence exactly
// once (#req-plan-total).
//
//spec:implements spec://go-demo/PROP-001#req-plan-total r=1
func (p *NaivePlanner) Plan(
	ctx context.Context,
	desired, actual seams.State,
) ([]seams.Action, error) {
	_ = ctx
	if desired == nil || actual == nil {
		return nil, seams.NewNilStateError()
	}
	var creates, updates, deletes []seams.Action
	for id, want := range desired {
		have, exists := actual[id]
		switch {
		case !exists:
			creates = append(creates, seams.Action{Op: seams.OpCreate, ID: id, To: want})
		case have != want:
			updates = append(updates, seams.Action{Op: seams.OpUpdate, ID: id, To: want})
		}
	}
	for id := range actual {
		if _, wanted := desired[id]; !wanted {
			deletes = append(deletes, seams.Action{Op: seams.OpDelete, ID: id})
		}
	}
	byID := func(actions []seams.Action) {
		sort.Slice(actions, func(i, j int) bool { return actions[i].ID < actions[j].ID })
	}
	byID(creates)
	byID(updates)
	byID(deletes)
	plan := make([]seams.Action, 0, len(creates)+len(updates)+len(deletes))
	plan = append(plan, creates...)
	plan = append(plan, updates...)
	plan = append(plan, deletes...)
	return plan, nil
}
