# TCG-ORACLE-RUST v0.1 — the rust-analyzer oracle process model {#root}

<status stage="spec" state="done"/>

##status-line **Status: v0.1 — authored with AGENTIC-TCG-RUST-PLAN v0.1 (Phase 1),
implemented by its Phases 3–4.** @impl/done

##companion-documents The component brief is
[`tools/vibe-agentic-tcg-rust.md`](../tools/vibe-agentic-tcg-rust.md);
the message grammar is
[`TCG-PROTOCOL-RUST-v0.1`](TCG-PROTOCOL-RUST-v0.1.md). @impl/done

##DOCUMENT-OWNS-THE-ORACLE-PROCESS This document
owns the oracle PROCESS: resolution, LSP lifecycle, configuration,
overlays, quiescence, the approximation posture, and latency. @impl/done

##SPIKE-FACTS-MEASURED-AGAINST-1-93-1 Spike
facts cited here were measured against rust-analyzer 1.93.1 on
2026-07-07 (AGENTIC-TCG-RUST-PLAN Phase 0). @impl/done

## 1. The process and its resolution {#resolution}

##kind-line-resolution `req r1` @impl/done

##ORACLE-IS-THE-CONSUMERS-OWN-RUST-ANALYZER The oracle process is the CONSUMER's own `rust-analyzer` binary — the
stack never bundles, links, or vendors an analyzer. @impl/done

##resolution-order-lead Resolution order,
run from the project root so `rust-toolchain.toml` pinning is
honoured, each failure recipe-carrying and never silently skipped: @impl/done

1. ##RESOLUTION-RUSTUP-WHICH `rustup which rust-analyzer` (the toolchain's component); @impl/done
2. ##RESOLUTION-PATH `rust-analyzer` on PATH; @impl/done
3. ##RESOLUTION-HARD-FAILURE hard failure: the bridge's `rust-analyzer-missing` error with the
   recipe `rustup component add rust-analyzer`. @impl/done

##STACK-OBLIGES-THE-MACHINE-TO-CARRY-RUST-ANALYZER Installing this stack OBLIGES the machine to carry rust-analyzer (the
same posture as node ≥ 22.6 for the TS stack): inside the stack's own
test suite an absent analyzer is a recipe-carrying FAILURE, never a
skip; outside the stack no obligation exists — a project without
rust-ai-native gets the product's not-installed recipe and owes
nothing. @impl/done

##RESOLVED-PATH-AND-VERSION-LAND-IN-INIT The resolved path and the server's reported version land in
the `init` result. *Specified, not built — half of it. The version half ships:
`init_result` emits `ra_version` (`crates/rust-ai-native-tcg/src/serve.rs:76-86`),
beside `position_encoding`, `pull_diagnostics` and `quiescent`. The path half
does not: the path IS resolved — `resolve_rust_analyzer`
(`crates/rust-ai-native-tcg-bridge/src/lib.rs:146`) returns it — but it is never
put into the result, and `ra_path` occurs in this package only as the shape
`TCG-PROTOCOL-RUST-v0.1.md#OP-INIT` promises, never as a field the code emits.* @spec/done

## 2. LSP session and capabilities {#session}

##kind-line-session `req r2` @impl/done

##BRIDGE-SPEAKS-LSP-3-17-OVER-STDIO The bridge speaks LSP 3.17 over the child's stdio (Content-Length
framing). @impl/done

##initialize-declares-lead The `initialize` request declares: @impl/done

