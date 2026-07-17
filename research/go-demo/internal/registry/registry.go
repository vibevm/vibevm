// Package registry is the composition root's selector: the ONLY
// package that imports cell packages, and the only flag reader
// (R-001 — one switch is the system's table of contents).
//
//spec:scope spec://go-demo/PROP-001#req-planner-seam r=1
package registry

import (
	"reconcile-demo/internal/cells/batchplanner"
	"reconcile-demo/internal/cells/naiveplanner"
	"reconcile-demo/internal/seams"
)

// PlannerKind is the closed variant set the flag selects from.
type PlannerKind int

const (
	// PlannerNaive selects the reference three-pass cell.
	PlannerNaive PlannerKind = iota + 1
	// PlannerBatch selects the one-pass replacement cell.
	PlannerBatch
)

// Config is the runtime tier: read once in main, passed down — never
// re-read ambiently (GUIDE §6).
type Config struct {
	Planner PlannerKind
}

// Planner selects the cell. provenance: default | env | cli — the
// composition root resolves it before calling here.
func Planner(cfg Config) seams.Planner {
	switch cfg.Planner {
	case PlannerBatch:
		return batchplanner.New()
	case PlannerNaive:
		return naiveplanner.New()
	}
	return naiveplanner.New()
}
