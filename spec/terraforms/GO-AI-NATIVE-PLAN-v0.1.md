# GO-AI-NATIVE-PLAN v0.1 — Go as the third supported language of the Discipline

**Status: CLOSED 2026-07-17 — all phases landed, floor green on the pilot, REPORT
written (§12).** Deferred by name: registry publishing; host installation of the go
stack; the bench corpus seeding (see §12 P3); token-level tcg (very-far-future).
_The §13 ledger is the record._

> Read-first: `CLAUDE.md` → `spec/boot/` → `spec/WAL.md`, then this plan. The WAL
> supersedes this plan's status line wherever they diverge. The three source corpora this
> campaign projects from: `packages/org.vibevm.ai-native/core-ai-native/v0.7.0/spec/`,
> `…/rust-ai-native-lang/v0.7.0/`, `…/typescript-ai-native-lang/v0.6.0/`.

## 1. Why this exists {#why}

The Discipline v0.2 ships two full language stacks — Rust (pilot, v0.7.0) and TypeScript
(v0.6.0) — each carrying a GUIDE, nine scaffold cards, the conform/specmap gates, an
umbrella CLI, an agentic type oracle (tcg), an MCP server, and agent skills. Go exists
only as a genre-complete but unexercised legacy projection
(`core-ai-native/spec/legacy-projections/GUIDE-GO-v0.1.md`), with no toolchain, no cards,
no oracle, no pilot.

This campaign promotes Go to the **third supported language**: a full
`go-ai-native` package family at parity with the Rust and TypeScript stacks — spec corpus
AND working code AND a pilot demo tree with a test corpus that proves the whole chain
end-to-end. vibevm itself has no Go code yet (it will, when the Kubernetes work begins);
the pilot is therefore a purpose-built research demo, and its domain is chosen to
rehearse exactly that future (a miniature reconciler).

## 2. Owner mandate (verbatim, 2026-07-17) {#mandate}

1. «Задача: аналогично rust-ai-native и typescript-ai-native разработай go-ai-native (для
   языка Go). […] Нужно реализовать примерно те же идеи, примерно те же карточки, но со
   спецификой Go. Если возможно, придумай как реализовать tcg-инструментарий. Вначале
   запланируй, что будешь делать, покажи мне это и сохрани план в файл.» — «Никому не
   делегируй, сделай всё сама в один поток без агентов.»
2. «Для того, чтобы проверить как всё это работает - сделай маленький исследовательский
   демо-проект на Go, ну и напиши корпус тестов которые всё это проверяют. В самой VibeVM
   нет пока кода на го (но будет потом, когда будем работать с Kubernetes) […]»
3. «Реализация для Go должна быть хороша - на уровне с Rust и Typescript. Это третий
   поддерживаемый нами язык.»
4. «План должен быть таким, чтобы сразу выбирать самый интересный и богатый вариант. На
   время решения задачи я отойду от компьютера, поэтому решать ты будешь ее сама по
   написанному тобой же плану, end to end.»
5. «TCG если реализовывать, то только agentic, для token level мы пока не готовы, это
   very-far-future.»
6. «Сам по себе Go установлен в C:/opt/go. Но если тебе понадобятся какие-то
   дополнительные инструменты, можно скачать их в C:/opt/gotools. Только удостоверься,
   что они опенсорсные и кроссплатформенные.»
7. «Структура того что ты сделаешь должна лечь в vibevm пакеты, и структура пакетов
   должна повторять то, что есть у rust и typescript.»
8. «Если нужно, не бойся использовать какие-то сложные механизмы типа - написать
   маленький кусочек кода на C++ чтобы построить LLVM IR или типа того. […] Я даю тебе
   разрешение качать в C:/opt/gotools в том числе такие вещи как clang, llvm или что
   угодно еще в интернете, что может помочь. Тем не менее, чем меньше ты завязываешься на
   внешние инструменты - тем лучше. Идеальная ситуация - это как в случае с Rust, когда
   мы обошлись по сути только доступными в Rust экосистеме библиотеками, плюс написали
   некоторое количество своего кода.»
   → *Plan's answer:* the D3/D4 design already sits at the ideal: the only external
   process is gopls (the language's own official analyzer — the exact rust-analyzer
   analog), the fact extractor is stdlib-only Go we author ourselves, and staticcheck /
   exhaustive are optional policy-gated floor steps. No LLVM/clang-class machinery is
   needed anywhere on the critical path.
