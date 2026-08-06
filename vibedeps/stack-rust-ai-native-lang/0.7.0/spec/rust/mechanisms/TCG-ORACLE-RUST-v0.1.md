# TCG-ORACLE-RUST v0.1 — the rust-analyzer oracle process model {#root}

<status stage="spec" state="done"/>

@fact:status-line **Status: v0.1 — authored with AGENTIC-TCG-RUST-PLAN v0.1 (Phase 1),
implemented by its Phases 3–4.** @status:impl/done

@fact:companion-documents The component brief is
[`tools/vibe-agentic-tcg-rust.md`](../tools/vibe-agentic-tcg-rust.md);
the message grammar is
[`TCG-PROTOCOL-RUST-v0.1`](TCG-PROTOCOL-RUST-v0.1.md). @status:impl/done

@fact:DOCUMENT-OWNS-THE-ORACLE-PROCESS This document
owns the oracle PROCESS: resolution, LSP lifecycle, configuration,
overlays, quiescence, the approximation posture, and latency. @status:impl/done

@fact:SPIKE-FACTS-MEASURED-AGAINST-1-93-1 Spike
facts cited here were measured against rust-analyzer 1.93.1 on
2026-07-07 (AGENTIC-TCG-RUST-PLAN Phase 0). @status:impl/done

## 1. The process and its resolution {#resolution}

@fact:kind-line-resolution `req r1` @status:impl/done

@fact:ORACLE-IS-THE-CONSUMERS-OWN-RUST-ANALYZER The oracle process is the CONSUMER's own `rust-analyzer` binary — the
stack never bundles, links, or vendors an analyzer. @status:impl/done

@fact:resolution-order-lead Resolution order,
run from the project root so `rust-toolchain.toml` pinning is
honoured, each failure recipe-carrying and never silently skipped: @status:impl/done

1. @fact:RESOLUTION-RUSTUP-WHICH `rustup which rust-analyzer` (the toolchain's component); @status:impl/done
2. @fact:RESOLUTION-PATH `rust-analyzer` on PATH; @status:impl/done
3. @fact:RESOLUTION-HARD-FAILURE hard failure: the bridge's `rust-analyzer-missing` error with the
   recipe `rustup component add rust-analyzer`. @status:impl/done

@fact:STACK-OBLIGES-THE-MACHINE-TO-CARRY-RUST-ANALYZER Installing this stack OBLIGES the machine to carry rust-analyzer (the
same posture as node ≥ 22.6 for the TS stack): inside the stack's own
test suite an absent analyzer is a recipe-carrying FAILURE, never a
skip; outside the stack no obligation exists — a project without
rust-ai-native gets the product's not-installed recipe and owes
nothing. @status:impl/done

@fact:RESOLVED-PATH-AND-VERSION-LAND-IN-INIT The resolved path and the server's reported version land in
the `init` result. *Specified, not built — half of it. The version half ships:
`init_result` emits `ra_version` (`crates/rust-ai-native-tcg/src/serve.rs:76-86`),
beside `position_encoding`, `pull_diagnostics` and `quiescent`. The path half
does not: the path IS resolved — `resolve_rust_analyzer`
(`crates/rust-ai-native-tcg-bridge/src/lib.rs:146`) returns it — but it is never
put into the result, and `ra_path` occurs in this package only as the shape
`TCG-PROTOCOL-RUST-v0.1.md#OP-INIT` promises, never as a field the code emits.* @status:spec/done

## 2. LSP session and capabilities {#session}

@fact:kind-line-session `req r2` @status:impl/done

@fact:BRIDGE-SPEAKS-LSP-3-17-OVER-STDIO The bridge speaks LSP 3.17 over the child's stdio (Content-Length
framing). @status:impl/done

@fact:initialize-declares-lead The `initialize` request declares: @status:impl/done

