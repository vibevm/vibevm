package seams

import "context"

// ResourceID names a resource — a defined type, so a bare string (or
// a same-shaped Revision) fails the build at the seam (scaffold B:
// Go's nominal types are the brand, for free).
type ResourceID string

// Revision is a resource's monotonically increasing revision.
type Revision int64

// State is one world snapshot: resource → revision.
type State map[ResourceID]Revision

// ActionOp is the closed operation set of a plan. The exhaustive
// linter carries the switch coverage the compiler cannot (GUIDE §5).
type ActionOp int

const (
	// OpCreate materialises a missing resource at the desired revision.
	OpCreate ActionOp = iota + 1
	// OpUpdate moves a stale resource to the desired revision.
	OpUpdate
	// OpDelete removes a resource absent from the desired state.
	OpDelete
)

// String renders the op for transcripts and goldens.
func (op ActionOp) String() string {
	switch op {
	case OpCreate:
		return "create"
	case OpUpdate:
		return "update"
	case OpDelete:
		return "delete"
	}
	return "unknown"
}

// Action is one planned step.
type Action struct {
	Op ActionOp
	ID ResourceID
	// To is the target revision; zero for deletes.
	To Revision
}

// Planner computes the action list transforming actual into desired
// (the seam both planner cells implement).
//
//spec:implements spec://go-demo/PROP-001#req-planner-seam r=1
type Planner interface {
	// Plan returns the deterministically ordered actions
	// (#req-plan-order) whose application yields desired exactly
	// (#req-plan-total). Both states may be empty; nil states are the
	// seam's one expected failure (ErrNilState).
	Plan(ctx context.Context, desired, actual State) ([]Action, error)
}

// Store owns the actual state — an injected capability; cells never
// reach ambient storage.
type Store interface {
	// Snapshot returns a copy of the current state.
	Snapshot(ctx context.Context) (State, error)
	// Apply performs one action.
	Apply(ctx context.Context, action Action) error
}

// Clock is the injected time capability (cells never call time.Now).
type Clock interface {
	// UnixNano stamps transcripts deterministically in tests.
	UnixNano() int64
}

// Clone copies a state so planners can never alias caller memory.
func Clone(s State) State {
	out := make(State, len(s))
	for id, rev := range s {
		out[id] = rev
	}
	return out
}