9. «покрой сам go-ai-native практиками ai-native языков, чтобы это не получился кусок
   нейрослопа. Например заметь, что rust-ai-native доработан, а typescript-ai-native
   нет - там даже спекмарк есть не везде. Пожалуйста, сделай чтобы go-ai-native был
   написан хорошо и с использованием ai-native практик.»
   → **D14 — self-application at the rust-ai-native grade, not the TS grade.** The Go
   stack's own Rust crates carry `specmark::scope!` / `#[spec]` tags citing the
   package's OWN anchored spec units (namespace `go-ai-native-lang`) and the core
   mechanisms; every spec document authored in Phases 2–3 carries stable `{#anchor}`s
   with `req rN` kind lines so those edges resolve; the package self-traces via
   `specmap.toml` (scan_roots over its crates, CLI drivers exempt, orphan gate);
   crate errors are `thiserror` enums whose messages cite the violated REQ and a fix
   surface; every pub seam carries a doctest; files respect the 600-line budget; no
   `unwrap`/`expect` in domain logic. «Goal set: go-ai-native реализован» (owner,
   same date).

Consequences bound into this plan: full code-bearing parity (not spec-only); the tcg line
ships its **agentic** delivery only (the token-level brief stays a stub, exactly like the
TS stack's); no delegation, single-threaded execution; phases are safe stops but the
campaign target is end-to-end in this session.

## 3. Directives & decisions in force {#directives}

- The four repo rules (attribution / conventional commits / atomicity / autonomy) —
  `spec://vibevm/common/PROP-000#commits`.
- Family-prefix naming, PROP-028 §2.4: every named surface carries the `go-ai-native`
  stem as a prefix; the aggregator is content-minimal with exact pins.
- PROP-027 (mcp kind: serving needs no vibe on the machine; exact-pin law), PROP-026
  (the `tcg_*` tools take a `language` parameter — `"go"` is a new enum value, not new
  tools), PROP-025 (lockfile binary dispatch), PROP-024 (code-bearing packages).
- Licensing: UPL-1.0 for everything we ship; permissive-only dependencies. Named
  verdicts: gopls (BSD-3, spawned never linked), staticcheck (MIT), `exhaustive` linter
  (BSD-2), stdlib-only go-extract. **golangci-lint is GPL-3.0 → never vendored, never in
  the floor** (legacy GUIDE-GO §0 already rules it a separate-process dev tool at most).
- The clean-room rule (boot 90-user.md) has no bite here: the agentic oracle stands on
  gopls over LSP and our own engines; the PLDI'25 repo is not needed and not opened.

## 4. Current-state facts (verified 2026-07-17, do not re-discover) {#facts}

- Package trees: a `-lang` stack is `vibe.toml + Cargo.{toml,lock} + LICENSE.md +
  README.md + specmap.toml + spec/{boot,cards,<language>,skills} + crates/* +
  crates/vendor/* [+ tools/* for sidecar languages]`. The mcp package vendors the whole
  stack's crates byte-identically plus `core-ai-native-mcp`. The aggregator is manifest-only.
- Measured code volumes (line counts of `.rs`): TS stack — cli 1668, conform 271,
  conform-frontend 138, extract-bridge 281, specmap 168, specmap-scan 381, tcg 1156,
  tcg-bridge 727, vendor 8008; tools: extract.ts 541, oracle.ts 1174. Rust stack — cli
  2278, conform-frontend 824 (in-process syn), conform 345, specmap 143, tcg 1709,
  tcg-bridge 1899 (the LSP client), env-audit 193, vendor 8008.
- Vendor set (byte-copies of core engines): `core-ai-native-conform`,
  `core-ai-native-specmap`, `core-ai-native-specmark`, `core-ai-native-specmark-grammar`;
  mcp packages add `core-ai-native-mcp`.
- `research/` already hosts `rust-demo`, `ts-demo`, `tcg-bench` → `research/go-demo` is
  the precedented home for the pilot.
- Campaign plans live in `spec/terraforms/` (AGENTIC-TCG-RUST-PLAN-v0.1.md,
  AGENTIC-TCG-TS-PLAN-v0.1.md are the closest ancestors).
- **go is installed at `C:/opt/go` but NOT on this session's PATH** (PATH probes found
  nothing; the owner supplied the location). Extra Go tools go to `C:/opt/gotools`
  (owner directive §2.6) — open-source and cross-platform only: gopls (BSD-3),
  staticcheck (MIT), exhaustive (BSD-2) all qualify. Windows 11, PowerShell + Git Bash;
  the F19 MAX_PATH lesson applies to deep paths.
- The host workspace excludes `packages/` from its Cargo members; host `self-check.sh`
  does not build package crates. Packages build standalone from their own workspace root.

## 5. Target end-state {#target}

