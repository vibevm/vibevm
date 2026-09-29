// Package wiring is the go-flag-sites red exhibit: a package OUTSIDE
// cells_dir, other than the registry, that imports a cell directly. A
// cell is born in one place — the registry — so this import is a
// selection flag that leaked past the composition root.
package wiring

import "example.com/demo/internal/cells/plan"

// Reach references the plan cell directly instead of going through the
// registry switch — the violation go-flag-sites catches.
var _ = plan.Solve
