package seams

import "fmt"

// PlanErrorCode is the Planner seam's closed failure set (#req-errors).
type PlanErrorCode int

const (
	// ErrNilState marks a nil desired or actual state — the seam's one
	// expected failure.
	ErrNilState PlanErrorCode = iota + 1
)

// String names the code for rendering.
func (c PlanErrorCode) String() string {
	switch c {
	case ErrNilState:
		return "nil-state"
	}
	return "unknown"
}

// PlanError is the seam's error value: Code + the violated REQ URI +
// the wrapped cause (Class F — the message IS a spec pointer).
//
//spec:implements spec://go-demo/PROP-001#req-errors r=1
type PlanError struct {
	Code PlanErrorCode
	Spec string
	Err  error
}

// Error renders the Class-F grammar: the failure, the violated REQ,
// and the fix surface.
func (e *PlanError) Error() string {
	return fmt.Sprintf(
		"plan: %s: violates REQ %s; fix surface: hand the planner non-nil states",
		e.Code, e.Spec,
	)
}

// Unwrap keeps the cause chain machine-walkable.
func (e *PlanError) Unwrap() error { return e.Err }

// NewNilStateError is the blessed constructor for the seam's one
// expected failure.
func NewNilStateError() *PlanError {
	return &PlanError{
		Code: ErrNilState,
		Spec: "spec://go-demo/PROP-001#req-errors",
	}
}