- @fact:DECLARES-UTF8-POSITION-ENCODING utf-8 in
  `general.positionEncodings` (granted by 1.93.1 — positions then need
  line-base conversion only; the utf-16 fallback converts through the
  line's text and is unit-tested on non-ASCII content), @status:impl/done
- @fact:DECLARES-PULL-DIAGNOSTICS pull diagnostics
  (`textDocument.diagnostic`), @status:impl/done
- @fact:DECLARES-WORK-DONE-PROGRESS `window.workDoneProgress`, @status:impl/done
- @fact:DECLARES-SERVER-STATUS-NOTIFICATION and the
  experimental `serverStatusNotification`. @status:impl/done

@fact:DOWNSTREAM-FEATURES-KEY-OFF-THE-GRANTED-SET Every downstream feature keys
off the GRANTED set — a capability the server did not grant degrades
per §6 into a well-formed error or a documented fallback, never a
crash. @status:impl/done

@fact:BRIDGE-ANSWERS-THE-SERVERS-OWN-REQUESTS The bridge answers the server's own requests:
`workspace/configuration` (with §3's config object),
`window/workDoneProgress/create` and `client/registerCapability` (null
results). @status:impl/done

## 3. Configuration: experimental diagnostics, deliberately on {#config}

@fact:kind-line-config `req r3` @status:impl/done

@fact:SPIKE-FINDING-EXPERIMENTAL-DIAGNOSTICS-ARE-DEFAULT-OFF The spike's central finding: rust-analyzer's most valuable native
diagnostics — type-mismatch (E0308), unresolved-name (E0425) — sit
behind the DEFAULT-OFF `diagnostics.experimental.enable`. @status:spec/done

@fact:NULL-CONFIG-ORACLE-ANSWERS-SILENCE A
null-config oracle answers silence for the very classes the tool
exists to catch. @status:spec/done

@fact:BRIDGE-SHIPS-ONE-CONFIG-OBJECT The bridge therefore ships one config object —
`{"diagnostics": {"experimental": {"enable": true}}}` — passed BOTH as
`initializationOptions` and as every `workspace/configuration` answer. @status:impl/done

@fact:ENABLING-EXPERIMENTAL-IS-A-DOCUMENTED-POSTURE This is a deliberate, documented posture: the enabled set is
experimental by rust-analyzer's own naming, which is one more reason
§5's approximation statement is spec, not fine print. @status:spec/done

@fact:FUTURE-CONFIG-NEEDS-EXTEND-ONE-OBJECT Future config
needs (feature flags, cargo target selection) extend this object in
one place. @status:spec/done

## 4. Overlays and versions {#overlays}

@fact:kind-line-overlays `req r4` @status:impl/done

@fact:OVERLAY-IS-AN-LSP-OWNED-TEXT-DOCUMENT An overlay is an LSP-owned text document: `didOpen {uri, version: 1,
text}` claims the document (the server stops reading disk for it),
`didChange` with full-text sync and a MONOTONICALLY increasing
per-document version replaces it, `didClose` releases it back to disk. @status:impl/done

@fact:rules-are-lsp-native-law-lead The rules the TS campaign learned the hard way are LSP-native law
here and the bridge enforces them structurally: @status:impl/done

- @fact:OVERLAY-RULE-VERSIONS-NEVER-REPEAT versions never repeat within an
  overlay's lifetime (a monotonic counter per open document, never derived
  from content); clearing an overlay closes the document and a later
  reopen starts again at 1 —
  `crates/rust-ai-native-tcg-bridge/src/oracle.rs:184` (`docs.remove`); @status:impl/done
- @fact:OVERLAY-RULE-VALIDATE-WITHOUT-CONTENT-READS-DISK `validate` WITHOUT inline content reads the disk file and opens it
  with that text, so version bookkeeping has exactly one owner (the
  bridge) and a later disk edit is picked up by the next validate's
  `didChange`; @status:impl/done
- @fact:OVERLAY-RULE-FILE-NEED-NOT-EXIST-ON-DISK an overlaid file need not exist on disk — a hypothetical new module
  participates via `didOpen` alone (spike-proven: a seeded error in a
  pure overlay is diagnosed with zero disk writes); @status:impl/done
- @fact:OVERLAY-RULE-NULL-CONTENT-MAPS-TO-DIDCLOSE `update {content: null}` maps to `didClose`. @status:impl/done

## 5. The approximation posture (r-a is not rustc) {#approximation}

@fact:kind-line-approximation `req r5` @status:impl/done

@fact:THIS-ORACLE-ANSWERS-WITH-INDEPENDENT-ANALYSIS The TS oracle answers with tsc's own engine; THIS oracle answers with
rust-analyzer's independent analysis, which is deliberately partial. @status:spec/done

@fact:consequences-all-normative-lead Consequences, all normative: @status:impl/done

- @fact:CLEAN-VALIDATE-DOES-NOT-CERTIFY-A-CLEAN-FLOOR A clean `validate` does NOT certify a clean floor. The floor
  (`rust-ai-native floor` — the seven steps: cargo fmt → cargo test →
  clippy → conform → specmap → test-gate → fast-loop; the compile rides
  inside the test step) remains the truth; consumer-facing docs repeat
  it. @status:impl/done
- @fact:DIFFERENTIAL-CORPUS-CURATES-NATIVE-COMPETENCE The differential corpus curates classes INSIDE r-a's native
  competence; each class is pinned to cargo check through the
  committed mapping table (1.93.1 rows: E0308↔E0308, E0425↔E0425,
  E0107↔E0061 arity, E0559↔E0609 unknown-field, E0063↔E0063,
  E0599↔E0599). Diagnostic CODES may differ for the same defect;
  existence-grain agreement is the claim, through the table. @status:impl/done
- @fact:KNOWN-SILENCES-ARE-DOCUMENTED-GAP-CASES Known silences are DOCUMENTED-GAP corpus cases, not omissions:
  privacy at 1.93.1 is the standing exhibit — the oracle answers
  nothing while cargo check speaks (rustc's code depends on the
  reference shape: E0423 for a use-imported tuple constructor, E0603
  for the module-path form — one defect class, two codes; the corpus
  pins the E0423 shape). The case asserts exactly that asymmetry so a
  future r-a flips it red and the gap list never rots. @status:impl/done
- @fact:OPEN-DELTA-CLASS-IS-NAMED Borrow-check subtleties, trait-solver edges, and macro-heavy code
  are named as the open delta class; no corpus case claims them. @status:impl/done

## 6. Quiescence, degradation, never crashes {#degradation}

@fact:kind-line-degradation `req r6` @status:impl/done

@fact:SERVER-LOADS-THE-WORKSPACE-AFTER-INITIALIZED After `initialized`, the server loads the workspace (cargo metadata,
cache priming). @status:spec/done

@fact:BRIDGE-WAITS-FOR-QUIESCENT-SERVER-STATUS The bridge waits for `experimental/serverStatus` with
`quiescent: true`, bounded by a deadline — and that flag is the ONLY
trusted signal. @status:impl/done

@fact:two-live-chain-findings-lead Two live-chain findings harden this (2026-07-07,
Phase 3): @status:impl/done

- @fact:FINDING-A-NO-SERVER-STATUS-ECHO (a) rust-analyzer does NOT echo `serverStatusNotification`
  in its InitializeResult even though it honours the declared client
  capability, so there is nothing to key a capability check off — the
  bridge declares and trusts the channel; @status:spec/done
- @fact:FINDING-B-PROGRESS-DRAIN-HEURISTIC-FALSIFIED (b) a progress-drain
  heuristic ("initial workDoneProgress tokens ended") was tried and
  FALSIFIED twice — a fast first token drains while indexing continues,
  yielding confident empty answers — so it is deliberately ABSENT, and
  a replay test pins that progress noise never satisfies the wait. @status:spec/done

@fact:DEADLINE-PASS-DEGRADES A
deadline pass degrades: answers carry `degraded: true`, so callers
can distinguish warm truth from cold best-effort. @status:impl/done

@fact:b5-extends-to-the-whole-session-lead B5 extends to the whole
session: @status:impl/done

- @fact:B5-UNKNOWN-OP-ANSWERS-A-PROTOCOL-ERROR an op the relay does not know answers a protocol error naming
  the known set; @status:impl/done
- @fact:B5-ANALYZER-CRASH-ENDS-THE-SESSION an analyzer crash surfaces `oracle-crashed` op-grain
  and ends the session (the product registry owns respawn-once); @status:impl/done
- @fact:B5-NO-INPUT-MAY-POISON-THE-SESSION no input may poison the session. @status:impl/done

## 7. Process lifecycle and Windows discipline {#lifecycle}

@fact:kind-line-lifecycle `req r7` @status:impl/done

@fact:ONE-LONG-LIVED-CHILD-PER-ROOT-SESSION One long-lived child per (root, session). @status:impl/done

@fact:GRACEFUL-EXIT-AND-THE-NO-ZOMBIE-PROPERTY Graceful exit is the LSP
dance — `shutdown` request, `exit` notification — with kill-on-drop as
the backstop; the no-zombie property is test-asserted (spike-proven:
clean exit code 0, no surviving pid). *Built 2026-08-05 — the proof caught up
to the mechanism, which had shipped long before it. The dance ships
(`shutdown` at `crates/rust-ai-native-tcg-bridge/src/oracle.rs:356`) and so
does the backstop (`Drop for ChildTransport` → `kill()` then `wait()`,
`client.rs:346-350`). The assertion now ships too:
`dropping_the_oracle_kills_the_child_process_no_zombie`
(`tests/live_oracle.rs`) reads the child PID through
`ChildTransport::child_pid()`, proves that process is alive, drops the wrapper
— deliberately NOT the graceful path, so the kill-on-drop backstop is what is
exercised — and polls the OS process table until the PID is gone, treating a
recycled PID as dead by comparing `start_time`. Between 2026-08-04 and this
build the sentence was a promise; it is now a description.* @status:impl/done

@fact:PATHS-BECOME-URIS-AFTER-PREFIX-STRIPPING Paths become URIs only after
verbatim-prefix stripping (`\\?\` breaks child argv and URI builders —
the standing lesson's fourth home). @status:impl/done

@fact:STDOUT-CARRIES-LSP-FRAMES-ONLY stdout carries LSP frames only;
rust-analyzer's own stderr chatter is drained and discarded by the
reader (surfaced only in bridge debug logging), so protocol streams
stay clean. *Specified, not built — the conclusion holds, the mechanism
described does not. Protocol streams do stay clean, but not by draining:
the child is spawned `.stderr(std::process::Stdio::null())`
(`crates/rust-ai-native-tcg-bridge/src/client.rs:303`, and again at
`lib.rs:163`), so the OS discards the chatter at the pipe and no reader
ever sees it. There is no bridge debug logging to surface it in — the
bridge crate has no logging facility at all.* @status:spec/done

## 8. Latency posture {#latency}

@fact:kind-line-latency `req r8` @status:impl/done

@fact:TARGETS-ARE-POSTED-AND-MEASURED-NEVER-GATED Targets are POSTED and MEASURED, never CI-gated (the standing split:
gate what cannot flake, record what can). @status:impl/done

@fact:spike-facts-lead Spike facts on a minimal
crate, this box: @status:impl/done

- @fact:SPIKE-INIT-HANDSHAKE init handshake ~10 ms; @status:impl/done
- @fact:SPIKE-INIT-TO-QUIESCENT init-to-quiescent 14.7 s
  cache-COLD (sysroot indexing dominates) and 2.5 s warm; @status:impl/done
- @fact:SPIKE-WARM-PULL-DIAGNOSTICS warm pull
  diagnostics 1–2 ms; @status:impl/done
- @fact:SPIKE-HOVER hover ~1 ms; @status:impl/done
- @fact:SPIKE-COMPLETION completion ~19 ms at 118 entries. @status:impl/done

@fact:posted-targets-lead Posted targets for demo-class trees: @status:impl/done

- @fact:TARGET-WARM-VALIDATE warm `validate` p50 < 500 ms, @status:impl/done
- @fact:TARGET-WARM-COMPLETE `complete` p50 < 300 ms — posted, not yet
  measured: the bench harness times `validate` only
  (`crates/rust-ai-native-tcg/src/bench.rs` emits `cold_init_ms`,
  `validate_p50_ms`, `validate_p95_ms` and no `complete` field); the
  measurement corpus is deliberately far-future work (`BACKLOG.md`
  B-042), @status:spec/done
- @fact:TARGET-COLD-INIT-TO-QUIESCENT cold init-to-quiescent < 15 s. @status:impl/done

@fact:BENCH-HARNESS-RECORDS-DISTRIBUTIONS The bench
harness records distributions per run; a target that moves, moves in a
committed REPORT with a reason — and per the owner's resolution a miss
CANCELS NOTHING: the campaign proceeds and the miss is reported
prominently. @status:impl/done

@fact:LARGE-WORKSPACE-CONSUMERS-ARE-WARNED Large-workspace consumers are warned —
the tcg brief's `##RISK-COLD-INIT-ON-LARGE-WORKSPACES` carries the
spec-layer warning (14.7 s cache-cold on a minimal crate; a big consumer
tree may exceed the first-request budget) — and the relay's eager init at
`serve` start (before the host's first frame) spends the cold cost as
early as possible. The shipped ceiling is the **45 s** quiescence budget
(`QUIESCENCE_BUDGET`, `crates/rust-ai-native-tcg/src/lib.rs:33`, used by
`spawn_oracle` and `serve`); this document's own
`##TARGET-COLD-INIT-TO-QUIESCENT` posts < 15 s for demo-class trees. @status:impl/done
