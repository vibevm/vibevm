# TCG-ORACLE-GO v0.1 — the gopls oracle process model {#root}

<status stage="spec" state="done"/>

@fact:status-line **Status: v0.1 — authored with GO-AI-NATIVE-PLAN v0.1 (Phase 3),
implemented by its Phase 7.** @status:impl/done

@fact:companion-documents The component brief is
[`tools/vibe-agentic-tcg-go.md`](../tools/vibe-agentic-tcg-go.md); the
message grammar is
[`TCG-PROTOCOL-GO-v0.1`](TCG-PROTOCOL-GO-v0.1.md). @status:impl/done

@fact:DOCUMENT-OWNS-THE-ORACLE-PROCESS This document owns
the oracle PROCESS: resolution, LSP lifecycle, configuration, overlays,
quiescence, the fidelity posture, and latency. @status:impl/done

@fact:QUANTITIES-ARE-CAMPAIGN-MEASURED Where the sibling Rust
mechanism cites measured spike facts, this one names the same
quantities as campaign-measured — *the bench harness
(`go-ai-native-tcg bench`) is the instrument; no Go corpus or baseline
has yet been taken, so the figures below are posted targets rather
than measured ones. Creating the Go test codebase to measure against
is deliberately far-future work (the corpus could be LLM-generated or
fuzzer-generated); it is not being built now* — a target moves only
with a committed REPORT reason. @status:spec/done

## 1. The process and its resolution {#resolution}

@fact:kind-line-resolution `req r1` @status:impl/done

@fact:ORACLE-IS-THE-CONSUMERS-OWN-GOPLS The oracle process is the CONSUMER's own `gopls` binary — the stack
never bundles, links, or vendors an analyzer. @status:impl/done

@fact:resolution-order-lead Resolution order, run
from the project root so `go.work`/module context is honoured, each
failure recipe-carrying and never silently skipped: @status:impl/done

1. @fact:RESOLUTION-GOPLS-ON-PATH the `GO_AI_NATIVE_GOPLS` env override when set (a set-but-not-a-file value refuses with the recipe, without probing further), then `gopls` on PATH; @status:impl/done
2. @fact:RESOLUTION-GOBIN `$GOBIN/gopls`, then `$(go env GOBIN)/gopls`; @status:impl/done
3. @fact:RESOLUTION-GOPATH-BIN `$(go env GOPATH)/bin/gopls`; @status:impl/done
4. @fact:RESOLUTION-HARD-FAILURE hard failure: the bridge's `gopls-missing` error with the recipe
   `go install golang.org/x/tools/gopls@latest`. @status:impl/done

@fact:STACK-OBLIGES-THE-MACHINE Installing this stack OBLIGES the machine to carry go ≥ 1.24 and gopls
(the same posture as rust-analyzer for the Rust stack and node ≥ 22.6
for the TS one): inside the stack's own test suite an absent tool is a
recipe-carrying FAILURE, never a skip; outside the stack no obligation
exists. @status:impl/done

@fact:INIT-RESULT-CARRIES-PATH-AND-VERSION The resolved path and the server's reported version land in
the `init` result. *Specified, not built — half of it. The version half
ships: `init_result` emits `gopls_version`
(`crates/go-ai-native-tcg/src/serve.rs:74-84`), beside `position_encoding`,
`pull_diagnostics` and `ready`. The path half does not: the path IS resolved —
`resolve_gopls` (`crates/go-ai-native-tcg-bridge/src/lib.rs:145`) returns it —
but it is never put into the result. The Rust stack states the same fact and
fails it the same way.* @status:spec/done

## 2. LSP session and capabilities {#session}

@fact:kind-line-session `req r1` @status:impl/done

@fact:BRIDGE-SPEAKS-LSP-3-17-OVER-STDIO The bridge speaks LSP 3.17 over the child's stdio (Content-Length
framing). @status:impl/done

@fact:initialize-declares-lead The `initialize` request declares: @status:impl/done