- ##DECLARES-UTF8-POSITION-ENCODING utf-8 in
  `general.positionEncodings` (granted by 1.93.1 — positions then need
  line-base conversion only; the utf-16 fallback converts through the
  line's text and is unit-tested on non-ASCII content), @impl/done
- ##DECLARES-PULL-DIAGNOSTICS pull diagnostics
  (`textDocument.diagnostic`), @impl/done
- ##DECLARES-WORK-DONE-PROGRESS `window.workDoneProgress`, @impl/done
- ##DECLARES-SERVER-STATUS-NOTIFICATION and the
  experimental `serverStatusNotification`. @impl/done

##DOWNSTREAM-FEATURES-KEY-OFF-THE-GRANTED-SET Every downstream feature keys
off the GRANTED set — a capability the server did not grant degrades
per §6 into a well-formed error or a documented fallback, never a
crash. @impl/done

##BRIDGE-ANSWERS-THE-SERVERS-OWN-REQUESTS The bridge answers the server's own requests:
`workspace/configuration` (with §3's config object),
`window/workDoneProgress/create` and `client/registerCapability` (null
results). @impl/done

## 3. Configuration: experimental diagnostics, deliberately on {#config}

##kind-line-config `req r3` @impl/done

##SPIKE-FINDING-EXPERIMENTAL-DIAGNOSTICS-ARE-DEFAULT-OFF The spike's central finding: rust-analyzer's most valuable native
diagnostics — type-mismatch (E0308), unresolved-name (E0425) — sit
behind the DEFAULT-OFF `diagnostics.experimental.enable`. @spec/done

##NULL-CONFIG-ORACLE-ANSWERS-SILENCE A
null-config oracle answers silence for the very classes the tool
exists to catch. @spec/done

##BRIDGE-SHIPS-ONE-CONFIG-OBJECT The bridge therefore ships one config object —
`{"diagnostics": {"experimental": {"enable": true}}}` — passed BOTH as
`initializationOptions` and as every `workspace/configuration` answer. @impl/done

##ENABLING-EXPERIMENTAL-IS-A-DOCUMENTED-POSTURE This is a deliberate, documented posture: the enabled set is
experimental by rust-analyzer's own naming, which is one more reason
§5's approximation statement is spec, not fine print. @spec/done

##FUTURE-CONFIG-NEEDS-EXTEND-ONE-OBJECT Future config
needs (feature flags, cargo target selection) extend this object in
one place. @spec/done

## 4. Overlays and versions {#overlays}

##kind-line-overlays `req r4` @impl/done

##OVERLAY-IS-AN-LSP-OWNED-TEXT-DOCUMENT An overlay is an LSP-owned text document: `didOpen {uri, version: 1,
text}` claims the document (the server stops reading disk for it),
`didChange` with full-text sync and a MONOTONICALLY increasing
per-document version replaces it, `didClose` releases it back to disk. @impl/done

##rules-are-lsp-native-law-lead The rules the TS campaign learned the hard way are LSP-native law
here and the bridge enforces them structurally: @impl/done

- ##OVERLAY-RULE-VERSIONS-NEVER-REPEAT versions never repeat or reset within a session (a monotonic counter
  per document, never derived from content); @impl/done
- ##OVERLAY-RULE-VALIDATE-WITHOUT-CONTENT-READS-DISK `validate` WITHOUT inline content reads the disk file and opens it
  with that text, so version bookkeeping has exactly one owner (the
  bridge) and a later disk edit is picked up by the next validate's
  `didChange`; @impl/done
- ##OVERLAY-RULE-FILE-NEED-NOT-EXIST-ON-DISK an overlaid file need not exist on disk — a hypothetical new module
  participates via `didOpen` alone (spike-proven: a seeded error in a
  pure overlay is diagnosed with zero disk writes); @impl/done
- ##OVERLAY-RULE-NULL-CONTENT-MAPS-TO-DIDCLOSE `update {content: null}` maps to `didClose`. @impl/done

## 5. The approximation posture (r-a is not rustc) {#approximation}

##kind-line-approximation `req r5` @impl/done

##THIS-ORACLE-ANSWERS-WITH-INDEPENDENT-ANALYSIS The TS oracle answers with tsc's own engine; THIS oracle answers with
rust-analyzer's independent analysis, which is deliberately partial. @spec/done

##consequences-all-normative-lead Consequences, all normative: @impl/done

- ##CLEAN-VALIDATE-DOES-NOT-CERTIFY-A-CLEAN-FLOOR A clean `validate` does NOT certify a clean floor. The floor
  (`rust-ai-native floor` → cargo check) remains the truth;
  consumer-facing docs repeat it. @impl/done
- ##DIFFERENTIAL-CORPUS-CURATES-NATIVE-COMPETENCE The differential corpus curates classes INSIDE r-a's native
  competence; each class is pinned to cargo check through the
  committed mapping table (1.93.1 rows: E0308↔E0308, E0425↔E0425,
  E0107↔E0061 arity, E0559↔E0609 unknown-field, E0063↔E0063,
  E0599↔E0599). Diagnostic CODES may differ for the same defect;
  existence-grain agreement is the claim, through the table. @impl/done
- ##KNOWN-SILENCES-ARE-DOCUMENTED-GAP-CASES Known silences are DOCUMENTED-GAP corpus cases, not omissions:
  privacy at 1.93.1 is the standing exhibit — the oracle answers
  nothing while cargo check speaks (rustc's code depends on the
  reference shape: E0423 for a use-imported tuple constructor, E0603
  for the module-path form — one defect class, two codes; the corpus
  pins the E0423 shape). The case asserts exactly that asymmetry so a
  future r-a flips it red and the gap list never rots. @impl/done
- ##OPEN-DELTA-CLASS-IS-NAMED Borrow-check subtleties, trait-solver edges, and macro-heavy code
  are named as the open delta class; no corpus case claims them. @impl/done

## 6. Quiescence, degradation, never crashes {#degradation}

##kind-line-degradation `req r6` @impl/done

##SERVER-LOADS-THE-WORKSPACE-AFTER-INITIALIZED After `initialized`, the server loads the workspace (cargo metadata,
cache priming). @spec/done

##BRIDGE-WAITS-FOR-QUIESCENT-SERVER-STATUS The bridge waits for `experimental/serverStatus` with
`quiescent: true`, bounded by a deadline — and that flag is the ONLY
trusted signal. @impl/done

##two-live-chain-findings-lead Two live-chain findings harden this (2026-07-07,
Phase 3): @impl/done

- ##FINDING-A-NO-SERVER-STATUS-ECHO (a) rust-analyzer does NOT echo `serverStatusNotification`
  in its InitializeResult even though it honours the declared client
  capability, so there is nothing to key a capability check off — the
  bridge declares and trusts the channel; @spec/done
- ##FINDING-B-PROGRESS-DRAIN-HEURISTIC-FALSIFIED (b) a progress-drain
  heuristic ("initial workDoneProgress tokens ended") was tried and
  FALSIFIED twice — a fast first token drains while indexing continues,
  yielding confident empty answers — so it is deliberately ABSENT, and
  a replay test pins that progress noise never satisfies the wait. @spec/done

##DEADLINE-PASS-DEGRADES A
deadline pass degrades: answers carry `degraded: true`, so callers
can distinguish warm truth from cold best-effort. @impl/done

##b5-extends-to-the-whole-session-lead B5 extends to the whole
session: @impl/done

- ##B5-UNKNOWN-OP-ANSWERS-A-PROTOCOL-ERROR an op the relay does not know answers a protocol error naming
  the known set; @impl/done
- ##B5-ANALYZER-CRASH-ENDS-THE-SESSION an analyzer crash surfaces `oracle-crashed` op-grain
  and ends the session (the product registry owns respawn-once); @impl/done
- ##B5-NO-INPUT-MAY-POISON-THE-SESSION no input may poison the session. @impl/done

## 7. Process lifecycle and Windows discipline {#lifecycle}

##kind-line-lifecycle `req r7` @impl/done

##ONE-LONG-LIVED-CHILD-PER-ROOT-SESSION One long-lived child per (root, session). @impl/done

##GRACEFUL-EXIT-AND-THE-NO-ZOMBIE-PROPERTY Graceful exit is the LSP
dance — `shutdown` request, `exit` notification — with kill-on-drop as
the backstop; the no-zombie property is test-asserted (spike-proven:
clean exit code 0, no surviving pid). *Specified, not built (→ B-044) — the
mechanism, not the proof. The dance ships (`shutdown` at
`crates/rust-ai-native-tcg-bridge/src/oracle.rs:356`) and so does the
backstop (`Drop for ChildTransport` → `kill()` then `wait()`,
`client.rs:346-350`). What does not exist is the assertion: the live test
checks only that `shutdown()` returned (`tests/live_oracle.rs:116`), and no
exit-code check and no surviving-pid or process-table probe exists in this
stack's test surface. «Test-asserted» and «spike-proven» describe a spike,
not a test in the tree.* @impl/plan

##PATHS-BECOME-URIS-AFTER-PREFIX-STRIPPING Paths become URIs only after
verbatim-prefix stripping (`\\?\` breaks child argv and URI builders —
the standing lesson's fourth home). @impl/done

##STDOUT-CARRIES-LSP-FRAMES-ONLY stdout carries LSP frames only;
rust-analyzer's own stderr chatter is drained and discarded by the
reader (surfaced only in bridge debug logging), so protocol streams
stay clean. *Specified, not built — the conclusion holds, the mechanism
described does not. Protocol streams do stay clean, but not by draining:
the child is spawned `.stderr(std::process::Stdio::null())`
(`crates/rust-ai-native-tcg-bridge/src/client.rs:303`, and again at
`lib.rs:163`), so the OS discards the chatter at the pipe and no reader
ever sees it. There is no bridge debug logging to surface it in — the
bridge crate has no logging facility at all.* @spec/done

## 8. Latency posture {#latency}

##kind-line-latency `req r8` @impl/done

##TARGETS-ARE-POSTED-AND-MEASURED-NEVER-GATED Targets are POSTED and MEASURED, never CI-gated (the standing split:
gate what cannot flake, record what can). @impl/done

##spike-facts-lead Spike facts on a minimal
crate, this box: @impl/done

- ##SPIKE-INIT-HANDSHAKE init handshake ~10 ms; @impl/done
- ##SPIKE-INIT-TO-QUIESCENT init-to-quiescent 14.7 s
  cache-COLD (sysroot indexing dominates) and 2.5 s warm; @impl/done
- ##SPIKE-WARM-PULL-DIAGNOSTICS warm pull
  diagnostics 1–2 ms; @impl/done
- ##SPIKE-HOVER hover ~1 ms; @impl/done
- ##SPIKE-COMPLETION completion ~19 ms at 118 entries. @impl/done

##posted-targets-lead Posted targets for demo-class trees: @impl/done

- ##TARGET-WARM-VALIDATE warm `validate` p50 < 500 ms, @impl/done
- ##TARGET-WARM-COMPLETE `complete` p50 < 300 ms — posted, not yet
  measured: the bench harness times `validate` only
  (`crates/rust-ai-native-tcg/src/bench.rs` emits `cold_init_ms`,
  `validate_p50_ms`, `validate_p95_ms` and no `complete` field); the
  measurement corpus is deliberately far-future work (`BACKLOG.md`
  B-042), @spec/done
- ##TARGET-COLD-INIT-TO-QUIESCENT cold init-to-quiescent < 15 s. @impl/done

##BENCH-HARNESS-RECORDS-DISTRIBUTIONS The bench
harness records distributions per run; a target that moves, moves in a
committed REPORT with a reason — and per the owner's resolution a miss
CANCELS NOTHING: the campaign proceeds and the miss is reported
prominently. @impl/done

##LARGE-WORKSPACE-CONSUMERS-ARE-WARNED Large-workspace consumers are warned about the product's
60 s first-request ceiling; the relay's eager init at `serve` start
(before the host's first frame) spends the cold cost as early as
possible. @impl/done
