# go-ai-native-mcp — floor

_Captured 2026-07-28 against `vibevm/vibepacks/org.vibevm.ai-native/go-ai-native-mcp/v0.1.0/`._

```console
$ go-ai-native floor --keep-going
FAIL	./... [setup failed]
FAIL

=== gofmt -l . ===
  gofmt: unformatted: tools\go-extract\test\fixtures\dirty\internal\cells\plan\plan.go
floor: `gofmt` FAILED

=== go vet ./... ===
pattern ./...: directory prefix . does not contain main module or its selected dependencies
floor: `vet` FAILED

=== go test ./... ===
# ./...
pattern ./...: directory prefix . does not contain main module or its selected dependencies
floor: `tests` FAILED

=== staticcheck ./... && exhaustive ./... ===
floor: the step's tool did not spawn (program not found) — go install honnef.co/go/tools/cmd/staticcheck@latest (or disable the step with a reason in conform.toml [go].floor_disable)
floor: the step's tool did not spawn (program not found) — go install github.com/nishanths/exhaustive/cmd/exhaustive@latest (or disable the step with a reason in conform.toml [go].floor_disable)
floor: `staticcheck` FAILED

=== go-ai-native-conform check ===
go-ai-native-conform: NO conform.toml — topology default in force (roots = ["."], no cells gate); run `go-ai-native init` to write a starting policy.
go-ai-native-conform: extracted 5 file(s), 0 cached (producer go-extract-1).
  go-ai-native-conform: NEW go-unsafe-in-domain tools/go-extract/test/fixtures/dirty/internal/cells/plan/plan.go:17 — violates REQ discipline://go-ai-native-lang/guide#errors: a seam error type without a Spec field cannot cite its REQ; fix surface: carry the violated spec:// URI (Code + Spec + Err) and render it
  go-ai-native-conform: NEW go-unsafe-in-domain tools/go-extract/test/fixtures/dirty/internal/cells/plan/plan.go:36 — violates REQ discipline://go-ai-native-lang/guide#errors: matching on an error's string couples to prose, not contract; fix surface: consume the seam's closed error set via errors.As on its Code
  go-ai-native-conform: NEW go-unsafe-in-domain tools/go-extract/test/fixtures/dirty/internal/cells/plan/plan.go:39 — violates REQ discipline://go-ai-native-lang/guide#errors: matching on an error's string couples to prose, not contract; fix surface: consume the seam's closed error set via errors.As on its Code
  go-ai-native-conform: NEW go-unsafe-in-domain tools/go-extract/test/fixtures/dirty/internal/cells/plan/plan.go:42 — violates REQ discipline://go-ai-native-lang/guide#bans: a suppression without a reason is unrecorded testimony; fix surface: append the reason (`//lint:ignore <Check> <reason>`), or fix the finding
  go-ai-native-conform: NEW go-unsafe-in-domain tools/go-extract/test/fixtures/dirty/internal/cells/plan/plan_test.go:6 — violates REQ discipline://go-ai-native-lang/guide#replacement: `t.Skip` hides both regressions and healings; fix surface: record the failure in discipline/registry/tests-baseline.json instead
go-ai-native-conform check: 5 finding(s) in scope <workspace> ({"go-unsafe-in-domain": 5}), 0 frozen in baseline, 5 new; SARIF at target\conform\report-go.sarif.
go-ai-native-conform: 5 new finding(s) against the baseline
floor: `conform` FAILED

=== go-ai-native-specmap --check ===
go-ai-native-specmap: NO specmap.toml — placeholder namespace `project` in force and the orphan gate is off; run `go-ai-native init` to write a starting policy.
reading .\specmap.json — run `rust-ai-native-specmap` (or your project's wrapper) first
floor: `specmap` FAILED

floor: no tests baseline at discipline/registry/tests-baseline.json — the test-gate step arms when `go-ai-native init` writes it
Error: floor: 6 step(s) failed: gofmt, vet, tests, staticcheck, conform, specmap
EXIT=1
```

**Scope:** every fact under `vibevm/vibepacks/org.vibevm.ai-native/go-ai-native-mcp/v0.1.0/` that this run bears on. The anchor list is not maintained here — a verdict cites this file in its `ev[]`, and the reverse index is derived from the verdict maps at the phase close (PHASE-C-BATCH-PLAN.md §5).