```
packages/org.vibevm.ai-native/
  go-ai-native/v0.1.0/                    # family aggregator (manifest-only)
    vibe.toml  LICENSE.md  README.md
  go-ai-native-lang/v0.1.0/               # the stack
    vibe.toml  Cargo.toml  Cargo.lock  LICENSE.md  README.md  specmap.toml
    spec/boot/20-stack-go-ai-native-lang.md
    spec/cards/INDEX.md + scaffold-{a..i}-*.md         (9 cards, Go projection)
    spec/go/GUIDE-AI-NATIVE-GO.md                      (full guide; supersedes the
                                                        legacy projection, which stays
                                                        untouched in core)
    spec/go/mechanisms/TCG-ORACLE-GO-v0.1.md           (gopls process model)
    spec/go/mechanisms/TCG-PROTOCOL-GO-v0.1.md         (wire contract, parity)
    spec/go/tools/vibe-agentic-tcg-go.md               (full 7-section brief)
    spec/go/tools/go-ai-native-tcg.md                  (token-level STUB, very-far-future)
    spec/go/tools/conform-frontend-go.md               (go-extract design brief)
    spec/skills/go-ai-native-sweep/SKILL.md
    spec/skills/go-ai-native-terraform/SKILL.md
    crates/go-ai-native-cli/               # bin go-ai-native: init/floor/health/
                                           #   test-gate/tripwire/trace/fast-loop/codemod
    crates/go-ai-native-conform/           # bin: the standalone gate
    crates/go-ai-native-conform-frontend/  # lib: Frontend impl (id "go-extract")
    crates/go-ai-native-extract-bridge/    # lib: spawns `go run` of the extractor
    crates/go-ai-native-specmap/           # bin: index + orphan gate
    crates/go-ai-native-specmap-scan/      # lib: //spec: directive scanner
    crates/go-ai-native-tcg/               # bin: serve / validate/scope/complete/type / bench
    crates/go-ai-native-tcg-bridge/        # lib: LSP client to the consumer's gopls
    crates/vendor/core-ai-native-{conform,specmap,specmark,specmark-grammar}
    tools/go-extract/extract.go            # stdlib-only sidecar (embedded via include_str!)
                                           # + test fixtures
  go-ai-native-mcp/v0.1.0/                 # the MCP server package
    vibe.toml  Cargo.toml  Cargo.lock  LICENSE.md  README.md  specmap.toml
    spec/tools/discipline-mcp-go.md
    crates/go-ai-native-mcp/ + byte-copies of the stack crates + vendor (+ mcp-core)
    tools/go-extract/                      # byte-copy (the stack vendoring law)

research/go-demo/                          # the pilot: a miniature reconciler
  go.mod  spec/PROP-001-reconciler.md  conform.toml  specmap.toml
  cmd/reconcile/main.go                    # composition root + flag registry
  internal/seams/                          # Store, Planner, Applier, Clock + closed errors
  internal/cells/naiveplanner/             # variant 1
  internal/cells/batchplanner/             # variant 2 (replaces=naive, flag=planner)
  internal/sim/                            # scaffold H: steppable in-memory world
  discipline/registry/{tests-baseline,debt,intent}.json
  discipline/golden/                       # characterization transcript(s)
  *_test.go corpus: units, Example-doctests, fuzz differential, quick properties, -race

spec/terraforms/GO-AI-NATIVE-PLAN-v0.1.md  # this file (+ ledger + report)
```

## 6. Design decisions {#decisions}

- **D1 — typology: «Go prescribes» → the guide's job is gap closure.** The language ships
  with its discipline pre-installed (gofmt, unused imports are compile errors, errors are
  values by culture, `internal/` is compiler-enforced encapsulation, no inheritance). The
  guide closes the four gaps the legacy projection identified: (1) error VALUES but open
  error SETS → seam-owned closed error sets; (2) silent interface conformance →
  `var _ Seam = (*Impl)(nil)` assertions; (3) unowned goroutines → structured ownership
  (errgroup/WaitGroup + context); (4) `init()` blessing side-effectful import → banned in
  cells. *Rejected:* re-deriving the projection from scratch — the legacy GUIDE-GO-v0.1
  is absorbed (all its normative content survives), exactly as GUIDE-AI-NATIVE-RUST
  absorbed GUIDE-RUST-v0.1. The legacy file in core stays untouched (other package, own
  version line); the new guide declares `supersedes`.
