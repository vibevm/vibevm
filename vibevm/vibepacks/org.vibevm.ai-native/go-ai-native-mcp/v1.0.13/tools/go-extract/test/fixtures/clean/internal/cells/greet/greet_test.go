package greet

import "testing"

// TestDeclaredMatrix is the compliant shape declared-test-matrices (R-060)
// checks for: the cases are DATA (a table), iterated once — the count is
// visible and grows linearly with the record, never a 2^n sweep.
func TestDeclaredMatrix(t *testing.T) {
	cases := []struct {
		name string
		in   string
	}{
		{"empty", ""},
		{"world", "world"},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			_ = tc.in
		})
	}
}

// TestDeclaredAxes is the narrowing's green half (R-060): three nested
// for-range loops over DECLARED axes (literal slices), not generated ranges.
// The cases are written as data — the product of the axes is merely
// expressed by the nesting — so this is compliant and emits nothing, where a
// bare-depth heuristic would have red'd it. This is the exact shape the
// `progress-core` and `vibe-workspace` host tests use (a closed set exhausted
// by nesting), and it must stay green.
func TestDeclaredAxes(t *testing.T) {
	for _, a := range []int{0, 1} {
		for _, b := range []int{0, 1} {
			for _, c := range []int{0, 1} {
				_ = a*4 + b*2 + c
			}
		}
	}
}