- @fact:INITIALIZE-DECLARES-UTF-8 utf-8 in
  `general.positionEncodings` (fallback: utf-16 positions converted
  through the line's text, unit-tested on non-ASCII content), @status:impl/done
- @fact:INITIALIZE-DECLARES-PULL-DIAGNOSTICS pull diagnostics (`textDocument.diagnostic`), @status:impl/done
- @fact:INITIALIZE-DECLARES-PUBLISH-DIAGNOSTICS publish-diagnostics handling, @status:impl/done
- @fact:INITIALIZE-DECLARES-WORK-DONE-PROGRESS and `window.workDoneProgress`. @status:impl/done

@fact:FEATURES-KEY-OFF-THE-GRANTED-SET Every downstream feature keys off the
GRANTED set — a capability the server does not grant degrades per §6
into a well-formed error or a documented fallback, never a crash. @status:impl/done

@fact:BRIDGE-ANSWERS-THE-SERVERS-REQUESTS The bridge answers the server's own requests:
`workspace/configuration` (with §3's config object),
`window/workDoneProgress/create` and `client/registerCapability` (null
results). @status:impl/done

@fact:DIAGNOSTICS-CHANNEL-HISTORY **Diagnostics channel, stated honestly.** gopls has historically
PUSHED diagnostics (`textDocument/publishDiagnostics`) and gained pull
support later than rust-analyzer; which channel the shipped gopls
grants is pinned by the Phase-7 live chain and recorded in the
differential corpus. *Specified, not built — the history is true, the
recording is not. The channel IS negotiated at run time and carried on the
capabilities (`pull_diagnostics`,
`crates/go-ai-native-tcg-bridge/src/client.rs:33`), but nothing records
which one the shipped gopls granted: `research/tcg-bench/` holds a
TypeScript corpus (`corpus/`) and a Rust one (`corpus-rust/`) and no Go
corpus at all, so there is no differential corpus for this to be recorded
in.* @status:spec/done

@fact:BRIDGE-SUPPORTS-BOTH-DIAGNOSTIC-CHANNELS The bridge supports BOTH: prefer the pull channel
when granted; otherwise collect pushed diagnostics for the target
document with a bounded settle window after `didOpen`/`didChange`. @status:impl/done

@fact:VALIDATE-ANSWERS-ONE-DOCUMENT Either way `validate` answers one document's diagnostics — never a
whole-workspace sweep. @status:impl/done

## 3. Configuration {#config}

@fact:kind-line-config `req r1` @status:impl/done

@fact:BRIDGE-SHIPS-ONE-CONFIGURATION-OBJECT The bridge ships one configuration object, passed as
`initializationOptions` and repeated in every
`workspace/configuration` answer. @status:impl/done

@fact:config-is-minimal-and-documented-lead v0.1 keeps it minimal and DOCUMENTED
— gopls's defaults are production-grade (its diagnostics are not
gated behind experimental flags the way rust-analyzer's E0308-class
ones are; the Rust bridge's config lesson transfers as a posture, not
as content): @status:impl/done

- @fact:CONFIG-STATICCHECK-STAYS-OFF staticcheck integration stays OFF (the floor runs
  staticcheck itself; one tool, one truth), @status:impl/done
- @fact:CONFIG-ANALYSES-STAY-AT-DEFAULTS analyses stay at gopls
  defaults, @status:impl/done
- @fact:CONFIG-FUTURE-KNOBS-EXTEND-ONE-OBJECT and any future knob (build tags, env) extends this one
  object in one place. @status:impl/done

## 4. Overlays and versions {#overlays}

@fact:kind-line-overlays `req r1` @status:impl/done

@fact:overlay-is-an-lsp-owned-document-lead An overlay is an LSP-owned text document: @status:impl/done

- @fact:OVERLAY-DIDOPEN-CLAIMS-THE-DOCUMENT `didOpen {uri, version: 1,
  text}` claims the document (the server stops reading disk for it), @status:impl/done
- @fact:OVERLAY-DIDCHANGE-REPLACES-IT `didChange` with full-text sync and a MONOTONICALLY increasing
  per-document version replaces it, @status:impl/done
- @fact:OVERLAY-DIDCLOSE-RELEASES-IT `didClose` releases it back to disk. @status:impl/done

@fact:proven-rules-are-law-here-lead The rules the TS and Rust campaigns proved are law here and the bridge
enforces them structurally: @status:impl/done

- @fact:OVERLAY-VERSIONS-NEVER-REPEAT-OR-RESET versions never repeat within an
  overlay's lifetime (a monotonic counter per open document, never derived
  from content); clearing an overlay (`update {content: null}`) closes the
  document and a later reopen starts again at 1 —
  `crates/go-ai-native-tcg-bridge/src/oracle.rs`, and the bridge's own
  `overlay_versions_are_monotonic_and_close_resets` test; @status:impl/done