- **D2 — specmark carrier: `//spec:` directive comments** (from legacy §5, confirmed):
  gofmt reformats doc comments since 1.19 but preserves `//name:value` directives
  verbatim; godoc hides them. Grammar: `//spec:implements <uri> r=<N>` ·
  `//spec:deviates <uri> r=<N> reason="…"` · `//spec:verifies <uri> r=<N>` ·
  `//spec:scope <uri> r=<N>` (in the package doc block). ≤3 edges per item.
  *Rejected:* doc-comment tags (gofmt re-wraps prose), struct tags (values only, not
  items), a sidecar map (rots — PROP-014 §5.1).
- **D3 — the agentic oracle stands on the consumer's gopls, over LSP.** Resolution order:
  `gopls` on PATH → `$GOBIN/gopls` → `$(go env GOPATH)/bin/gopls` → hard,
  recipe-carrying failure (`go install golang.org/x/tools/gopls@latest`). Installing the
  stack obliges the machine to carry go ≥ 1.24 and gopls (the same posture as
  rust-analyzer / node ≥ 22.6). **Fidelity posture — Go sits BETWEEN TS and Rust, and the
  spec says so:** the TS oracle IS tsc's engine; rust-analyzer is NOT rustc; gopls stands
  on `go/types` — the reference library implementation of the spec, the same framework
  `go vet` uses — while the gc compiler runs `types2` (its ported twin). The delta is a
  deliberately-synchronized pair, far tighter than r-a↔rustc, but not identity: the floor
  (`go build`/`go vet`) remains the truth, verbatim. A documented-gap corpus pins any
  observed silence. *Rejected:* a bespoke go/types sidecar oracle (we would rebuild
  completions/hover that gopls already ships); embedding gopls as a library (version
  pinning, dependency mass — the same grounds that far-backlogged `ra_ap_*`).
- **D4 — conform facts come from `go-extract`: a stdlib-only Go sidecar** (go/parser +
  go/ast + go/token + encoding/json, zero third-party imports, so `go run <file>` works
  with no module context). Delivery: embedded in the Rust bridge via `include_str!`,
  materialised content-addressed under `target/conform/go-extract/` — the proven
  ts-extract mechanism, byte-for-byte in spirit. Emits NDJSON facts: `item` (func / type /
  method / const / var, exported flag, attached `//spec:` directives), `import`,
  `go_unsafe` (the Go ban-set census: `init()` decls, blank imports, ambient-default
  calls, naked `go` statements outside owner constructs, `t.Skip` in tests,
  `fmt.Errorf`-without-`%w` heuristics), `marker` (spec directives), `file_metrics`.
  *Rejected:* tree-sitter-go in-process (loses go-grade positions/directive semantics and
  adds a grammar dependency); parsing Go with regex in Rust (lies on strings/comments);
  gopls as the fact source (a heavyweight child for what a parse answers).
- **D5 — exhaustiveness is a linter-carried rule, named honestly.** Go has no sum types
  and no exhaustive `switch` — the deepest gap, closed by const-enum sets + the
  `exhaustive` linter (BSD-2) as an evidence provider, plus closed error-set structs
  (`Code + Spec + Err`, `errors.As`-consumed). The conform rule
  `error-message-cites-req` and the seam-error grammar are ours; switch exhaustiveness
  enforcement rides the linter. *Rejected:* pretending a `default`-panic arm equals
  compiler exhaustiveness (it is a runtime trap, not a compile gate — documented as the
  honest degradation).
- **D6 — doctests are native: `Example` functions with `// Output:`.** Go's testing
  package compiles AND runs examples, diffing stdout — a stronger guarantee than Rust
  doctests (behavioral, not just compiling). Scaffold G binds to `ExampleXxx` presence
  per exported seam item; the census counts them via go-extract.
- **D7 — the floor is seven steps:** `gofmt -l` → `go vet ./...` → `go test ./...`
  (JSON stream parsed) → `staticcheck ./...` + `exhaustive` (policy-disable-able, with
  the printed `DISABLED by policy` line and a reason, the TS eslint idiom) → conform →
  specmap `--check` → test-gate (when a baseline exists). `govulncheck` is CI-posture,
  not floor (network-touching). One exit code; per-policy origin lines.
- **D8 — test-gate parses `go test -json`** (the native machine format; no TAP shim
  needed). xfail-strict semantics per BROWNFIELD §4; **no in-source xfail twin** —
  `t.Skip` on a known-failing test is banned (hides regressions AND healings), so
  `tests-baseline.json` carries full weight alone: Go is the one stack where the registry
  is the only xfail home (legacy §7, kept and stated in the guide).
