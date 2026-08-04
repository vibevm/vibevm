// A declared test matrix (R-060): the cases are DATA (a table), iterated
// once — the compliant shape declared-test-matrices checks for. The count
// is visible and grows linearly with the record, never a 2^n sweep.
const cases: ReadonlyArray<{ in: string }> = [{ in: "a" }, { in: "b" }];
for (const tc of cases) {
  // one declared case at a time
}