- @fact:VALIDATE-WITHOUT-CONTENT-READS-DISK `validate` WITHOUT inline content reads the disk file and opens it
  with that text, so version bookkeeping has exactly one owner (the
  bridge) and a later disk edit is picked up by the next validate's
  `didChange`; @status:impl/done
- @fact:OVERLAID-FILE-NEED-NOT-EXIST-ON-DISK an overlaid file need not exist on disk — a hypothetical new file in
  an existing package participates via `didOpen` alone; @status:impl/done
- @fact:UPDATE-NULL-MAPS-TO-DIDCLOSE `update {content: null}` maps to `didClose`. @status:impl/done

## 5. The fidelity posture (gopls is go/types, not the compiler) {#fidelity}

@fact:kind-line-fidelity `req r1` @status:impl/done

@fact:fidelity-spectrum-lead The three stacks now span a fidelity spectrum, and this oracle's place
on it is spec, not fine print: @status:impl/done

- @fact:FIDELITY-TS-ORACLE-IS-THE-COMPILER The TS oracle IS the compiler (the LanguageService is tsc's engine —
  agreement by construction). @status:spec/done
- @fact:FIDELITY-RUST-ANALYZER-IS-NOT-RUSTC rust-analyzer is NOT rustc (an independent, deliberately partial
  analysis). @status:spec/done
- @fact:FIDELITY-GOPLS-STANDS-ON-GO-TYPES **gopls stands on `go/types` — the reference library implementation
  of the Go specification, the same framework `go vet` builds on —
  while the gc compiler type-checks with `types2`, go/types'
  deliberately-synchronized port.** The delta is a maintained-identical
  pair whose divergences are treated as bugs upstream: far tighter
  than rust-analyzer↔rustc, still not identity. @status:spec/done

@fact:consequences-lead Consequences, all normative: @status:impl/done

- @fact:CLEAN-VALIDATE-DOES-NOT-CERTIFY-A-CLEAN-FLOOR A clean `validate` does NOT certify a clean floor. The floor
  (`go-ai-native floor` → gofmt/vet/build/test) remains the truth;
  consumer-facing docs repeat it. @status:impl/done
- @fact:DIFFERENTIAL-CORPUS-PINS-DIAGNOSTIC-CLASSES The differential corpus curates diagnostic classes and pins each to
  the floor's own verdict (`go build` / `go vet` exit + message class)
  through a committed mapping table: type mismatch, undeclared name,
  wrong argument count, unknown field, missing return, unused
  import/variable. *Specified, not built: there is no Go corpus and no
  mapping table. The corpora that exist are
  `research/tcg-bench/corpus/` (TypeScript, 7 cases) and
  `research/tcg-bench/corpus-rust/` (Rust, 9 cases); no `corpus-go`
  exists anywhere in the tree. The six classes named above are a
  curation nobody has performed yet.* @status:spec/done
- @fact:KNOWN-ASYMMETRIES-ARE-DOCUMENTED-GAP-CASES Known asymmetries are DOCUMENTED-GAP corpus cases, not omissions —
  the standing candidates to probe in Phase 7: diagnostics gated on
  saved-vs-overlay state, `go.mod`-dependent resolution under a pure
  overlay, and vet-only findings (printf shapes) that the floor
  reports and the oracle may not. Each observed asymmetry becomes a
  corpus case asserting exactly that shape, so the gap list never
  rots. @status:impl/done

## 6. Quiescence, degradation, never crashes {#degradation}

@fact:kind-line-degradation `req r1` @status:impl/done

@fact:workspace-load-after-initialized After `initialized`, the server loads the workspace (go.mod parsing,
package metadata, cache priming). @status:spec/done

@fact:READINESS-WAIT-IS-DEADLINE-BOUNDED The bridge bounds its readiness wait
by a deadline keyed on `workDoneProgress` end events for the initial
load; a deadline pass degrades: answers carry `degraded: true`, so
callers can distinguish warm truth from cold best-effort. @status:impl/done

@fact:PROGRESS-DRAIN-HEURISTIC-IS-INHERITED-AS-A-WARNING The Rust
campaign's falsified progress-drain heuristic is inherited as a
WARNING, not a mechanism: no wait strategy is trusted until the
Phase-7 live chain pins gopls's actual signalling, and a replay test
pins whatever is chosen. @status:impl/done

