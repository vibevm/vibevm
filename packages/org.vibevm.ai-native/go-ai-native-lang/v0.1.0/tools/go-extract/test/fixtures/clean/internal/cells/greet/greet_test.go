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
