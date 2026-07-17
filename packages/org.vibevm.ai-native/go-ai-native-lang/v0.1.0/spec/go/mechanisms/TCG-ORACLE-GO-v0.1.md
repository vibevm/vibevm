# TCG-ORACLE-GO v0.1 — the gopls oracle process model {#root}

**Status: v0.1 — authored with GO-AI-NATIVE-PLAN v0.1 (Phase 3),
implemented by its Phase 7.** The component brief is
[`tools/vibe-agentic-tcg-go.md`](../tools/vibe-agentic-tcg-go.md); the
message grammar is
[`TCG-PROTOCOL-GO-v0.1`](TCG-PROTOCOL-GO-v0.1.md). This document owns
the oracle PROCESS: resolution, LSP lifecycle, configuration, overlays,
quiescence, the fidelity posture, and latency. Where the sibling Rust
mechanism cites measured spike facts, this one names the same
quantities as campaign-measured: the Phase-7 live chain and the bench
harness record them; a target moves only with a committed REPORT
reason.

## 1. The process and its resolution {#resolution}

`req r1`

The oracle process is the CONSUMER's own `gopls` binary — the stack
never bundles, links, or vendors an analyzer. Resolution order, run
from the project root so `go.work`/module context is honoured, each
failure recipe-carrying and never silently skipped:

1. `gopls` on PATH;
2. `$GOBIN/gopls`, then `$(go env GOBIN)/gopls`;
3. `$(go env GOPATH)/bin/gopls`;
4. hard failure: the bridge's `gopls-missing` error with the recipe
   `go install golang.org/x/tools/gopls@latest`.

Installing this stack OBLIGES the machine to carry go ≥ 1.24 and gopls
(the same posture as rust-analyzer for the Rust stack and node ≥ 22.6
for the TS one): inside the stack's own test suite an absent tool is a
recipe-carrying FAILURE, never a skip; outside the stack no obligation
exists. The resolved path and the server's reported version land in
the `init` result.

## 2. LSP session and capabilities {#session}

`req r1`

