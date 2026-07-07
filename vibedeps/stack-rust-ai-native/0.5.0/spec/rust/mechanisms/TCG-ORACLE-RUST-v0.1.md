# TCG-ORACLE-RUST v0.1 — the rust-analyzer oracle process model {#root}

**Status: v0.1 — authored with AGENTIC-TCG-RUST-PLAN v0.1 (Phase 1),
implemented by its Phases 3–4.** The component brief is
[`tools/vibe-agentic-tcg-rust.md`](../tools/vibe-agentic-tcg-rust.md);
the message grammar is
[`TCG-PROTOCOL-RUST-v0.1`](TCG-PROTOCOL-RUST-v0.1.md). This document
owns the oracle PROCESS: resolution, LSP lifecycle, configuration,
overlays, quiescence, the approximation posture, and latency. Spike
facts cited here were measured against rust-analyzer 1.93.1 on
2026-07-07 (AGENTIC-TCG-RUST-PLAN Phase 0).

## 1. The process and its resolution {#resolution}

`req r1`

The oracle process is the CONSUMER's own `rust-analyzer` binary — the
stack never bundles, links, or vendors an analyzer. Resolution order,
run from the project root so `rust-toolchain.toml` pinning is
honoured, each failure recipe-carrying and never silently skipped:

1. `rustup which rust-analyzer` (the toolchain's component);
2. `rust-analyzer` on PATH;
3. hard failure: the bridge's `rust-analyzer-missing` error with the
   recipe `rustup component add rust-analyzer`.

Installing this stack OBLIGES the machine to carry rust-analyzer (the
same posture as node ≥ 22.6 for the TS stack): inside the stack's own
test suite an absent analyzer is a recipe-carrying FAILURE, never a
skip; outside the stack no obligation exists — a project without
rust-ai-native gets the product's not-installed recipe and owes
nothing. The resolved path and the server's reported version land in
the `init` result.

## 2. LSP session and capabilities {#session}

`req r2`

The bridge speaks LSP 3.17 over the child's stdio (Content-Length
framing). The `initialize` request declares: utf-8 in
`general.positionEncodings` (granted by 1.93.1 — positions then need
line-base conversion only; the utf-16 fallback converts through the
line's text and is unit-tested on non-ASCII content), pull diagnostics
(`textDocument.diagnostic`), `window.workDoneProgress`, and the
experimental `serverStatusNotification`. Every downstream feature keys
off the GRANTED set — a capability the server did not grant degrades
per §6 into a well-formed error or a documented fallback, never a
crash. The bridge answers the server's own requests:
`workspace/configuration` (with §3's config object),
`window/workDoneProgress/create` and `client/registerCapability` (null
results).

## 3. Configuration: experimental diagnostics, deliberately on {#config}

`req r3`

The spike's central finding: rust-analyzer's most valuable native
diagnostics — type-mismatch (E0308), unresolved-name (E0425) — sit
behind the DEFAULT-OFF `diagnostics.experimental.enable`. A
null-config oracle answers silence for the very classes the tool
exists to catch. The bridge therefore ships one config object —
`{"diagnostics": {"experimental": {"enable": true}}}` — passed BOTH as
`initializationOptions` and as every `workspace/configuration` answer.
This is a deliberate, documented posture: the enabled set is
experimental by rust-analyzer's own naming, which is one more reason
§5's approximation statement is spec, not fine print. Future config
needs (feature flags, cargo target selection) extend this object in
one place.

## 4. Overlays and versions {#overlays}

`req r4`

An overlay is an LSP-owned text document: `didOpen {uri, version: 1,
text}` claims the document (the server stops reading disk for it),
`didChange` with full-text sync and a MONOTONICALLY increasing
per-document version replaces it, `didClose` releases it back to disk.
The rules the TS campaign learned the hard way are LSP-native law
here and the bridge enforces them structurally:

- versions never repeat or reset within a session (a monotonic counter
  per document, never derived from content);
- `validate` WITHOUT inline content reads the disk file and opens it
  with that text, so version bookkeeping has exactly one owner (the
  bridge) and a later disk edit is picked up by the next validate's
  `didChange`;
- an overlaid file need not exist on disk — a hypothetical new module
  participates via `didOpen` alone (spike-proven: a seeded error in a
  pure overlay is diagnosed with zero disk writes);
- `update {content: null}` maps to `didClose`.

## 5. The approximation posture (r-a is not rustc) {#approximation}

`req r5`

The TS oracle answers with tsc's own engine; THIS oracle answers with
rust-analyzer's independent analysis, which is deliberately partial.
Consequences, all normative:

- A clean `validate` does NOT certify a clean floor. The floor
  (`discipline-rust floor` → cargo check) remains the truth;
  consumer-facing docs repeat it.
- The differential corpus curates classes INSIDE r-a's native
  competence; each class is pinned to cargo check through the
  committed mapping table (1.93.1 rows: E0308↔E0308, E0425↔E0425,
  E0107↔E0061 arity, E0559↔E0609 unknown-field, E0063↔E0063,
  E0599↔E0599). Diagnostic CODES may differ for the same defect;
  existence-grain agreement is the claim, through the table.
- Known silences are DOCUMENTED-GAP corpus cases, not omissions:
  privacy at 1.93.1 is the standing exhibit — the oracle answers
  nothing while cargo check speaks (rustc's code depends on the
  reference shape: E0423 for a use-imported tuple constructor, E0603
  for the module-path form — one defect class, two codes; the corpus
  pins the E0423 shape). The case asserts exactly that asymmetry so a
  future r-a flips it red and the gap list never rots.
- Borrow-check subtleties, trait-solver edges, and macro-heavy code
  are named as the open delta class; no corpus case claims them.

## 6. Quiescence, degradation, never crashes {#degradation}

`req r6`

After `initialized`, the server loads the workspace (cargo metadata,
cache priming). The bridge waits for `experimental/serverStatus` with
`quiescent: true`, bounded by a deadline — and that flag is the ONLY
trusted signal. Two live-chain findings harden this (2026-07-07,
Phase 3): (a) rust-analyzer does NOT echo `serverStatusNotification`
in its InitializeResult even though it honours the declared client
capability, so there is nothing to key a capability check off — the
bridge declares and trusts the channel; (b) a progress-drain
heuristic ("initial workDoneProgress tokens ended") was tried and
FALSIFIED twice — a fast first token drains while indexing continues,
yielding confident empty answers — so it is deliberately ABSENT, and
a replay test pins that progress noise never satisfies the wait. A
deadline pass degrades: answers carry `degraded: true`, so callers
can distinguish warm truth from cold best-effort. B5 extends to the whole
session: an op the relay does not know answers a protocol error naming
the known set; an analyzer crash surfaces `oracle-crashed` op-grain
and ends the session (the product registry owns respawn-once); no
input may poison the session.

## 7. Process lifecycle and Windows discipline {#lifecycle}

`req r7`

One long-lived child per (root, session). Graceful exit is the LSP
dance — `shutdown` request, `exit` notification — with kill-on-drop as
the backstop; the no-zombie property is test-asserted (spike-proven:
clean exit code 0, no surviving pid). Paths become URIs only after
verbatim-prefix stripping (`\\?\` breaks child argv and URI builders —
the standing lesson's fourth home). stdout carries LSP frames only;
rust-analyzer's own stderr chatter is drained and discarded by the
reader (surfaced only in bridge debug logging), so protocol streams
stay clean.

## 8. Latency posture {#latency}

`req r8`

Targets are POSTED and MEASURED, never CI-gated (the standing split:
gate what cannot flake, record what can). Spike facts on a minimal
crate, this box: init handshake ~10 ms; init-to-quiescent 14.7 s
cache-COLD (sysroot indexing dominates) and 2.5 s warm; warm pull
diagnostics 1–2 ms; hover ~1 ms; completion ~19 ms at 118 entries.
Posted targets for demo-class trees: warm `validate` p50 < 500 ms,
`complete` p50 < 300 ms, cold init-to-quiescent < 15 s. The bench
harness records distributions per run; a target that moves, moves in a
committed REPORT with a reason — and per the owner's resolution a miss
CANCELS NOTHING: the campaign proceeds and the miss is reported
prominently. Large-workspace consumers are warned about the product's
60 s first-request ceiling; the relay's eager init at `serve` start
(before the host's first frame) spends the cold cost as early as
possible.
