package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// fixturesRoot is the test corpus root (clean + dirty cells) relative
// to the package directory (where `go test` runs).
const fixturesRoot = "test/fixtures"

// extractFixture runs the extractor over a fixture file and returns its facts.
func extractFixture(t *testing.T, rel string) []fact {
	t.Helper()
	src, err := os.ReadFile(filepath.FromSlash(filepath.Join(fixturesRoot, rel)))
	if err != nil {
		t.Fatalf("read fixture %s: %v", rel, err)
	}
	return extractSource(rel, src).Facts
}

// hasUnsafeKind reports whether fs carries a go_unsafe fact of the kind.
func hasUnsafeKind(fs []fact, want string) bool {
	for _, f := range fs {
		if f.Fact == "go_unsafe" && f.Kind == want {
			return true
		}
	}
	return false
}

func unsafeLine(fs []fact, want string) uint32 {
	for _, f := range fs {
		if f.Fact == "go_unsafe" && f.Kind == want {
			return f.Line
		}
	}
	return 0
}

func conformanceFacts(fs []fact) []fact {
	var out []fact
	for _, f := range fs {
		if f.Fact == "go_conformance" {
			out = append(out, f)
		}
	}
	return out
}

// --- A. message half: seam_error_message_no_req -----------------------

// The dirty fixture's PlanError.Error() renders "plan: %d" — no REQ
// token — so the message half fires, at the Error() method line (line
// 22), distinct from the structure half's type-decl line (17), so the
// gate keys the two halves as separate findings.
func TestSeamErrorMessage_DirtyFixtureReds(t *testing.T) {
	fs := extractFixture(t, "dirty/internal/cells/plan/plan.go")
	if !hasUnsafeKind(fs, "seam_error_message_no_req") {
		t.Fatal("dirty fixture must emit seam_error_message_no_req")
	}
	if line := unsafeLine(fs, "seam_error_message_no_req"); line != 22 {
		t.Fatalf("message half anchors the Error() method line 22, got %d", line)
	}
	// A type missing both halves reports two findings at distinct sites.
	if !hasUnsafeKind(fs, "seam_error_missing_req") {
		t.Fatal("structure half must still fire alongside the message half")
	}
}

// The clean fixture's GreetError.Error() renders "violates REQ %s" —
// the Class-F marker (the URI rides the Spec field, not a literal
// spec://) — so the message half stays green.
func TestSeamErrorMessage_CleanFixtureGreens(t *testing.T) {
	fs := extractFixture(t, "clean/internal/cells/greet/greet.go")
	if hasUnsafeKind(fs, "seam_error_message_no_req") {
		t.Fatal("clean fixture must not emit seam_error_message_no_req")
	}
	if hasUnsafeKind(fs, "seam_error_missing_req") {
		t.Fatal("clean fixture must not emit seam_error_missing_req")
	}
}

// A literal spec:// in the Error() body satisfies the message half too.
func TestSeamErrorMessage_LiteralSpecURIGreens(t *testing.T) {
	src := []byte(`package plan

type E struct{ Spec string }

func (e *E) Error() string { return "violates spec://" + e.Spec }
`)
	if hasUnsafeKind(extractSource("e.go", src).Facts, "seam_error_message_no_req") {
		t.Fatal("a literal spec:// in Error() must satisfy the message half")
	}
}

// --- B. conformance assertion: go_conformance ------------------------

func TestGoConformance_LiveSelectorForm(t *testing.T) {
	src := []byte("package plan\n\n" +
		"var _ seams.Planner = (*BatchPlanner)(nil) // silent conformance made loud\n")
	got := conformanceFacts(extractSource("internal/cells/plan/planner.go", src).Facts)
	if len(got) != 1 {
		t.Fatalf("live form emits one go_conformance, got %d: %+v", len(got), got)
	}
	if got[0].Seam != "seams.Planner" {
		t.Errorf("seam = %q, want seams.Planner", got[0].Seam)
	}
	if got[0].Impl != "BatchPlanner" {
		t.Errorf("impl = %q, want BatchPlanner", got[0].Impl)
	}
	if got[0].Line != 3 {
		t.Errorf("line = %d, want 3", got[0].Line)
	}
}