@fact:b5-extends-to-the-whole-session-lead B5 extends to the whole session: @status:impl/done

- @fact:UNKNOWN-OP-ANSWERS-A-PROTOCOL-ERROR an op the relay does not know answers a protocol error naming the
  known set; @status:impl/done
- @fact:ANALYZER-CRASH-ENDS-THE-SESSION an analyzer crash surfaces `oracle-crashed` op-grain and ends the
  session (the product registry owns respawn-once); @status:impl/done
- @fact:NO-INPUT-MAY-POISON-THE-SESSION no input may poison the session. @status:impl/done

## 7. Process lifecycle and Windows discipline {#lifecycle}

@fact:kind-line-lifecycle `req r1` @status:impl/done

@fact:ONE-LONG-LIVED-CHILD-PER-ROOT-SESSION One long-lived child per (root, session). @status:impl/done

@fact:GRACEFUL-EXIT-IS-THE-LSP-DANCE Graceful exit is the LSP
dance — `shutdown` request, `exit` notification — with kill-on-drop as
the backstop; the no-zombie property is test-asserted. @status:impl/done

@fact:PATHS-BECOME-URIS-AFTER-VERBATIM-PREFIX-STRIPPING Paths become
URIs only after verbatim-prefix stripping (`\\?\` breaks child argv
and URI builders — the standing house lesson). @status:impl/done

@fact:STDOUT-CARRIES-LSP-FRAMES-ONLY stdout carries LSP
frames only; gopls's own stderr chatter is drained and discarded by
the reader (surfaced only in bridge debug logging), so protocol
streams stay clean. @status:impl/done

## 8. Latency posture {#latency}

@fact:kind-line-latency `req r1` @status:impl/done

@fact:TARGETS-ARE-POSTED-AND-MEASURED-NEVER-GATED Targets are POSTED and MEASURED, never CI-gated (the standing split:
gate what cannot flake, record what can). @status:impl/done

@fact:posted-targets-lead Posted targets for
demo-class trees: @status:impl/done

- @fact:TARGET-WARM-VALIDATE warm `validate` p50 < 500 ms, @status:impl/done
- @fact:TARGET-COMPLETE `complete` p50 < 300 ms — posted, not yet measured:
  the bench harness (`crates/go-ai-native-tcg/src/bench.rs`) records
  per-case `warm_ms` for `validate` only and computes no percentile of
  any kind; the measurement corpus is deliberately far-future work
  (`BACKLOG.md` B-042), @status:spec/done
- @fact:TARGET-COLD-INIT cold init-to-ready < 15 s. @status:impl/done

@fact:BENCH-HARNESS-RECORDS-DISTRIBUTIONS The bench harness
(`go-ai-native-tcg bench`) records distributions per run; the Phase-7
ledger entry carries the first measured set on `research/go-demo`,
and a target that moves, moves in a committed REPORT with a reason.
*Specified, not built — the harness, not the ledger entry. The harness
ships and does record a per-case distribution: `run_bench`
(`crates/go-ai-native-tcg/src/bench.rs:106`) collects `warm_ms` over three
warm passes per case. What does not exist is the measured set:
`research/tcg-bench/reports/` holds `bench-2026-07-07-baseline.json`
(TypeScript) and `bench-rust-2026-07-07-baseline.json` (Rust) and no Go
run at all, so nothing has ever been measured on `research/go-demo` and
there is no committed REPORT for a target to move against.* @status:spec/done

@fact:LARGE-WORKSPACE-COLD-INIT-WARNING Large-workspace consumers are warned about the product's 60 s
first-request ceiling; the relay's eager init at `serve` start spends
the cold cost as early as possible. *Specified, not built — the eager
init is real, the 60 s ceiling is not a number this stack has anywhere.
The relay does spend the cold cost up front: `serve` boots gopls before
the first host frame (`crates/go-ai-native-tcg/src/serve.rs:235`). A
large-workspace warning also exists, one document over — the tcg brief's
`##RISK-COLD-INIT-ON-LARGE-WORKSPACES` — but it is a spec-layer risk
record, not anything the product emits to a consumer at run time. And the
60 s figure is supported by nothing: the shipped readiness budget is
`READINESS_BUDGET = 45 s` (`crates/go-ai-native-tcg/src/lib.rs:32`) and
this document's own `##TARGET-COLD-INIT` posts `< 15 s`. Three numbers,
and 60 is not one of them.* @status:spec/done