- **D9 — naming (PROP-028 family-prefix):** binaries `go-ai-native`,
  `go-ai-native-conform`, `go-ai-native-specmap`, `go-ai-native-tcg`,
  `go-ai-native-mcp`; crates add `-cli`, `-conform-frontend`, `-extract-bridge`,
  `-specmap-scan`, `-tcg-bridge`; the agent-visible MCP server name is `go-ai-native`.
  Frontend id: `"go-extract"`. Packages: `stack:org.vibevm.ai-native/go-ai-native-lang`,
  `mcp:org.vibevm.ai-native/go-ai-native-mcp`, aggregator
  `stack:org.vibevm.ai-native/go-ai-native` — all v0.1.0 (a new version line; the family
  version-mirrors within itself via exact pins, per the aggregator law).
- **D10 — the tcg line ships agentic-only (owner ruling §2.5).** Full-parity brief
  `vibe-agentic-tcg-go.md` + mechanisms TCG-ORACLE-GO / TCG-PROTOCOL-GO + the working
  `go-ai-native-tcg` crate pair. The token-level `go-ai-native-tcg.md` is a STUB at the
  TS stub's depth (asymmetry note, layering, staged ambition, licensing, honest note) and
  is dispositioned very-far-future. Wire parity: NDJSON, `ORACLE_PROTOCOL = 1`, the same
  five ops + init/update/shutdown, the same five-kind error taxonomy with two Go renames
  (`gopls-missing`, `workspace-unloadable` → go.mod / `go env` failures).
- **D11 — the demo domain is a miniature reconciler** (desired vs actual state, diff →
  actions → converge): the Kubernetes-shaped rehearsal the owner named, small enough to
  read in one sitting, rich enough to exercise every scaffold — two planner cells behind
  one seam (differential fuzz oracle between them), a steppable simulator (scaffold H),
  closed error sets, Example-doctests, a flag registry, golden transcripts.
  *Rejected:* CRUD/todo demos (no non-obvious dynamics → scaffolds H/D degenerate).
- **D12 — Phase 0 provisions the toolchain per the owner's layout:** go lives at
  `C:/opt/go` (owner-installed); session PATH gains `C:/opt/go/bin` and
  `C:/opt/gotools`; extra tools install with `GOBIN=C:/opt/gotools` via
  `go install golang.org/x/tools/gopls@latest honnef.co/go/tools/cmd/staticcheck@latest
  github.com/nishanths/exhaustive/cmd/exhaustive@latest` — all three open-source
  (BSD-3 / MIT / BSD-2) and cross-platform, per the owner's constraint. If the module
  proxy is unreachable, the campaign degrades gracefully: live gopls tests and
  staticcheck/exhaustive floor steps go replay-only/policy-disabled WITH the recipe
  printed, and the miss is recorded in §13 — never silently skipped.
- **D13 — test posture mirrors the siblings:** every crate replay-tests its child
  protocol without the real child (recorded NDJSON/LSP transcripts as goldens); fixtures
  (`clean/`, `dirty/`) live inside the package; the live chain (real go, real gopls) runs
  where the toolchain exists and is marked as the environment-dependent tier. The demo
  carries its own go-native test corpus and is ALSO the stack's end-to-end acceptance
  target (`go-ai-native floor` green on it).

## 7. Phases {#phases}

Each phase ends with: its own topic commits (Conventional Commits, why-bodies), the host
`bash tools/self-check.sh` green, the package workspace(s) `cargo build && cargo test`
green (once they exist), and a §13 ledger entry. Any phase boundary is a safe stop.

- **Phase 0 — provisioning & verify (no repo commits).** Install go (winget →
  `GoLang.Go`; fallback zip), gopls, staticcheck, exhaustive; record exact versions in
  §13. Read the vendored engine APIs (`conform-core` Frontend/Fact/rules,
  `specmap-core`), the TS extract-bridge, and the Rust tcg-bridge top-down — the
  just-in-time reading list for Phases 4–7.
- **Phase 1 — package skeletons.** Three package dirs + manifests (vibe.toml per the
  measured forms: stack with boot_snippet/binaries/skills; mcp with exact pin +
  [[mcp_server]]; aggregator with exact pins), LICENSE.md (UPL-1.0), README.md stubs,
  specmap.toml. *Acceptance:* `vibe check`-clean manifests (or documented why not
  runnable); host floor green. *Commit:* `feat(ai-native): go-ai-native family skeletons`.
- **Phase 2 — the spec corpus.** `GUIDE-AI-NATIVE-GO.md` (full, isomorphic §0–§16-class
  structure: law → cells → surface/naming → nine scaffolds → errors → registry → bans →
  specmap → prose → replacement → test matrices → weak reader → tooling pointer → wiring
  → sweep idioms; absorbs the legacy projection), boot snippet, cards INDEX + 9 cards
  (Band 1/2/3, checker statuses `specified` except where Phase 5–7 ships them — those
  rows say `shipped`). *Commits:* guide+boot; cards.