func TestGoConformance_BareInterfaceName(t *testing.T) {
	src := []byte("package plan\n\nvar _ Planner = (*BatchPlanner)(nil)\n")
	got := conformanceFacts(extractSource("planner.go", src).Facts)
	if len(got) != 1 || got[0].Seam != "Planner" || got[0].Impl != "BatchPlanner" {
		t.Fatalf("bare interface name: %+v, want seam=Planner impl=BatchPlanner", got)
	}
}

// The codemod near-misses must NOT match — they keep a symbol
// referenced from tests and lack the exact `= (*Ident)(nil)` RHS.
func TestGoConformance_NearMissesDoNotMatch(t *testing.T) {
	cases := map[string]string{
		"blank no type": "package plan\n\nvar _ = New\n",
		"star type":     "package plan\n\nvar _ *Type\n",
	}
	for name, src := range cases {
		t.Run(name, func(t *testing.T) {
			got := conformanceFacts(extractSource("gen.go", []byte(src)).Facts)
			if len(got) != 0 {
				t.Fatalf("near-miss %q must not emit go_conformance: %+v", name, got)
			}
		})
	}
}

// --- C. cell manifest: //spec:cell → item attrs ----------------------

// A //spec:cell directive in a type's doc comment rides the owning
// item's Attrs as the raw `key=value …` text — the Go cell manifest the
// bridge renders into the engine's cell(seam=,variant=) attr, so one
// cell-name rule reads Rust #[cell] and Go //spec:cell identically.
func TestSpecCellDirective_AttachesToOwningItem(t *testing.T) {
	src := []byte("package plan\n\n" +
		"// BatchPlanner is the batch Planner cell.\n" +
		"//\n" +
		"//spec:cell seam=Planner variant=batch replaces=naive flag=planner\n" +
		"type BatchPlanner struct{}\n")
	fs := extractSource("internal/cells/plan/planner.go", src).Facts
	var cell *fact
	for i := range fs {
		if fs[i].Fact == "item" && fs[i].Symbol == "BatchPlanner" {
			cell = &fs[i]
			break
		}
	}
	if cell == nil {
		t.Fatal("BatchPlanner item fact not emitted")
	}
	if len(cell.Attrs) != 1 {
		t.Fatalf("cell item carries one attr, got %+v", cell.Attrs)
	}
	want := "seam=Planner variant=batch replaces=naive flag=planner"
	if cell.Attrs[0] != want {
		t.Errorf("attr = %q, want the raw directive args verbatim", cell.Attrs[0])
	}
}

// A free-floating //spec:cell (blank line before the declaration) has no
// owning item, so it attaches to nothing — the directive is dropped, not
// invented onto a neighbour.
func TestSpecCellDirective_WithoutOwnerIsDropped(t *testing.T) {
	src := []byte("package plan\n\n" +
		"//spec:cell seam=Planner variant=batch\n\n" +
		"type Other struct{}\n")
	for _, f := range extractSource("planner.go", src).Facts {
		if f.Fact == "item" && len(f.Attrs) > 0 {
			t.Fatalf("orphan directive must not attach: %+v", f)
		}
	}
}

// --- D. swept test matrices: test_sweep (R-060) ----------------------

// The dirty fixture's plan_test.go carries BOTH swept-matrix halves: a
// `1 << 3` bit-mask loop AND a three-deep C-style for nest — the two
// declared-test-matrices violations — emitted only because the file is a
// _test.go (in_test).
func TestSweptMatrix_DirtyTestFileEmitsBitmaskAndNestedLoops(t *testing.T) {
	fs := extractFixture(t, "dirty/internal/cells/plan/plan_test.go")
	var sweeps []fact
	for _, f := range fs {
		if f.Fact == "test_sweep" {
			sweeps = append(sweeps, f)
		}
	}
	if len(sweeps) != 2 {
		t.Fatalf("plan_test.go emits two test_sweep facts (bitmask + nested-loops), got %+v", sweeps)
	}
	byKind := map[string]fact{}
	for _, s := range sweeps {
		byKind[s.Kind] = s
	}
	bm, ok := byKind["bitmask"]
	if !ok {
		t.Fatalf("bitmask sweep must fire, got %+v", sweeps)
	}
	if bm.Detail == "" || !strings.Contains(bm.Detail, "<<") {
		t.Errorf("bitmask detail = %q, want the rendered bit-mask bound", bm.Detail)
	}
	nl, ok := byKind["nested-loops"]
	if !ok {
		t.Fatalf("nested-loops sweep must fire, got %+v", sweeps)
	}
	if nl.Detail != "3" {
		t.Errorf("nested-loops detail = %q, want depth 3", nl.Detail)
	}
}

