package batchplanner

import (
	"context"
	"sort"

	"reconcile-demo/internal/seams"
)

// BatchPlanner computes the same plan as naiveplanner in one pass over
// the merged key set. Construct via New. Pure, like its predecessor.
type BatchPlanner struct{}

var _ seams.Planner = (*BatchPlanner)(nil) // silent conformance made loud

// New is the blessed construction path.
func New() *BatchPlanner { return &BatchPlanner{} }

// Plan walks the union of both key sets once, then orders the classes
// (#req-plan-order); agreement with naiveplanner is pinned by the
// differential fuzz oracle (#req-plan-total).
//
//spec:implements spec://go-demo/PROP-001#req-plan-total r=1
func (p *BatchPlanner) Plan(
	ctx context.Context,
	desired, actual seams.State,
) ([]seams.Action, error) {
	_ = ctx
	if desired == nil || actual == nil {
		return nil, seams.NewNilStateError()
	}
	union := make(map[seams.ResourceID]struct{}, len(desired)+len(actual))
	for id := range desired {
		union[id] = struct{}{}
	}
	for id := range actual {
		union[id] = struct{}{}
	}
	ids := make([]seams.ResourceID, 0, len(union))
	for id := range union {
		ids = append(ids, id)
	}
	sort.Slice(ids, func(i, j int) bool { return ids[i] < ids[j] })

	var creates, updates, deletes []seams.Action
	for _, id := range ids {
		want, wanted := desired[id]
		have, exists := actual[id]
		switch {
		case wanted && !exists:
			creates = append(creates, seams.Action{Op: seams.OpCreate, ID: id, To: want})
		case wanted && have != want:
			updates = append(updates, seams.Action{Op: seams.OpUpdate, ID: id, To: want})
		case !wanted && exists:
			deletes = append(deletes, seams.Action{Op: seams.OpDelete, ID: id})
		}
	}
	plan := make([]seams.Action, 0, len(creates)+len(updates)+len(deletes))
	plan = append(plan, creates...)
	plan = append(plan, updates...)
	plan = append(plan, deletes...)
	return plan, nil
}