- **Phase 3 — tcg + tools specs, skills.** TCG-ORACLE-GO-v0.1 (gopls process model:
  resolution, LSP session/capabilities incl. the push-vs-pull diagnostics question named
  honestly, overlays, quiescence, fidelity posture D3, lifecycle, latency posture with
  posted-not-gated targets), TCG-PROTOCOL-GO-v0.1 (wire parity + enrichment hop + error
  taxonomy), vibe-agentic-tcg-go.md (7 sections), go-ai-native-tcg.md (stub),
  conform-frontend-go.md, the two SKILL.md files. *Commit:* one spec-corpus commit.
- **Phase 4 — engines & the extractor.** Vendor byte-copies; workspace Cargo.toml;
  `tools/go-extract/extract.go` (stdlib-only; NDJSON per D4) + fixtures;
  `go-ai-native-extract-bridge` (spawn `go run`, content-addressed materialisation,
  protocol client, replay tests). *Acceptance:* bridge tests green without go; live
  extract green with go. *Commits:* engines+workspace; extractor+bridge.
- **Phase 5 — the gates.** `go-ai-native-conform-frontend` (facts via bridge),
  `go-ai-native-conform` (bin: check/freeze over conform-core, `[go]` topology in
  conform.toml: roots/cells_dir/seam), `go-ai-native-specmap`(+`-scan`: directive scanner
  feeding specmap-core; orphan ratchet). Rule set at parity with the TS stack's
  (`go-unsafe-in-domain`, `go-cell-isolation`, file budget, error-cites-req) plus the
  D4 census kinds. Tests: fixtures clean/dirty, determinism (run twice, diff), baseline
  ratchet. *Commits:* conform pair; specmap pair.
- **Phase 6 — the umbrella CLI.** `go-ai-native-cli`: `init` (conform.toml, specmap.toml,
  registries, topology detection from go.mod/packages), `floor` (D7, one exit code,
  policy-origin lines), `health` (collector: danger-band, suppression census =
  unreasoned `//nolint`+`t.Skip` count, example-coverage, orphan backlog, deviation
  debt), `test-gate` (D8), `tripwire`, `trace` (explain over the index), `fast-loop`
  (per-package go test budget), `codemod add-cell` (emits a cell skeleton + registry
  arm + directive tags). *Commits:* 2–3 by verb group.
- **Phase 7 — the agentic tcg.** `go-ai-native-tcg-bridge` (LSP 3.17 client to gopls:
  Content-Length framing, capability negotiation, overlays with monotonic versions,
  quiescence wait with `degraded` flag, five-kind error taxonomy, kill-on-drop +
  shutdown/exit dance, replay-tested from recorded transcripts);
  `go-ai-native-tcg` (bin: `serve` NDJSON relay with in-process enrichment through the
  SAME conform rules; one-shot validate/scope/complete/type; `bench`). Live chain where
  gopls exists. *Commits:* bridge; cli+enrichment.
- **Phase 8 — the MCP server.** `go-ai-native-mcp` package: vendored stack crates +
  mcp-core; the server crate (parity map: init/floor/conform_check/conform_freeze/
  specmap_check/specmap_write/trace_explain/test_gate/tripwire/health/fast_loop/
  codemod_add_cell + tcg_validate/tcg_scope/tcg_complete/tcg_type/tcg_bench — 17 tools,
  the TS count: no ledger verb), capture guard, `language:"go"` refusal grammar for
  other languages; `spec/tools/discipline-mcp-go.md`. *Commit:* one.
- **Phase 9 — the pilot demo + end-to-end.** `research/go-demo` per D11: code, spec with
  anchored reqs, `//spec:` tags, conform/specmap tomls, registries, goldens; the go test
  corpus (units, Examples, fuzz differential naive-vs-batch, quick properties, -race);
  then the acceptance run: `go-ai-native init`-equivalents already in place →
  **`go-ai-native floor` GREEN on the demo**, `health` snapshot committed, a tcg
  one-shot `validate` answering against a demo file (live tier). *Commits:* demo; corpus;
  end-to-end wiring fixes if any.
