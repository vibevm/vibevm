// Package naiveplanner is the reference planner cell: the plainest
// possible diff, one pass per operation class. It exists to be READ —
// and to be the differential oracle's old side when batchplanner
// replaces it (scaffold D).
//
//spec:scope spec://go-demo/PROP-001#req-planner-seam r=1
package naiveplanner