// A declared matrix (a table + one range loop) emits nothing, and a
// bit-mask loop OUTSIDE a _test.go file never fires (test context only).
func TestSweptMatrix_DeclaredAndNonTestAreSilent(t *testing.T) {
	declared := []byte("package plan\n\n" +
		"func table() {\n" +
		"	cases := []struct{ in string }{{\"a\"}, {\"b\"}}\n" +
		"	for _, tc := range cases { _ = tc }\n" +
		"}\n")
	for _, f := range extractSource("plan.go", declared).Facts {
		if f.Fact == "test_sweep" {
			t.Fatalf("a declared matrix never fires, got %+v", f)
		}
	}
	nonTest := []byte("package plan\n\n" +
		"func gen() {\n" +
		"	for mask := 0; mask < 1<<3; mask++ { _ = mask }\n" +
		"}\n")
	for _, f := range extractSource("plan.go", nonTest).Facts {
		if f.Fact == "test_sweep" {
			t.Fatalf("a bit-mask outside a _test.go file never fires, got %+v", f)
		}
	}
}

// Three nested C-style for-loops (generated axes) emit the nested-loops
// signal — the Cartesian half of R-060 — once, at the third loop's depth.
func TestSweptMatrix_NestedForStmtsEmitNestedLoops(t *testing.T) {
	src := []byte("package plan\n\n" +
		"func TestNestedSweep(t *testing.T) {\n" +
		"	for i := 0; i < 2; i++ {\n" +
		"		for j := 0; j < 2; j++ {\n" +
		"			for k := 0; k < 2; k++ { _ = i }\n" +
		"		}\n" +
		"	}\n" +
		"}\n")
	var nested []fact
	for _, f := range extractSource("plan_test.go", src).Facts {
		if f.Fact == "test_sweep" && f.Kind == "nested-loops" {
			nested = append(nested, f)
		}
	}
	if len(nested) != 1 {
		t.Fatalf("three nested ForStmts emit one nested-loops, got %+v", nested)
	}
	if nested[0].Detail != "3" {
		t.Errorf("detail = %q, want depth 3", nested[0].Detail)
	}
}

// Three nested for-range loops over DECLARED collections (literal slices)
// emit nothing — the narrowing (R-060): a declared axis does not count, so
// exhausting a closed set by nesting for-range is compliant. This is the
// exact shape the host's progress-core / vibe-workspace tests use.
func TestSweptMatrix_NestedRangeOverDeclaredAxesIsSilent(t *testing.T) {
	src := []byte("package plan\n\n" +
		"func TestDeclaredAxes(t *testing.T) {\n" +
		"	for _, a := range []int{0, 1} {\n" +
		"		for _, b := range []int{0, 1} {\n" +
		"			for _, c := range []int{0, 1} { _ = a }\n" +
		"		}\n" +
		"	}\n" +
		"}\n")
	for _, f := range extractSource("plan_test.go", src).Facts {
		if f.Fact == "test_sweep" {
			t.Fatalf("a declared-axis for-range nest never fires, got %+v", f)
		}
	}
}

// The mixed case (R-060 refinement): a generated ForStmt nested inside a
// declared for-range still counts — depth tracks generated axes only, so
// three ForStmts at any level (here under one for-range wrapper) fire.
func TestSweptMatrix_MixedNestCountsOnlyGeneratedAxes(t *testing.T) {
	src := []byte("package plan\n\n" +
		"func TestMixed(t *testing.T) {\n" +
		"	for _, tc := range []int{0, 1} {\n" +
		"		for i := 0; i < 2; i++ {\n" +
		"			for j := 0; j < 2; j++ {\n" +
		"				for k := 0; k < 2; k++ { _ = tc }\n" +
		"			}\n" +
		"		}\n" +
		"	}\n" +
		"}\n")
	var nested []fact
	for _, f := range extractSource("plan_test.go", src).Facts {
		if f.Fact == "test_sweep" && f.Kind == "nested-loops" {
			nested = append(nested, f)
		}
	}
	if len(nested) != 1 || nested[0].Detail != "3" {
		t.Fatalf("three ForStmts under a for-range wrapper fire one nested-loops at depth 3, got %+v", nested)
	}
}