- **Phase 10 — closeout.** Isomorphism sweep (structure-diff each artifact against its
  Rust/TS sibling), READMEs finalized, §13 ledger completed, §12 REPORT written
  (predictions checked), WAL + CONTINUE.md updated, final push (`git push origin main` —
  routine; mirrors fan-out stays the owner's explicit call per PROP-016).

**Deferred by name (not this campaign):** registry publishing of the three packages
(owner decision, PROP-002); installing the go stack into the vibevm host project
(pointless until host Go code exists); a `go-ai-native-env-audit` twin (the Rust stack's
env-audit has no Go consumer yet); token-level tcg (very-far-future by mandate);
cross-language Stage-B delivery experiments.

## 8. Risks & fallbacks {#risks}

- **R1 — toolchain provisioning fails** (no winget, blocked proxy for `go install`).
  *Detection:* Phase 0 command failures. *Fallback:* official go zip into
  `~\go-toolchain` + session PATH; if the module proxy is unreachable, gopls/staticcheck/
  exhaustive go absent → live tiers replay-only, floor steps policy-disabled with
  recipes, recorded in §13. The campaign still lands: every child-protocol surface is
  replay-tested by design (D13).
- **R2 — gopls diagnostics model differs from rust-analyzer's** (push
  `publishDiagnostics` vs pull; quiescence signalling differs). *Detection:* Phase 7 live
  chain. *Fallback:* the bridge supports both (prefer pull if the capability is granted,
  else collect pushed diagnostics with a settle window); the mechanism spec names
  whichever the live chain proves, and posted latency targets move only with a §13 note.
- **R3 — session/context length.** *Fallback:* every phase is a safe stop; the WAL's
  standing line + this plan's status line + per-phase commits are the resume pointer; a
  fresh session resumes at the recorded phase.
- **R4 — Windows path depth / cargo in deep package trees.* The Rust/TS stacks already
  build in the same depth — precedent says fine; if a MAX_PATH bite appears, build with
  `CARGO_TARGET_DIR` pointed at a shallow scratch dir and record it.
- **R5 — scope creep into the engines.** The vendored engines are consumed AS-IS; if a
  Go need seems to require an engine change, the need is re-designed around the engine or
  filed as debt for a core-ai-native version bump — never patched in-vendor.

## 9. Quick-start (for a cold resume) {#quick-start}

```sh
git -C /c/Users/olegc/gits/vibevm status -sb && git log --oneline -8
bash tools/self-check.sh                             # host floor must be green
go version; gopls version; staticcheck -version      # Phase-0 provisioning state
# package workspaces (once they exist):
cargo build --manifest-path packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/Cargo.toml
cargo test  --manifest-path packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/Cargo.toml
# the pilot (Phase 9+):
cd research/go-demo && go test ./... && go vet ./...
# end-to-end acceptance:
#   <go-ai-native binary> floor   # run from research/go-demo, all seven steps green
```

## 10. Whole-campaign acceptance {#acceptance}

1. The three packages exist with the §5 trees, manifests in the measured forms, UPL-1.0.
2. The spec corpus is complete and isomorphic (guide, 9 cards + INDEX, 2 tcg mechanisms,
   3 tool briefs, 2 skills, boot snippet, mcp brief) — section structures diff cleanly
   against the Rust/TS siblings.
3. `cargo build && cargo test` green in both code-bearing package workspaces (all crates,
   including replay suites, fixtures, determinism tests) on this machine.
4. `research/go-demo` exists, its own `go test ./...` (with -race and fuzz seed corpus)
   green, and **`go-ai-native floor` exits 0 on it** with all seven steps live (or with
   policy-disabled steps ONLY under R1, each printing its recipe).
5. The agentic tcg answers a live one-shot `validate` against a demo file citing at least
   one conform finding class end-to-end (live tier; replay tier green regardless).
6. Host `self-check.sh` green; all work committed in topic commits; pushed to origin.

## 11. Predictions (falsifiable, checked in §12) {#predictions}

- **P1:** the Go stack's own code lands in 6–9k lines of Rust + 0.5–1k lines of Go
  (extractor + demo), tracking the TS stack's measured envelope.
- **P2:** all nine scaffold classes survive projection to Go with no dropped card; the
  weakest projections are B (typestate without phantom generics culture) and the
  exhaustiveness half of the error contract — both carried by named workarounds, not
  omissions.
- **P3:** the Go oracle's fidelity posture lands strictly between TS and Rust, and the
  live chain surfaces at least one gopls-vs-floor behavioral asymmetry worth a
  documented-gap corpus case.
- **P4:** the vendored engines absorb Go unchanged (zero in-vendor edits — R5 never
  fires).
- **P5:** no artifact KIND beyond the isomorphic set is needed; if one appears, that is a
  REPORT finding about the form, not a silent extra file.

## 12. REPORT {#report}

Written at close, 2026-07-17. Results vs §11:

