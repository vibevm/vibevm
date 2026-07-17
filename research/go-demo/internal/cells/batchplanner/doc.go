// Package batchplanner is the optimized planner cell: one pass over a
// merged key set instead of naiveplanner's three. It REPLACES
// naiveplanner behind the seam (the cell manifest below), so it ships
// the differential fuzz oracle asserting agreement (scaffold D — the
// replacement protocol).
//
//spec:scope spec://go-demo/PROP-001#req-planner-seam r=1
//spec:cell seam=Planner variant=batch replaces=naive flag=planner
package batchplanner
