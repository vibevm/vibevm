// Package sim is the reconciler's steppable reference model
// (scaffold H): an in-memory world a reader can EXECUTE instead of
// mentally simulating — feed a desired state, call Step, watch
// convergence. It doubles as the seam's store fake in every test.
//
//spec:scope spec://go-demo/PROP-001#req-converge r=1
package sim