- **P1 — PARTIALLY CONFIRMED.** The stack's own Rust landed at **6,113 lines** (inside
  the 6–9k envelope). Go landed at **674 (extractor) + 1,526 (demo) = 2,200** — above
  the 0.5–1k prediction, because the pilot grew a full test corpus (matrix, property,
  fuzz oracle, simulator conformance, executed Examples) rather than a minimal tree.
  The overshoot is deliberate richness, not scope creep.
- **P2 — CONFIRMED.** All nine scaffold classes survived projection; the predicted weak
  spots (B without typestate culture, linter-carried exhaustiveness) landed exactly as
  named workarounds (defined types + constructors; the `exhaustive` evidence provider).
- **P3 — PARTIALLY CONFIRMED.** The fidelity posture landed as designed (gopls =
  go/types, between TS and Rust) and the live chain proved the mechanism (a seeded type
  error through a pure overlay; push-fallback + `$/progress` readiness both exercised in
  replay). A **live gopls-vs-floor asymmetry corpus case was NOT yet captured** — the
  bench corpus ships empty; seeding it is the named deferral.
- **P4 — FALSIFIED, instructively.** The engines did NOT absorb Go unchanged: the
  neutral core needed `Fact::GoUnsafe`, `GoConfig`, the Go walk, and `rules/go.rs` —
  shipped as the authored **core-ai-native 0.8.0** (R5 held: zero in-vendor edits; the
  fix surface was a version bump, exactly the escape R5 names). Lesson for the
  Discipline: a third language IS a core minor version, plan it as one.
- **P5 — CONFIRMED.** No artifact kind beyond the isomorphic set was needed.

Misfires and findings worth carrying: Go's `flag` package stops at the first non-flag
argument (the `--files` boolean-marker fix); a literal `.` scan root is eaten by
hidden-dir filters without a depth-0 guard; mdspec mints doc-ids from the document
HEADING, not the filename (the demo's dangling-edge lesson, now in its README); a
materialised Go tool inside a consumer module needs a go.mod cut-off or `./...`
compiles it; `-race` needs cgo on Windows (scoped out by the guide's goroutine rule for
the demo). The deliberate sibling-import in the demo's differential oracle became the
first REAL frozen ratchet entry — the replacement-window debt pattern works end to end.

## 13. Execution ledger {#ledger}

- **2026-07-17 — plan authored.** Phase −1 facts gathered (crate volumes, manifest
  forms, toolchain absence). Status: Phase 0 next.
- **2026-07-17 — Phases 0–3.** Toolchain provisioned (go 1.26.5 at C:/opt/go; gopls
  0.23 + staticcheck 0.8-rc + exhaustive→master-with-bumped-x/tools into C:/opt/gotools
  — the pinned v0.12.0 does not compile under go 1.26). Skeletons, GUIDE, boot, nine
  cards, tcg mechanisms/briefs/stub, skills: commits 05976fa…d5c6865.
- **2026-07-17 — Phases 4–6.** core-ai-native **0.8.0** (Go in the neutral engine:
  bfb72da); go-extract + bridge + conform gate (1f3d56a; fixtures pin the ten-finding
  census); specmap scanner + orphan ratchet with package-grain scope (27e10af); the
  ten-verb umbrella CLI with the `go test -json` gate and the Example-coverage join
  (969c3fb + 8a26e25). Every phase: full workspace tests green.
- **2026-07-17 — Phase 7.** The agentic tcg (0d28898): LSP bridge to the consumer's
  gopls (env→PATH→GOBIN→GOPATH/bin), `$/progress` readiness, dual diagnostics channels,
  the `--stdin-file` overlay extraction, FILLED markers, brand detection via the new
  `underlying` item field. **Live chain green on real gopls at first attempt** (seeded
  error through a pure overlay; hover; no-zombie shutdown). Finding-parity pins relay =
  gate.
- **2026-07-17 — Phase 8.** go-ai-native-mcp (044b028): 17 tools, capture guard,
  language guard, server replay — green on first build.
- **2026-07-17 — Phase 9.** research/go-demo (509bb82): the reconciler pilot;
  **`go-ai-native floor` ALL GREEN (7 steps, 0 disabled)**; the live tcg one-shot
  contract proven (dirty overlay exit 1 with Class-F advice, clean exit 0 — real exit
  codes checked, never a piped tail's); health snapshot committed; ONE deliberate
  finding frozen (the oracle's sibling import — replacement debt).
- **2026-07-17 — Phase 10.** README finalized; D14 self-trace green
  (`rust-ai-native-specmap --gate` over the stack: 0 orphans, 3 CLI drivers exempt);
  REPORT written; campaign CLOSED.
