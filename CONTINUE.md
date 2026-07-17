# CONTINUE.md — cold-resume checkpoint (2026-07-17 evening, GO-AI-NATIVE CLOSED)

> `spec/WAL.md` is the canonical living state; if this snapshot and the WAL diverge, the WAL wins.

## TL;DR

**The GO-AI-NATIVE campaign is CLOSED end-to-end in one session** — Go is the
Discipline's third supported language at full Rust/TS parity, per the owner's mandate
(«реализация на уровне с Rust и Typescript», agentic-only tcg, демо + тест-корпус,
end-to-end по собственному плану). Everything is committed on `main` and pushed; host
self-check green; the campaign plan's REPORT is written
(`spec/terraforms/GO-AI-NATIVE-PLAN-v0.1.md` §12–§13).

## What landed (12 commits, 05976fa…)

- **Packages** (`packages/org.vibevm.ai-native/`): `go-ai-native-lang` 0.1.0 — the
  stack: GUIDE-AI-NATIVE-GO, 9 cards, TCG-ORACLE-GO/TCG-PROTOCOL-GO, briefs
  (vibe-agentic-tcg-go full-parity; go-ai-native-tcg token-level STUB very-far-future),
  sweep/terraform skills, 8 crates (cli/conform/conform-frontend/extract-bridge/
  specmap/specmap-scan/tcg/tcg-bridge) + `tools/go-extract/extract.go` (stdlib-only);
  `go-ai-native-mcp` 0.1.0 — 17 tools; aggregator `go-ai-native` 0.1.0.
- **core-ai-native 0.8.0**: Fact::GoUnsafe, GoConfig ([go] table), the Go walk
  (depth-0 guard!), rules/go.rs (GoUnsafeInDomain cell-scoped kinds, GoCellIsolation —
  no sibling imports at all, no in-cell seam). 0.7.0 untouched (Rust/TS pins ^0.7).
- **The pilot** `research/go-demo`: a miniature reconciler (K8s rehearsal) —
  `go-ai-native floor` ALL GREEN (7 steps), one deliberate frozen finding (the
  differential fuzz oracle imports the sibling it replaces — replacement-window debt),
  live tcg one-shot: dirty overlay → exit 1 + Class-F advice; clean → exit 0.
- **Live chains proven**: gopls bridge green at first attempt (seeded type error via
  pure overlay, hover, no-zombie shutdown); finding-parity relay=gate; MCP server
  replay; fresh-go-project init→gates walk.

## Machine facts (this box)

go 1.26.5 at `C:/opt/go` (NOT on PATH — export `PATH="/c/opt/go/bin:/c/opt/gotools:$PATH"`
or set `GO_AI_NATIVE_GO`); gopls 0.23 / staticcheck 0.8-rc / exhaustive at
`C:/opt/gotools` (exhaustive built from master with bumped x/tools — v0.12.0 does not
compile under go 1.26). `-race` needs cgo here (no gcc) — the demo has no goroutines,
so the guide's rule is not engaged.

## Candidate next steps (a RESUME is report-then-wait)

1. **Registry publishing** of the three go packages + core 0.8.0 (owner decision,
   PROP-002; never autonomous).
2. **Bench-corpus seeding** (REPORT P3's deferral): capture the first live
   gopls-vs-floor asymmetry case in `go-ai-native-tcg bench` corpus form.
3. **Rust/TS stacks onto core ^0.8** when their next versions bump (no action now).
4. Host Go code (Kubernetes work) eventually consumes the stack via `vibe install`.

## Non-obvious findings (do not re-learn)

- Go's `flag` package stops at the first non-flag arg → the extractor's `--files` is a
  boolean marker; probe = `--files` with zero args.
- mdspec mints doc-ids from the document HEADING (`# PROP-001 — …` → PROP-001), not the
  filename — tags citing the filename dangle.
- A materialised Go tool inside a consumer module needs a go.mod cut-off, or the
  consumer's `./...` compiles it as project code.
- A literal `.` scan root is eaten by hidden-dir filters without a depth-0 guard.
- exit codes after `| tail` are the tail's — check the real one (`> /dev/null; echo $?`).

## Quick-start

```sh
git status -sb && git log --oneline -12
bash tools/self-check.sh                 # host floor
export PATH="/c/opt/go/bin:/c/opt/gotools:$PATH"
cargo test --manifest-path packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/Cargo.toml
cd research/go-demo && \
  /c/Users/olegc/gits/vibevm/packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/target/debug/go-ai-native.exe floor
```

## Pointer

- **Canonical living state:** `spec/WAL.md` (the evening 2026-07-17 checkpoint).
- **Campaign record:** `spec/terraforms/GO-AI-NATIVE-PLAN-v0.1.md` (CLOSED; §12 REPORT,
  §13 ledger).
