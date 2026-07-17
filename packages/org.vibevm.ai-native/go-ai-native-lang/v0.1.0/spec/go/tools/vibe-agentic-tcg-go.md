# Tool Spec: `vibe-agentic-tcg-go` — the Agentic Type Oracle for Go
*Status: component brief at FULL seven-section parity (problem · design
stance · component shape · staged ambition · licensing · risk register ·
summary). Commissioned 2026-07-17 by the GO-AI-NATIVE-PLAN mandate as the
Go member of the agentic tcg line — the sibling of `go-ai-native-tcg.md`
(token-level, VERY-FAR-future by the owner's ruling) and of the shipped
TS/Rust twins. Mechanism specs:
[`TCG-ORACLE-GO-v0.1`](../mechanisms/TCG-ORACLE-GO-v0.1.md),
[`TCG-PROTOCOL-GO-v0.1`](../mechanisms/TCG-PROTOCOL-GO-v0.1.md); the
product-side MCP seam is vibevm's PROP-026 (the `language` parameter was
cut for exactly this kind of arrival: a new value, not new tools).*

## 1. What problem it solves

Go is the Discipline's third supported language, and a Go-editing agent
today learns about a type error at write-grain latency: write the file,
run `go build`/`go vet` (fast, but a full write-and-shell round trip),
parse stderr, retry. The TS and Rust campaigns proved that most of a
logit-mask's value is deliverable WITHOUT the mask — information ("what
is in scope, does this type-check"), feedback latency (milliseconds on a
warm session instead of a write-and-run loop), and generation-time
discipline (the gate's own rules answered while writing) — as tools the
agent consults. The mask's GUARANTEE stays with the floor; the oracle
makes red floor iterations rare.

The Go twin arrives with the friendliest fidelity story of the three,
stated up front rather than discovered later: **gopls stands on
`go/types`, the reference library implementation of the Go
specification** — the same framework `go vet` builds on — while the gc
compiler type-checks with types2, go/types' deliberately-synchronized
port. The oracle is therefore far closer to "the compiler's own answer"
than rust-analyzer is to rustc, and one honest step short of the TS
oracle (which IS tsc's engine). The floor (`go-ai-native floor` →
gofmt/vet/build/test) remains the truth, verbatim.

## 2. Design stance (consequences of what we know)

- **Stand on the consumer's gopls, over LSP.** The oracle process is
  the CONSUMER's own `gopls` (PATH → GOBIN → GOPATH/bin resolution from
  the project root; absence is a recipe-carrying error, never a silent
  skip — `go install golang.org/x/tools/gopls@latest`). No bespoke type
  engine, no library embedding: importing gopls's internals is not a
  supported surface, and an in-process `go/packages`-based oracle would
  rebuild the completions/hover gopls already ships. (A far-future
  embedding via `golang.org/x/tools` public APIs is the Go analog of
  the far-backlogged `ra_ap_*` line — re-enters planning only by the
  owner's word.)
- **Overlays, never disk.** LSP text-document overlays (`didOpen` /
  `didChange` with monotonic per-document versions) validate the
  HYPOTHETICAL edit before it lands. A hypothetical new file in an
  existing package participates via `didOpen` alone.
- **Both diagnostics channels, capability-negotiated.** gopls's
  diagnostics have historically been push-model; pull support is newer.
  The bridge prefers pull when granted and otherwise collects pushed
  diagnostics for the target document under a bounded settle window —
  whichever the shipped gopls grants is pinned by the live chain
  (TCG-ORACLE-GO §2). No heuristic is trusted untested — the Rust
  campaign's falsified progress-drain lesson is inherited as a warning.
- **Enrichment is in-process — one engine, zero drift.** The gate's
  fact source (the stdlib-only go-extract sidecar) is spawned by the
  same bridge machinery the conform frontend uses, so the relay runs
  the REAL conform rules (`go_ai_native_conform::build_rules` →
  `conform_core::check`) on the overlay content. Findings come back
  flagged against the project's frozen ratchet baseline, with Class-F
  advice citing GUIDE REQs; a finding-parity test diffs the relay's
  finding set against `go-ai-native-conform check` on the same file.
- **Dual transport, agent's choice** (PROP-018 §2.8): a persistent
  `serve` relay for MCP hosts, and one-shot CLI forms
  (`vibe bin exec go-ai-native-tcg -- validate …`) for agents without
  MCP. Same protocol shape on both.
- **Capability routing is free here.** A tool you may ignore does not
  distort (DR1-015 inverted) — and the TS battery's Stage-A null adds
  the honest complement: a tool merely offered may not help until
  delivery binds. This brief ships mechanics; delivery experiments are
  the backlogged cross-language Stage B.

## 3. Component shape (how it fits vibevm)

Three processes; enrichment happens inside the relay (the Rust twin's
topology, with go-extract as the fact sidecar where Rust calls a
library):

```
agent ──MCP (tcg_validate/tcg_scope/tcg_complete/tcg_type,
   │         language:"go")──▶ vibe mcp serve
   │                           (vibe-tcg registry: lazy spawn,
   │                            slot dispatch, consent)
   └─or one-shot─▶ vibe bin exec go-ai-native-tcg -- <op> ──▶ go-ai-native-tcg
                                            (TCG-PROTOCOL-GO frames up;
                                             enrichment in-process:
                                             go-extract facts+markers →
                                             conform rules → findings,
                                             baseline flags, advice)
                                                │ LSP over stdio
                                                ▼
                                              gopls
                                              (consumer's install;
                                               overlays, diagnostics,
                                               hover, completion)
```

- **The bridge** (`crates/go-ai-native-tcg-bridge`, this package): the
  LSP client as a seam — Content-Length framing, capability-negotiated
  handshake, overlay/version bookkeeping, readiness wait with a
  `degraded` flag, five-way typed error taxonomy, replay-tested without
  gopls.
- **The CLI** (`crates/go-ai-native-tcg`, bin **`go-ai-native-tcg`**,
  the package's 4th `[[binary]]`): `serve` (the enriching relay;
  self-inits with the project's conform.toml topology so a host's first
  frame can be `validate`) + one-shot ops + `bench` (the corpus/latency
  harness).
- **The product seam** (vibevm, PROP-026): the SAME four `tcg_*` tools;
  `language: "go"` dispatches through the lockfile to this package's
  slot artifact. No new tools, no new PROP — the enum-value promise,
  cashed a second time.
- **Determinism and auditability:** given (project state, overlay set,
  policy, gopls version), answers are deterministic modulo gopls's own
  analysis; enriched findings cite `spec://` REQs; nothing samples.

## 4. Staged ambition

- **Stage A — the consultation oracle (THIS brief):** validate / scope /
  complete / type over LSP overlays, discipline-enriched, MCP +
  one-shot delivery, mechanics proven by the differential corpus and
  the bench baseline on `research/go-demo`.
- **Stage B — richer discipline advice:** defined-type constructor
  suggestions at seams, policy-ranked completions, cell-topology
  answers — fields the protocol already carries.
- **Stage C — delivery experiments** (cross-language, with the TS and
  Rust twins, when the owner commissions the delivery line): MCP-mounted
  and write-path-hook arms over a Go task battery on `research/go-demo`.
- **Stage D — the x/tools embedding (FAR BACKLOG):** in-process
  semantic access replacing the LSP child — richer scope/brand answers,
  custom traversals — at the cost of pinning an analyzer version.
  Re-enters planning only by the owner's word.
- **Stage E — token-level TCG** (`go-ai-native-tcg.md`, VERY-FAR-future
  by the owner's 2026-07-17 ruling): the same oracle answers a
  logit-masker inside a decode loop when an inference substrate exists.
  Nothing here is thrown away; the consumer of the answers changes.

## 5. Licensing posture

- gopls / golang.org/x/tools: BSD-3 — and the consumer brings their own
  binary (we spawn, never link, never vendor).
- LSP: an open specification; the client is in-house code.
- go-extract: in-house, stdlib-only Go.
- The conform engines: in-house (this package + vendored
  core-ai-native copies).
- Net: nothing viral anywhere near the critical path; the owner's
  "minimum external tooling" ideal is met — one official ecosystem
  binary plus our own code.

## 6. The honest risk register

- **gopls is go/types, not the compiler — the defining nuance, owned in
  the spec.** The delta to types2 is a synchronized pair, far tighter
  than r-a↔rustc, but saved-state assumptions, `go.mod` resolution
  under pure overlays, and vet-only findings can diverge from the
  floor's verdict. The corpus curates within competence, a mapping
  table makes the delta inspectable, documented-gap cases keep known
  silences visible, and every consumer-facing doc says: the floor is
  the truth.
- **No guarantee by construction.** The agent may ignore the oracle —
  the TS Stage-A null says a merely-offered tool goes unconsulted by
  weak models. Mechanics ship here; delivery binding is Stage C's
  measured question.
- **Diagnostics-channel variance across gopls versions.** Push vs pull
  differs by release; both paths ship, capability-gated, and the live
  chain pins the shipped behavior. Replay transcripts keep the unit
  suite gopls-free either way.
- **Cold init on large workspaces.** Module graph loading dominates;
  posted targets (< 15 s demo-class) move only with a recorded REPORT
  reason; the eager-init-at-serve-start posture and the `degraded`
  flag are the mitigations.
- **Windows child lifecycle.** The house lesson set applies
  (verbatim-free paths into URIs, kill-on-drop, shutdown/exit dance,
  no-zombie assertions).
- **Scope creep toward an LSP relay.** The surface is the four queries
  + lifecycle, full stop (PROP-026 §6); rename/code-actions/references
  go through the owner.

## 7. One-line summary

`vibe-agentic-tcg-go` gives a coding agent millisecond-class,
overlay-true Go answers — gopls diagnostics of unwritten edits,
in-scope symbols, type-valid completions, and discipline findings from
the same conform engine as the gate — through the same four `tcg_*`
tools the TS and Rust twins proved (`language: "go"`), honestly
labelled as go/types' answer rather than the compiler's, so agents stop
paying the write-and-rerun tax while the mask (the only thing this is
NOT) waits for an inference substrate on its own very-far-future track.
