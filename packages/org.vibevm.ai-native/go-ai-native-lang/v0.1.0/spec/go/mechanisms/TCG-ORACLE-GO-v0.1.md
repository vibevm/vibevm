# TCG-ORACLE-GO v0.1 — the gopls oracle process model {#root}

<status stage="spec" state="done"/>

##status-line **Status: v0.1 — authored with GO-AI-NATIVE-PLAN v0.1 (Phase 3),
implemented by its Phase 7.** @impl/done

##companion-documents The component brief is
[`tools/vibe-agentic-tcg-go.md`](../tools/vibe-agentic-tcg-go.md); the
message grammar is
[`TCG-PROTOCOL-GO-v0.1`](TCG-PROTOCOL-GO-v0.1.md). @impl/done

##DOCUMENT-OWNS-THE-ORACLE-PROCESS This document owns
the oracle PROCESS: resolution, LSP lifecycle, configuration, overlays,
quiescence, the fidelity posture, and latency. @impl/done

##QUANTITIES-ARE-CAMPAIGN-MEASURED Where the sibling Rust
mechanism cites measured spike facts, this one names the same
quantities as campaign-measured: the Phase-7 live chain and the bench
harness record them; a target moves only with a committed REPORT
reason. @impl/done

## 1. The process and its resolution {#resolution}

##kind-line-resolution `req r1` @impl/done

##ORACLE-IS-THE-CONSUMERS-OWN-GOPLS The oracle process is the CONSUMER's own `gopls` binary — the stack
never bundles, links, or vendors an analyzer. @impl/done

##resolution-order-lead Resolution order, run
from the project root so `go.work`/module context is honoured, each
failure recipe-carrying and never silently skipped: @impl/done

1. ##RESOLUTION-GOPLS-ON-PATH `gopls` on PATH; @impl/done
2. ##RESOLUTION-GOBIN `$GOBIN/gopls`, then `$(go env GOBIN)/gopls`; @impl/done
3. ##RESOLUTION-GOPATH-BIN `$(go env GOPATH)/bin/gopls`; @impl/done
4. ##RESOLUTION-HARD-FAILURE hard failure: the bridge's `gopls-missing` error with the recipe
   `go install golang.org/x/tools/gopls@latest`. @impl/done

##STACK-OBLIGES-THE-MACHINE Installing this stack OBLIGES the machine to carry go ≥ 1.24 and gopls
(the same posture as rust-analyzer for the Rust stack and node ≥ 22.6
for the TS one): inside the stack's own test suite an absent tool is a
recipe-carrying FAILURE, never a skip; outside the stack no obligation
exists. @impl/done

##INIT-RESULT-CARRIES-PATH-AND-VERSION The resolved path and the server's reported version land in
the `init` result. @impl/done

## 2. LSP session and capabilities {#session}

##kind-line-session `req r1` @impl/done

##BRIDGE-SPEAKS-LSP-3-17-OVER-STDIO The bridge speaks LSP 3.17 over the child's stdio (Content-Length
framing). @impl/done

##initialize-declares-lead The `initialize` request declares: @impl/done

- ##INITIALIZE-DECLARES-UTF-8 utf-8 in
  `general.positionEncodings` (fallback: utf-16 positions converted
  through the line's text, unit-tested on non-ASCII content), @impl/done
- ##INITIALIZE-DECLARES-PULL-DIAGNOSTICS pull diagnostics (`textDocument.diagnostic`), @impl/done
- ##INITIALIZE-DECLARES-PUBLISH-DIAGNOSTICS publish-diagnostics handling, @impl/done
- ##INITIALIZE-DECLARES-WORK-DONE-PROGRESS and `window.workDoneProgress`. @impl/done

##FEATURES-KEY-OFF-THE-GRANTED-SET Every downstream feature keys off the
GRANTED set — a capability the server does not grant degrades per §6
into a well-formed error or a documented fallback, never a crash. @impl/done

##BRIDGE-ANSWERS-THE-SERVERS-REQUESTS The bridge answers the server's own requests:
`workspace/configuration` (with §3's config object),
`window/workDoneProgress/create` and `client/registerCapability` (null
results). @impl/done

##DIAGNOSTICS-CHANNEL-HISTORY **Diagnostics channel, stated honestly.** gopls has historically
PUSHED diagnostics (`textDocument/publishDiagnostics`) and gained pull
support later than rust-analyzer; which channel the shipped gopls
grants is pinned by the Phase-7 live chain and recorded in the
differential corpus. @spec/done

##BRIDGE-SUPPORTS-BOTH-DIAGNOSTIC-CHANNELS The bridge supports BOTH: prefer the pull channel
when granted; otherwise collect pushed diagnostics for the target
document with a bounded settle window after `didOpen`/`didChange`. @impl/done

##VALIDATE-ANSWERS-ONE-DOCUMENT Either way `validate` answers one document's diagnostics — never a
whole-workspace sweep. @impl/done

## 3. Configuration {#config}

##kind-line-config `req r1` @impl/done

##BRIDGE-SHIPS-ONE-CONFIGURATION-OBJECT The bridge ships one configuration object, passed as
`initializationOptions` and repeated in every
`workspace/configuration` answer. @impl/done

##config-is-minimal-and-documented-lead v0.1 keeps it minimal and DOCUMENTED
— gopls's defaults are production-grade (its diagnostics are not
gated behind experimental flags the way rust-analyzer's E0308-class
ones are; the Rust bridge's config lesson transfers as a posture, not
as content): @impl/done

- ##CONFIG-STATICCHECK-STAYS-OFF staticcheck integration stays OFF (the floor runs
  staticcheck itself; one tool, one truth), @impl/done
