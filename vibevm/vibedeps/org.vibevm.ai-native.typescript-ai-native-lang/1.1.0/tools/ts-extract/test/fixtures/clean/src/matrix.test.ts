// A declared test matrix (R-060): the cases are DATA (a table), iterated
// once — the compliant shape declared-test-matrices checks for. The count
// is visible and grows linearly with the record, never a 2^n sweep.
const cases: ReadonlyArray<{ in: string }> = [{ in: "a" }, { in: "b" }];
for (const tc of cases) {
  // one declared case at a time
}

// The narrowing's green half (R-060): three nested for-of loops over
// DECLARED axes (literal arrays), not generated ranges. The cases are
// written as data — the product of the axes is merely expressed by the
// nesting — so this is compliant and emits nothing, where a bare-depth
// heuristic would have red'd it. This is the exact shape a closed-set
// exhaustion takes (the host's progress-core / vibe-workspace tests), and
// it must stay green.
for (const a of [0, 1]) {
  for (const b of [0, 1]) {
    for (const c of [0, 1]) {
      // one declared combination
    }
  }
}
