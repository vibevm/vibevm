// Package registry is deliberately untagged: its export is the orphan
// the ratchet gate must catch (no //spec: marker, no package scope).
package registry

// Planner is a naked export.
func Planner() string { return "naive" }