The bridge speaks LSP 3.17 over the child's stdio (Content-Length
framing). The `initialize` request declares: utf-8 in
`general.positionEncodings` (fallback: utf-16 positions converted
through the line's text, unit-tested on non-ASCII content), pull
diagnostics (`textDocument.diagnostic`), publish-diagnostics handling,
and `window.workDoneProgress`. Every downstream feature keys off the
GRANTED set — a capability the server does not grant degrades per §6
into a well-formed error or a documented fallback, never a crash. The
bridge answers the server's own requests: `workspace/configuration`
(with §3's config object), `window/workDoneProgress/create` and
`client/registerCapability` (null results).

**Diagnostics channel, stated honestly.** gopls has historically
PUSHED diagnostics (`textDocument/publishDiagnostics`) and gained pull
support later than rust-analyzer; which channel the shipped gopls
grants is pinned by the Phase-7 live chain and recorded in the
differential corpus. The bridge supports BOTH: prefer the pull channel
when granted; otherwise collect pushed diagnostics for the target
document with a bounded settle window after `didOpen`/`didChange`.
Either way `validate` answers one document's diagnostics — never a
whole-workspace sweep.

## 3. Configuration {#config}

`req r1`

The bridge ships one configuration object, passed as
`initializationOptions` and repeated in every
`workspace/configuration` answer. v0.1 keeps it minimal and DOCUMENTED
— gopls's defaults are production-grade (its diagnostics are not
gated behind experimental flags the way rust-analyzer's E0308-class
ones are; the Rust bridge's config lesson transfers as a posture, not
as content): staticcheck integration stays OFF (the floor runs
staticcheck itself; one tool, one truth), analyses stay at gopls
defaults, and any future knob (build tags, env) extends this one
object in one place.

## 4. Overlays and versions {#overlays}

`req r1`

An overlay is an LSP-owned text document: `didOpen {uri, version: 1,
text}` claims the document (the server stops reading disk for it),
`didChange` with full-text sync and a MONOTONICALLY increasing
per-document version replaces it, `didClose` releases it back to disk.
The rules the TS and Rust campaigns proved are law here and the bridge
enforces them structurally:

- versions never repeat or reset within a session (a monotonic counter
  per document, never derived from content);
- `validate` WITHOUT inline content reads the disk file and opens it
  with that text, so version bookkeeping has exactly one owner (the
  bridge) and a later disk edit is picked up by the next validate's
  `didChange`;
- an overlaid file need not exist on disk — a hypothetical new file in
  an existing package participates via `didOpen` alone;
- `update {content: null}` maps to `didClose`.

## 5. The fidelity posture (gopls is go/types, not the compiler) {#fidelity}

`req r1`

The three stacks now span a fidelity spectrum, and this oracle's place
on it is spec, not fine print:

- The TS oracle IS the compiler (the LanguageService is tsc's engine —
  agreement by construction).
- rust-analyzer is NOT rustc (an independent, deliberately partial
  analysis).
- **gopls stands on `go/types` — the reference library implementation
  of the Go specification, the same framework `go vet` builds on —
  while the gc compiler type-checks with `types2`, go/types'
  deliberately-synchronized port.** The delta is a maintained-identical
  pair whose divergences are treated as bugs upstream: far tighter
  than rust-analyzer↔rustc, still not identity.

Consequences, all normative:

- A clean `validate` does NOT certify a clean floor. The floor
  (`go-ai-native floor` → gofmt/vet/build/test) remains the truth;
  consumer-facing docs repeat it.
- The differential corpus curates diagnostic classes and pins each to
  the floor's own verdict (`go build` / `go vet` exit + message class)
  through a committed mapping table: type mismatch, undeclared name,
  wrong argument count, unknown field, missing return, unused
  import/variable.
- Known asymmetries are DOCUMENTED-GAP corpus cases, not omissions —
  the standing candidates to probe in Phase 7: diagnostics gated on
  saved-vs-overlay state, `go.mod`-dependent resolution under a pure
  overlay, and vet-only findings (printf shapes) that the floor
  reports and the oracle may not. Each observed asymmetry becomes a
  corpus case asserting exactly that shape, so the gap list never
  rots.

## 6. Quiescence, degradation, never crashes {#degradation}

`req r1`

After `initialized`, the server loads the workspace (go.mod parsing,
package metadata, cache priming). The bridge bounds its readiness wait
by a deadline keyed on `workDoneProgress` end events for the initial
load; a deadline pass degrades: answers carry `degraded: true`, so
callers can distinguish warm truth from cold best-effort. The Rust
campaign's falsified progress-drain heuristic is inherited as a
WARNING, not a mechanism: no wait strategy is trusted until the
Phase-7 live chain pins gopls's actual signalling, and a replay test
pins whatever is chosen. B5 extends to the whole session: an op the
relay does not know answers a protocol error naming the known set; an
analyzer crash surfaces `oracle-crashed` op-grain and ends the session
(the product registry owns respawn-once); no input may poison the
session.

## 7. Process lifecycle and Windows discipline {#lifecycle}

`req r1`

One long-lived child per (root, session). Graceful exit is the LSP
dance — `shutdown` request, `exit` notification — with kill-on-drop as
the backstop; the no-zombie property is test-asserted. Paths become
URIs only after verbatim-prefix stripping (`\\?\` breaks child argv
and URI builders — the standing house lesson). stdout carries LSP
frames only; gopls's own stderr chatter is drained and discarded by
the reader (surfaced only in bridge debug logging), so protocol
streams stay clean.

## 8. Latency posture {#latency}

`req r1`

Targets are POSTED and MEASURED, never CI-gated (the standing split:
gate what cannot flake, record what can). Posted targets for
demo-class trees: warm `validate` p50 < 500 ms, `complete` p50 <
300 ms, cold init-to-ready < 15 s. The bench harness
(`go-ai-native-tcg bench`) records distributions per run; the Phase-7
ledger entry carries the first measured set on `research/go-demo`,
and a target that moves, moves in a committed REPORT with a reason.
Large-workspace consumers are warned about the product's 60 s
first-request ceiling; the relay's eager init at `serve` start spends
the cold cost as early as possible.