- ##CONFIG-ANALYSES-STAY-AT-DEFAULTS analyses stay at gopls
  defaults, @impl/done
- ##CONFIG-FUTURE-KNOBS-EXTEND-ONE-OBJECT and any future knob (build tags, env) extends this one
  object in one place. @impl/done

## 4. Overlays and versions {#overlays}

##kind-line-overlays `req r1` @impl/done

##overlay-is-an-lsp-owned-document-lead An overlay is an LSP-owned text document: @impl/done

- ##OVERLAY-DIDOPEN-CLAIMS-THE-DOCUMENT `didOpen {uri, version: 1,
  text}` claims the document (the server stops reading disk for it), @impl/done
- ##OVERLAY-DIDCHANGE-REPLACES-IT `didChange` with full-text sync and a MONOTONICALLY increasing
  per-document version replaces it, @impl/done
- ##OVERLAY-DIDCLOSE-RELEASES-IT `didClose` releases it back to disk. @impl/done

##proven-rules-are-law-here-lead The rules the TS and Rust campaigns proved are law here and the bridge
enforces them structurally: @impl/done

- ##OVERLAY-VERSIONS-NEVER-REPEAT-OR-RESET versions never repeat or reset within a session (a monotonic counter
  per document, never derived from content); @impl/done
- ##VALIDATE-WITHOUT-CONTENT-READS-DISK `validate` WITHOUT inline content reads the disk file and opens it
  with that text, so version bookkeeping has exactly one owner (the
  bridge) and a later disk edit is picked up by the next validate's
  `didChange`; @impl/done
- ##OVERLAID-FILE-NEED-NOT-EXIST-ON-DISK an overlaid file need not exist on disk — a hypothetical new file in
  an existing package participates via `didOpen` alone; @impl/done
- ##UPDATE-NULL-MAPS-TO-DIDCLOSE `update {content: null}` maps to `didClose`. @impl/done

## 5. The fidelity posture (gopls is go/types, not the compiler) {#fidelity}

##kind-line-fidelity `req r1` @impl/done

##fidelity-spectrum-lead The three stacks now span a fidelity spectrum, and this oracle's place
on it is spec, not fine print: @impl/done

- ##FIDELITY-TS-ORACLE-IS-THE-COMPILER The TS oracle IS the compiler (the LanguageService is tsc's engine —
  agreement by construction). @spec/done
- ##FIDELITY-RUST-ANALYZER-IS-NOT-RUSTC rust-analyzer is NOT rustc (an independent, deliberately partial
  analysis). @spec/done
- ##FIDELITY-GOPLS-STANDS-ON-GO-TYPES **gopls stands on `go/types` — the reference library implementation
  of the Go specification, the same framework `go vet` builds on —
  while the gc compiler type-checks with `types2`, go/types'
  deliberately-synchronized port.** The delta is a maintained-identical
  pair whose divergences are treated as bugs upstream: far tighter
  than rust-analyzer↔rustc, still not identity. @spec/done

##consequences-lead Consequences, all normative: @impl/done

- ##CLEAN-VALIDATE-DOES-NOT-CERTIFY-A-CLEAN-FLOOR A clean `validate` does NOT certify a clean floor. The floor
  (`go-ai-native floor` → gofmt/vet/build/test) remains the truth;
  consumer-facing docs repeat it. @impl/done
- ##DIFFERENTIAL-CORPUS-PINS-DIAGNOSTIC-CLASSES The differential corpus curates diagnostic classes and pins each to
  the floor's own verdict (`go build` / `go vet` exit + message class)
  through a committed mapping table: type mismatch, undeclared name,
  wrong argument count, unknown field, missing return, unused
  import/variable. @impl/done
- ##KNOWN-ASYMMETRIES-ARE-DOCUMENTED-GAP-CASES Known asymmetries are DOCUMENTED-GAP corpus cases, not omissions —
  the standing candidates to probe in Phase 7: diagnostics gated on
  saved-vs-overlay state, `go.mod`-dependent resolution under a pure
  overlay, and vet-only findings (printf shapes) that the floor
  reports and the oracle may not. Each observed asymmetry becomes a
  corpus case asserting exactly that shape, so the gap list never
  rots. @impl/done

## 6. Quiescence, degradation, never crashes {#degradation}

##kind-line-degradation `req r1` @impl/done

##workspace-load-after-initialized After `initialized`, the server loads the workspace (go.mod parsing,
package metadata, cache priming). @spec/done

##READINESS-WAIT-IS-DEADLINE-BOUNDED The bridge bounds its readiness wait
by a deadline keyed on `workDoneProgress` end events for the initial
load; a deadline pass degrades: answers carry `degraded: true`, so
callers can distinguish warm truth from cold best-effort. @impl/done

##PROGRESS-DRAIN-HEURISTIC-IS-INHERITED-AS-A-WARNING The Rust
campaign's falsified progress-drain heuristic is inherited as a
WARNING, not a mechanism: no wait strategy is trusted until the
Phase-7 live chain pins gopls's actual signalling, and a replay test
pins whatever is chosen. @impl/done

##b5-extends-to-the-whole-session-lead B5 extends to the whole session: @impl/done

- ##UNKNOWN-OP-ANSWERS-A-PROTOCOL-ERROR an op the relay does not know answers a protocol error naming the
  known set; @impl/done
- ##ANALYZER-CRASH-ENDS-THE-SESSION an analyzer crash surfaces `oracle-crashed` op-grain and ends the
  session (the product registry owns respawn-once); @impl/done
- ##NO-INPUT-MAY-POISON-THE-SESSION no input may poison the session. @impl/done

## 7. Process lifecycle and Windows discipline {#lifecycle}

##kind-line-lifecycle `req r1` @impl/done

##ONE-LONG-LIVED-CHILD-PER-ROOT-SESSION One long-lived child per (root, session). @impl/done

##GRACEFUL-EXIT-IS-THE-LSP-DANCE Graceful exit is the LSP
dance — `shutdown` request, `exit` notification — with kill-on-drop as
the backstop; the no-zombie property is test-asserted. @impl/done

##PATHS-BECOME-URIS-AFTER-VERBATIM-PREFIX-STRIPPING Paths become
URIs only after verbatim-prefix stripping (`\\?\` breaks child argv
and URI builders — the standing house lesson). @impl/done

##STDOUT-CARRIES-LSP-FRAMES-ONLY stdout carries LSP
frames only; gopls's own stderr chatter is drained and discarded by
the reader (surfaced only in bridge debug logging), so protocol
streams stay clean. @impl/done

## 8. Latency posture {#latency}

##kind-line-latency `req r1` @impl/done

##TARGETS-ARE-POSTED-AND-MEASURED-NEVER-GATED Targets are POSTED and MEASURED, never CI-gated (the standing split:
gate what cannot flake, record what can). @impl/done

##posted-targets-lead Posted targets for
demo-class trees: @impl/done

- ##TARGET-WARM-VALIDATE warm `validate` p50 < 500 ms, @impl/done
- ##TARGET-COMPLETE `complete` p50 <
  300 ms, @impl/done
- ##TARGET-COLD-INIT cold init-to-ready < 15 s. @impl/done

##BENCH-HARNESS-RECORDS-DISTRIBUTIONS The bench harness
(`go-ai-native-tcg bench`) records distributions per run; the Phase-7
ledger entry carries the first measured set on `research/go-demo`,
and a target that moves, moves in a committed REPORT with a reason. @impl/done

##LARGE-WORKSPACE-COLD-INIT-WARNING Large-workspace consumers are warned about the product's 60 s
first-request ceiling; the relay's eager init at `serve` start spends
the cold cost as early as possible. @impl/done
