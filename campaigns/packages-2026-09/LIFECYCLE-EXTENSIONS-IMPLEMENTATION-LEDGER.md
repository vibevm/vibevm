# Lifecycle/extensions — implementation reconciliation ledger

_Rolling owner-facing checkpoint, 2026-08-28. This is the durable execution
map for `TZ-LIFECYCLE-EXTENSIONS-v0.1.md`; PROP-054 remains semantic authority.
`SPEC-DEBT-LIFECYCLE*.md` remains the amendment queue. A row is complete only
with landed commit, source, and decisive test evidence._

## 1. Why this ledger exists

Several worker processes were lost during a multi-hour network outage, while
the old WAL, CONTINUE and TZ header still said that implementation had not
started. The repository and every campaign worktree were reconciled against
`main` before continuing.

Result: **no unique implementation was lost or stranded.** Final R3.2, R7.1 and
R8.1 worktrees contain only packets/reports beyond their landed commits.
Intermediate dirty R2/R3 worktrees were older construction snapshots
superseded by richer files on `main`. Their unique R4/R5 audit decisions were
synthesised here, then 33 obsolete worktrees were removed under rolling GC.
B-107, R3.3, R8.2a grammar, R6.2a/R6.2b, R7.2, R7.3 and the R8.1 portability
hardening have since landed. R3.4 is now fully integrated on `main`: observer,
writer, shared command wire/state, borrowed workspace/install plumbing,
Install/Lifecycle/Update/Reinstall owners, exact displaced continuation and
the final four-family parity matrix. The final quality tail repaired every
panel-discovered stale oracle and discipline ratchet rather than baselining it.
All accepted R3.4 and R7.4-through-A15a worktrees and fan-out worktrees were
reviewed, archived and reclaimed under rolling GC; **only `main` remains**.
Missing later waves were never implemented.

The reusable decisions from untracked architecture/review reports are
synthesised below. Those reports are evidence inputs, not authority and not the
only durable home of a decision.
[`RETROSPECTIVE-SPEC-HARVEST-2026-08-27.md`](RETROSPECTIVE-SPEC-HARVEST-2026-08-27.md)
is the tracked index/queue for report-derived candidates not yet owned by a
later implementation row; every future wave drains its named candidates rather
than depending on untracked `cache/` archaeology.

## 2. Evidence standard and current baseline

- Integration checkpoint: `main` at `2b08c818` contains the complete accepted
  R3.4 chain and R7.4 through A14. A12's application service is `053b7e37` /
  `ba874cdf`; A13's shared compile-trace funnel is `3f01e2dc` / `afdd3adc`.
  A14's credential-free selected-world resolver is `cd793ca9` / `2b08c818`.
  The lower service owns one opaque selected-world bundle, install/resume,
  phase dispatch and non-rendering trace finalisation; CLI still owns registered
  report families, deferred-plan routing, presentation, credentials and LLM
  provider construction; registry/package-source composition is the separate
  neutral A15a crate.
- The last coherent whole-panel boundary is R7.4 A0–A8 at `a4253de7`:
  `self-check: all green`, exit 0 on 2026-08-28. The script's lexical
  denominator advertised 47 while dynamic loop calls executed 54 gates; the
  denominator defect is recorded in PROP-055, and every emitted gate including
  the final whole-run user-home tripwire passed. A12 deliberately used exact
  affected gates; the next whole-panel boundary is A15.
- That boundary's host `vibe check`: 0 errors, 5 warnings, 0 info. Workspace
  conform: 27 visible acknowledged findings, 0 frozen and **0 new**. The
  panel's full workspace tests, clippy `-D warnings`, codegen, wire-diff,
  package/MCP suites, self-traces and markup validation all passed.
- Post-A14 checkpoint `cargo xtask specmap`: 6790 units, 2178 tagged code items, 1959 edges,
  0 suspects, 0 gated orphans, 0 unresolved host edges and 21 standing
  warnings. The 25 non-host edges are outside this map's jurisdiction.
- R6.2b integration evidence on the current tree: `cargo xtask codegen`
  regenerated all 46 schemas with no diff; `cargo test -p vibe-spec --locked`
  passed 628 unit tests plus integration/doc tests; strict package clippy passed
  with `-D warnings`; two independent freeze reviews ended in `PASS` after the
  final hostile diagnostic repair.
- R3.4 observer evidence on the current tree: 17 focused observer REDs and the
  complete 647-test `vibe-spec` library/integration/doc suite pass; targeted
  clippy is clean; independent final freeze is `PASS`. The seam uses generated
  trace-index metadata types, contains unwinding sink panics, performs no
  off-mode clock/allocation/encode, and decides budget stand-down before encode.
- R3.4 writer evidence on the current tree: `4d95a129`; 61 focused trace REDs,
  383 `vibe-workspace` library tests, 55 `vibe-safefs` tests plus six doctests,
  the complete `vibe-wire` suite, targeted three-crate clippy and codegen
  idempotence pass. Two independent post-repair freezes plus one final
  concurrency freeze ended in `PASS`. The writer keeps one generated index,
  create-new snapshots, project-serialized newest-nine retention, exact
  crash residue, a single concurrent soft-ceiling crossing and no observer→
  compiler failure channel.
- R3.4 report/state prerequisite evidence on the current tree: `34d3f363`
  shares one generated `Duration`/`TimingRow`/`CompileTraceReport` across the
  index plus install/lifecycle/update/reinstall roots and validates seven
  relational-law families; `0301f8f2` adds the false-defaulted sticky state
  bit, effective adoption and exact state-proven `SupersededTrace`. The focused
  wire/lifecycle/CLI gates and mutation reds are green; root additionally ran
  the handler-envelope byte-identity RED, post-commit `check-codegen` is clean,
  and an independent final review ended PASS. Disabled report/state members
  remain absent on old bytes; no handler-envelope field was added.
- R3.4 borrowed compilation evidence on the current tree: `be04a184` threads
  one optional borrowed run through `vibe-install` and every real workspace
  unit/node boot compile while the old wrappers pass `None`; attempt allocation
  reacquires only an exact pending occurrence and evolves terminal attempts;
  target-bearing portable bases, emitted-output fingerprints, fresh
  observe-before-declare and traced-only unit sorting are centrally owned.
  Root's merged-tree gate passed all 411 `vibe-workspace` library tests, the
  2/2 cross-crate integration test, fmt and strict three-crate clippy. The
  independent final review ended PASS; 25/26 files are blob-identical to the
  accepted worker commit, and the remaining test differs only by the required
  `RunMetadata.trace_compile=true|false` cross-atom adaptation. Specmap is
  clean at `1298d1af` with zero suspects/gated orphans/unresolved host edges.
- R3.4 command-owner core evidence on the current tree: `cad8ecc1` adds a
  non-creating, lock-serialized `TraceRun::open_existing`, one private non-Clone
  CLI preparation and generic `CommandExit<R> → FinalizedCommand<R>` funnel.
  Adopted/missing runs never start mid-history; state-proven predecessors are
  terminalised before the current open; disabled/park/unavailable paths call no
  finish clock; plain drop performs no I/O. Rich command errors never enter
  trace files (`command failed` is fixed), while the same error object returns
  intact; sealed `BoundedDiagnostic` makes the writer's streaming clamp the
  only whole-message clamp. Root passed 86 writer + 21 CLI owner/security tests,
  fmt and strict two-crate clippy; two independent final reviews ended PASS.
  Specmap is clean at `1a81cffd` with zero suspects/gated orphans/unresolved
  host edges.
- R3.4 install/lifecycle activation evidence on the current tree: `dcbf89b0`
  adds the two command owners, flag/manifest activation, one prepared
  manifest/config/root/workspace epoch, prepared `vibe-install` planning and
  slot-lifecycle siblings, additive post-apply Workspace return, the typed
  report/failure/presentation funnel and Fresh/Ready hosted resume continuity.
  Root's merged-tree gate passed all 434 `vibe-workspace` library tests, every
  `vibe-install` suite, 594 CLI unit tests, the five trace targets (7/7/7/5/3),
  three resume paths, the direct-callback failure RED and the exact lifecycle /
  hosted / update compatibility targets. Workspace-wide check, fmt and strict
  four-crate clippy are clean; three independent final reviews ended PASS.
  The two old per-row-echo tests were subsequently migrated to the single
  command-root contribution member by `eb2e7148` and are green.
- R3.4 Update/Reinstall completion evidence: `e589bdaa` gives both commands one
  selected input/identity epoch, one borrowed recorder, owned drafts and the
  four-family funnel; `ee63f4a1`/`a045e1f2` close bounded state-proven
  supersession, honest unavailable wording and exact-run continuation
  ownership; `30482dcc`, `d7676be8`, `eb2e7148` and `cebdedc5` land the hard
  failure, hosted resume/unavailable and compatibility/parity REDs. Central
  gates passed all 650 CLI unit tests, all 434 `vibe-workspace` library tests,
  the complete `vibe-install` suite, 21 focused trace/hosted/lifecycle targets,
  workspace-wide all-target check, fmt and strict four-crate clippy. The matrix
  proves scoped/whole Update, plain/empty/normal-force Reinstall, success,
  park, flagless resume, missing adopted trace, displacement, Ready SlotFailed,
  postcompile hard failure, quiet and exact trace-member omission / root
  invocation compatibility. One accepted exception is explicit below:
  Reinstall member invocation now reports the selected node rather than the
  older, incorrect workspace-root identity.
- R7.3 integration evidence on the current tree: state transaction tests 5/5;
  hosted cancellation/progress/sequential-slot e2e 2/5/1; targeted five-crate
  clippy clean; 48-schema codegen idempotent; specmap has zero gated orphans;
  the sixth independent freeze ended in `PASS` after two root-owned truth-tail
  fixes.
- R7.5 P1 wire evidence on the current tree: normative coherence amendment
  `6d843467`, generated/schema/validator/corpus atom `d3a9d59b`, specmap
  `55937044`. The root independently passed all 245 `vibe-wire` tests, strict
  `vibe-wire`/xtask clippy, conform 27 known / 0 new, post-commit
  `check-codegen`, `wire-diff`, host check 0 errors, markup 508/0 and specmap
  6814 units / 2218 tagged items / 1987 edges with zero suspects, gated
  orphans or unresolved host edges. The wire carries canonical-decimal byte
  counts beyond u32, exact measured-witness attribution, portable artifact
  paths, a four-state base source layer and optional relation enrichment; it
  ships no source body, heuristic verdict, new evidence command or runtime
  provider.
- R7.5 P2/A1 facts evidence: `1dac7531` moves the package authored-source
  scanner, canonical address-prefix builder and four-state per-address
  adoption join into dependency-clean `vibe-facts`; `65b0cba6` indexes both
  new code edges. Root passed 16 unit + 8 doctest and the existing 6/6
  `cli_facts` integrations, strict two-crate clippy, conform 27 known / 0 new,
  fmt and specmap 6814/2220/1989 with zero suspects/orphans/unresolved. The
  scanner uses the shared MD/XML projection and pair-collision law, excludes
  generated boot lanes, supports in-place slots without slot-record or
  `vibe-workspace` coupling and ships no source body.
- R7.5 P2/A2-A3 evidence: one-read raw/projection/registry witnesses
  `5fb4246d` / map `ba9a38f5`; shared query/provider and exact three-digest
  recipes `6a300cf8` / map `0d1fa300`; lock content authority
  `a1095d8e` / map `5b7231ee`; current/carried adapter `ea031767` / map
  `c33ddda2`, with normative trust refinements `500f7b62`, `8ff9fdf7` /
  `8a05d344`. Root gates passed 37 query lib + 3 doctests, 57 trace lib + 8
  doctests, all 449 workspace tests, strict three-crate clippy, conform 27
  known / 0 new and workspace all-target check. The current map is built once
  under an exact namespace; carried bytes require lock hash → slot source hash
  → owned map row → capability no-follow/single-link one-byte hash+parse.
  Today's installed slots carry no map and therefore report honest
  unavailable; `vibe specmap` is the existing producer for packages that ship
  one.
- R7.5 P2/A4a-A4b input-measurement evidence: A4a `c3e51139` / `172c854d`
  replaced the historical walk-per-pattern implementation with one union walk
  and exact pattern-major replay. A4b `5b01d71c` freezes and implements the
  separate effective-declaration fingerprint, no-follow/single-link proof,
  two identical bounded reads per accepted file, legacy raw fallback on
  evidence-only refusal, total manifest refusal and current-run state carriage
  on ordinary success/fresh/hosted satisfaction. The temporary A4a phrase
  “one physical raw read” is retired: the lasting law is one enumeration/one
  logical union row, two detection reads, zero raw fallback on a clean tree.
  Root passed 10 declaration, 11 physical-observation, 7 carriage and all 10
  prior input REDs; full lifecycle is 255 passed / 3 ignored plus 15 doctests,
  strict clippy, workspace all-target check and conform 0 new. Combined map
  after A4b/A4c0 is 6819 units / 2241 tagged items / 2010 edges with zero
  suspects, gated orphans or unresolved host edges.
- R7.5 P2/A4c0 artifact substrate: normative streaming recipe `3b4b7b59` and
  safefs primitive `d24de1ff`. A regular single-link file is SHA-256 streamed
  twice through one held no-follow handle with equal byte counts/digests and a
  final-name proof; no content-sized allocation or byte cap exists. Bounded
  direct-child enumeration supports the deterministic empty-directory-aware
  tree walk A4c1 will add. Root passed all 97 safefs tests + 10 doctests,
  strict all-feature clippy, dependent/workspace check and conform 0 new.
  The A4c design also exposed B3: agent output probes must compare only
  `(id,kind,path)` after witnesses become additive, or completed hosted rows
  re-park forever; A4c1 owns that repair with artifact carriage.
- R7.5 P2/A4c1 artifact evidence: `2cabc7a7` lands exact file-v1 and
  empty-directory-aware streaming tree-v1 witnesses, per-artifact refusal,
  ordinary/hosted production baselines, and B3's identity-only hosted probe.
  Fresh preserves the durable producing baseline while its current re-probe
  lives in an invocation-local typed map for A5; the first implementation that
  overwrote W1 by W2 was rejected despite green tests because it made external
  mutation match itself. Corrected E5 proves W1 remains durable, W2/refusal is
  current, legacy absence is not upgraded and a witnessed hosted row converges
  on invocation three. Root passed 292 lifecycle tests / 3 ignored + 15
  doctests, strict clippy, workspace check and conform 0 new. A4 measurement
  is complete; A5 consumes the transient/current plus durable/baseline halves.
- R7.5 P2/A5 verification reconciliation: lifecycle writer `594734a3` builds
  the one generated member from the completed current declaration/input prefix
  plus every invocation-accumulated artifact (including slot-stage outputs),
  re-observes at verify, validates the wire and pins the path-qualified
  evidence-id schedule with an independent longhand golden. Orchestrator funnel
  `63c35c85` arms only the complete phase epoch, fires with zero verify rows,
  stops stale/missing/unstable before the suffix, and preserves the exact member
  through success, stale stop, later-handler and generic failure into both CLI
  and MCP; `38a5c8f1` freezes the runtime laws. Root passed lifecycle 305/3 +
  15 doctests, orchestrator 9+5 focused, CLI 3 e2e + 5 goldens + 4 projection,
  MCP 3 e2e + 1 projection, strict touched-crate clippy, workspace all-target
  check, conform 0 new and facts 508/0. Specmap `97919def` is
  6825/2266/2037 at zero suspects/orphans/unresolved. P2 is complete; P3 owns
  only thin requirements surfaces and the fake external PDSA reference test.
- R7.5 P3 surfaces/e2e: `1ff0ad64` exposes thin CLI `vibe requirements` and
  MCP `requirements_query` adapters over the one shared library; `e9301051`
  proves hosted park/resume remeasures invalidated predecessors while an
  uninterrupted stale comparison needs the fake external process's second
  invocation, with no provider call, PDSA product vocabulary or engine
  back-edge. `fdb1c465` teaches the installed MCP skill the new query and
  `46f9321b` records its 14 new code/spec edges. The boundary panel paid three
  stale-ratchet debts rather than hiding them: local decoder classification
  `5eeb4283`, the exact 20-scalar/13-structure optional-shape count
  `8b871fb1`, and authored-to-vendor conform cache synchronisation `b7515063`.
  The final exact-tree run executed all 54 dynamic gates through
  `self-check: all green`: workspace 978s, both user-home tripwires 184s/177s,
  sync-engines 51/51, generated wire/specmap/wire-diff clean, markup 508/0 and
  every package test/clippy/conform/self-trace green. R7.5 is complete.
- R4.0 pure registry extraction: `6af1b86f` moves the one declaration,
  activation, ordering, selector and view collector into
  `vibe-extension-registry`; lifecycle keeps type-identical public re-exports
  while `EffectiveManifestKind` and the execution-shaped plan remain above the
  kernel. Runtime dependencies are exactly `glob`, `specmark`, `thiserror`,
  `vibe-core`; dev-only AST/dependency fences also reject grouped/renamed/glob
  ambient `std` access and any higher-crate edge. Root caught and repaired two
  worker proof gaps (private-module rather than public-root identity; raw-text
  rather than syntax-complete ambient scan), then independently passed kernel
  22, lifecycle 287/3 ignored, orchestrator 126 plus doctests, strict clippy,
  install/CLI check, conform 0 new and the cargo-metadata DAG. Map `8531cf82`
  relocates the exact semantic edge multiset plus three intentional module
  scopes: 6826/2284/2054, zero suspects/orphans/unresolved. R4.0 is complete.
- R4.1 owner controls: `52a59dcc` retains every package manifest's parsed
  controls and adds the pure dependency-seat→lane-owner projection; selected
  host behavior stays unchanged because installed controls are inert until
  their package owns the view. Five REDs prove exact projection, host/package/
  sibling isolation, live activation/disable in the package's own lane and
  loud self-target refusal. Root replaced a vacuous worker filter (`0 tests`)
  with the full vibe-install suite and accepted kernel 26, lifecycle 287/3
  ignored, orchestrator world 17, full install + doctests, strict clippy and
  conform 0 new.
- R4.1 unit publication: `91142777` compiles the full per-unit INDEX/STATIC
  before mutation and publishes selected/stale state through the existing
  crash-recoverable transaction; `ab68d145` makes the law authoritative in
  PROP-054. A true RED recreates the old half-published INDEX on backend
  refusal; success/fresh bytes+mtimes, manager binding and the existing fault/
  rollback suite stay green. Root accepted workspace 452, strict clippy,
  dependent check and conform 0 new. Combined map `3883f15e` is
  6828/2293/2063 at zero suspects/orphans/unresolved.
- R4.1 TransformPlan substrate: T1 `b65f9958` promotes the byte-identical
  compiler digest primitive and lands the lossless semantic TOML config tree,
  checked datetime and canonical config digest; T3 `48d7dc75` lands the typed
  dependency/host selector subject (including host+path), public compiled
  selector value, canonical OR-set equality and one subject-less enabled-row
  view without adding a temporary plan allocation. Root rejected the first
  worker pass for a five-digit TOML year, unspellable host+path subject,
  double-collection plan and equality/digest disagreement; two same-cwd
  claudez corrections fixed them and conform split the 638-line test cell.
  The independent Opus/max T2 audit then exposed the still-ambiguous child
  digest/provider/optional/epoch frames; central freeze `d5fcd92d` closes the
  exact bytes, private epoch authority and refusal precedence before code.
  Root gates: vibe-spec 664, registry 33 + doctest, lifecycle 287/3 ignored,
  strict three-crate clippy, downstream workspace/install/orchestrator/CLI
  check and conform 0 new. Map `5f6bca62` adds exactly 17 semantic edges and is
  6830/2307/2080 at zero suspects/orphans/unresolved. T2 implementation is next.
- R4.1 T2 plan identity: `49e944f0` lands the opaque typed plan/provider/
  implementation/config family, dense builder, exact implementation/plan
  frames, selector/config presence, bounded typed refusals and syntax-aware
  dependency/ambient/opacity fences; `87ef2df6` exposes the shared borrowed
  ContentHash grammar so hostile revalidation clones no parser error. Root
  rejected three proof rounds: public enum fields leaked future epoch
  authority, the first fence repeated the known raw-text failure, worker gate
  pipelines masked Cargo exits, then the selector-count mutation hit entry
  count and the hex assertion was vacuous. Same-cwd corrections made the
  values opaque, introduced dev-only syn/toml structural gates and fixed the
  independent vectors. Root reran core 7, vibe-spec 685 + 5/2/7/4, strict
  core/spec clippy, workspace/lifecycle/install/orchestrator/CLI check, conform
  0 new and three live RED mutations (post-dedup count, `std::path` alias,
  epoch-constructor visibility). Map `b768bcb8` is 6831/2318/2091 at zero
  suspects/orphans/unresolved. T4 carriage is next.
- R4.1 T4 carriage: `a252fcc8` moves the 593-line ArtifactPlan cell into a
  462-line parent plus 176-line focused plan module, adds private whole-plan
  carriage, pins all four legacy constructors to empty and forwards the plan
  through opaque backend retargeting. Nonempty carriage is deliberately inert:
  exact nine-item schedule, emitted bytes/provenance/fingerprint, typed failure
  and no-header behavior equal the empty plan. Root accepted vibe-spec 691 +
  5/2/7/4, workspace 452 + 5/7/27/1, strict clippy/downstream/conform and a
  live retarget-drop mutation. Map `5aa44611` is 6831/2324/2097 at zero
  suspects/orphans/unresolved. T5 behavior registry is next; central ruling
  keeps its four identity behaviors cfg-test-only rather than reserving public
  no-op builtin names.
- R4.1 T5 registry: `0eb46c82` lands one private four-method TransformBehavior,
  deterministic name/epoch/stage registry and bounded collision/unknown/epoch/
  stage refusals without adding any pass or shipping builtin. Production
  catalog is empty; one cfg-test support module owns the exact four
  `test-identity-*` epoch-1 vehicles for T5 and future T6. Four real IR vectors,
  wrong-stage matrix, pointer identity, empty-catalog/drop, hostile name and
  syntax-aware ownership/collector/public-surface fences are green. Root
  rejected a private test-catalog copy and a fence that blessed Box/failed to
  see path aliases, then accepted vibe-spec 708 + 5/2/7/4, strict clippy/
  downstream/conform and live epoch-check/Box-fence mutations. Map `0f73cdfe`
  is 6831/2335/2108 at zero suspects/orphans/unresolved. T6 is split into T6a
  fallible discovery, T6b identity positions and T6c lane witness.
- R4.1 T6a fallible discovery: `01f1522e` makes the one worklist discovery
  seam return the caller's exact generic error through simple input, `#use`,
  `#source` expansion and `#embed` recursion. The three existing parse callers
  remain infallible through one private exhaustive `Infallible` eliminator;
  source lookup failures retain their separate recorded-observation semantics.
  Root rejected a vacuous failure-recorder assertion and an unproved syntax
  classifier, accepted the claudez correction, then reran 721 + 5/2/7/4
  vibe-spec tests, strict clippy/downstream/conform and live swallow/unwrap
  mutations. Map `ce3e62bf` is 6831/2344/2117 at zero suspects/orphans/
  unresolved. T6b identity positions is next.
- R4.1 T6b identity positions: `6ffedb03` resolves a whole carried plan
  against one injected registry before anything executes, then runs it at
  source-before-parse, document-after-parse, lane-after-assemble and
  emitted-after-emit. A nonempty compatibility-fragment plan refuses before
  any lookup; a selector-bearing source/document entry, a changed lane and
  changed emitted bytes refuse as typed capability gaps owed to T7/T8, T6c
  and T9, while byte-equal emitted output returns the original artifact
  untouched. One shared transform-first classifier keeps document, prefix
  and emitted faults typed under the single new public variant without
  publishing a taxonomy, and T6a's tripwire fired as designed: the
  `Infallible` eliminator is gone and all three discovery callers propagate
  the real error. T4's inert-nonempty claim is retired honestly — the empty
  half stays byte/error/schedule exact, while the same nonempty plan now
  refuses under the empty production catalog and executes under the injected
  one, so parity is caused by execution rather than by inertness. Root read
  the complete dirty diff, deduplicated the twice-defined fault conversion,
  documented the entry-count cast against `checked_entry_count`, then
  reproduced 747 + 5/2/7/4 vibe-spec tests, a post-`clean` strict clippy,
  downstream checks, conform at 0 new, diff hygiene and six independently
  authored mutations — source-chain removal, schedule metadata drop, emitted
  classifier bypass, omitted lane behavior call, equal-byte provenance
  rebuild and a disabled frame check — each RED on exactly its focused test
  and reverted byte-exact against a pre-mutation hash. Map `1d2e3f2a` is
  6831/2368/2141 at zero suspects/orphans/unresolved. T6c lane witness is
  next.
- R4.1 T6c lane witness: `cb6006d4` opens the lane position. The evidence was
  missing rather than wrong — `VerificationWitness::Lane` was a unit variant
  carrying nothing and the transition law ended in a catch-all, so a lane
  transition was unchecked by construction. It now carries the lane's
  provenance copied from the pass INPUT, because evidence taken after the
  behavior ran only ever agrees with itself, and the manager-side gate runs the
  intrinsic contract then the provenance transition UNCONDITIONALLY in the
  wrapper rather than through the `#[cfg(test)]` inter-pass verifier hook,
  which would have passed every test while shipping production unguarded. A
  lane transform owns `contributions` and nothing else; `frame.renames` is the
  sharpest immutable member because it flows onward into
  `EmissionProvenance.renames`. T6b's full-equality detector and its
  `LaneChange` gap are retired, and the reordering vehicle they left behind
  became the real intrinsic fixture rather than being deleted. Root read the
  complete diff, then reproduced 757 + 5/2/7/4 tests, a post-`clean` strict
  clippy, downstream, conform 0-new, diff hygiene and SEVEN mutations — the
  five the packet mandated plus two of its own: deleting one provenance
  comparison reddens exactly that field's test and no other, and neutering the
  inter-pass verifier's lane arm reddens exactly one test, proving the two
  guard paths are independently covered instead of sharing a single proof.
  Removing the gate reproduced the whole argument for the atom: a structurally
  broken lane escapes to the backend as an untyped `Backend` error, and a
  forged `OriginRename` reaches the emission provenance and the emitted bytes.
  The witness boundary and its rejected alternatives are recorded at ABI §6.4
  (`4f2acb42`) instead of surviving only in a review note. Map `0191a2b3` is
  6831/2379/2152 at zero suspects/orphans/unresolved. T7 DocumentSubject
  carrier is next.
- Gate repair `67ab9683` fixes the conform content store's Windows cache slot:
  `sha256:<hex>` had created 1,393 NTFS alternate streams on one base file and
  failed with OS error 665. Authored engine + six vendored copies now use one
  ordinary `sha256-<hex>.json` file per entry; the exact RED, 51-pair
  sync-engine check and a cold 1,404-file conform rebuild pass. The temporary
  package target was immediately cleaned (1.7 GiB reclaimed).
- B-107 is closed by `f8f197cd`/`c195eae1`: all 502 judged records map
  one-to-one to live paths (98 retain their extension; 404 become XML), six new
  unjudged live documents bring the corpus to 508, and all 19,548 verdicts plus
  42,318 evidence lines retain their payload/timestamps. Stability proves 75
  files / 2,280 verdicts sealable, 191 files / 859 moved facts requiring
  re-judgement, and 236 refused files / 8,634 verdicts; a further 122 moved facts
  live inside 22 refused files and are reported separately, never double-counted.
  Judging debt was remeasured on final R3.4 `main` after integration and GC:
  **2,501 unjudged facts in 154 files, 0 orphaned, 484 stale files** — unchanged
  from the pre-harvest checkpoint. One historical gap is named separately from
  1,082 comparable moved facts; no sealing or re-judgement occurred.
- Historical baseline panel: green on 2026-08-27 at pre-R3.4 checkpoint
  `4a51a169`. Superseded as current evidence by the final all-green R3.4 panel
  at `3656a889` above.
- Host materialisation is proved: a fresh `vibe reinstall --force` migrated all
  37 tracked slots to strict ownership records with 1,354 independently
  rehashed rows; subsequent install materialised 0/37 and the boot tree was
  byte-identical (`BOOT_BYTE_NOOP=True`).
- Publication: `cargo xtask mirror` fanned the complete R3.4 checkpoint
  `2bb53f95` plus tags to GitVerse and GitHub successfully on 2026-08-28.

## 3. Granular R1–R8 ledger

Legend: `done` = landed and proven; `partial` = useful substrate only;
`missing` = no production implementation; `future` = deliberately not built in
this campaign, but compatibility law is preserved.

### R1 — diff materialisation

| Step | State | Landed evidence |
|---|---|---|
| R1.1 strict JTD slot record | done | `6d606ef2`; `vibedeps/slot_record.rs`; slot-record corpus/tests |
| R1.2 record-owned diff, no wipe | done | `1cf4f189`, `45f3a8c3`, `b90cd209`; shared wire-path ordering; 37 host records / 1,354 SHA-valid rows; unrecorded/mtime/hardlink reds |
| R1.3 mutable-source hash gate | done | `6a7f750d`; mutable install unit + CLI e2e |
| R1.4 verify-heal and hook only on nonempty diff (owner 3.A) | done | `4503fdb6`, `9c545f0d`; `cli_hook_rerun.rs` |
| R1.5 amendment draft | done as draft | `c7438ff0` and `SPEC-DEBT-LIFECYCLE.md` §§1–7; authoritative movement pending |

### R2 — lifecycle engine

| Step | State | Landed evidence |
|---|---|---|
| R2.1 strict `[[extension]]` grammar | done | `b580bd1d`; manifest wire/semantic suites |
| R2.2 nine-phase line, phase verbs, clean chain | done | `8d91ccf3`, `26cc4472`; `cli_lifecycle.rs` |
| R2.3 two-epoch collection/order/controls/selectors | done | `85f5eb56`, `767fb4da`, `f2e97fff`; registry + pre-barrier tests |
| R2.4 envelope and builtin `log` | done | `910e022c`; lifecycle wire + dispatch tests |
| R2.5 durable freshness and `--force` | done | `07941230`; state and update-world tests |
| R2.6 script/binary handlers and hook sugar | done | `a8aa5d4a`; script/binary/failure/hook suites |
| R2.7 data presets and `vibe extensions` | done | `21ff9da7`, `516b49b7`; preset/query/JTD corpus tests |
| R2.8 owner scenario §10.1 | done | `3137990c`; `cli_lifecycle_commissioning.rs` |

### R3 — explicit compiler IR and manager

| Step | State | Landed evidence / exact absence |
|---|---|---|
| R3.1 five levels, six carriers, typed pass manager | done | `3630ab9e`; `compiler/{ir,pass,pipeline}.rs` |
| R3.2 parse→close→merge→embed→qualify→absorb→link→assemble→emit | done | `a7961003`, `96eef07d`, `e53b9a4e`, `ec7ea7fe`, `e653654d`, `2feef271`, `6de7ef05`, `6f3fa61a`, `302a3509`, `4403cb55`; 84 boot-artifact + 10 emit gates |
| R3.3 verifier-each skeleton | done | `15793f2e`; immutable test-only manager verifier, typed level/transition errors, SCC/document/lane/marker/fence invariants; 61 focused verifier tests + independent freeze |
| R3.4 compile snapshots/timings | done | `6f4a717d` metadata/index/filename; `7adfbb5a` activation; `fa0662a9` observer; `4d95a129` writer/retention/budget; `34d3f363` shared report; `0301f8f2` sticky/displaced identity; `be04a184` borrowed compilation; `cad8ecc1` owner funnel; `dcbf89b0` install/lifecycle; main owner/parity chain `3091df74`…`f59e26c3`; discipline/RED/specmap tail `6a4a31dc`…`3656a889`; final all-green panel + GC |

R3.2 also landed a crash-safe whole-artifact transaction/selector tier. That
unplanned safety work is accepted substrate, not a substitute for R3.3/R3.4.

### R4 — tier-1 staged transforms

| Step | State | Evidence |
|---|---|---|
| R4.0 one pure registry below lifecycle/workspace | done | kernel `6af1b86f`, map `8531cf82`; exact runtime-dependency/AST-ambient/public-reexport fences; kernel 22, lifecycle 287/3 ignored, orchestrator 126 + doctests, strict clippy/check/conform/DAG green |
| R4.1 four positions, owner-scoped activation, header, per-unit fingerprint, reference oracle | in progress | controls `52a59dcc`; transaction `91142777` / `ab68d145`; T1 `b65f9958`; T3 `48d7dc75`; T2 `49e944f0` + `87ef2df6`; T4 `a252fcc8`; T5 `0eb46c82`; T6a `01f1522e`; T6b `6ffedb03`; T6c `cb6006d4`; map `0191a2b3` = 6831/2379/2152; T7–T10 subject wire/world/header/fingerprint open |
| R4.2 builtin XML minify | partial | pure strict kernel `016f0fab` and reversible comment codec `fbbd5140`; no activation/on-off e2e |
| R4.3 lane analyzer | missing | no `vibe extensions analyze` or machine report |

### R5 — native tier

| Step | State | Required result |
|---|---|---|
| R5.1 native JTD context/reply/manifest + `vibe-ext` macro | missing | schema first; panic-abort compile refusal; unwind catch boundary |
| R5.2 loader | missing | separate unsafe-quarantine crate, libloading/cache/free-once tests |
| R5.3 source/prebuilt resolution and in-slot build | partial substrate | build ignores exist; no native artifact/provider build path |
| R5.4 pending bootstrap convergence | missing | install may mark pending; build rebuilds and recompiles once |
| R5.5 native/builtin minify parity | missing | owner scenario §10.2 |

### R6 — full compiler pass tier

| Step | State | Evidence / gap |
|---|---|---|
| R6.1 `compiler_internals` + executable pass grammar | partial | conspicuous flag/raw table land; kind-specific required/forbidden fields intentionally deferred |
| R6.2a whole-IR wire epoch | done | `c26cd039`; six strict carriers, generated types, derived corpus, 42 conversion-gate/producer-oracle tests and independent final freeze |
| R6.2b strict domain conversion | done | `17afb5b6`; lossless `AnyIr` projection, all fifteen ordered gates, production replay at 12–14, emitted identity/framing at 15, owned custom targets, bounded hostile diagnostics and independent final freeze |
| R6.3 before/after/replace, frontend/backend | missing | compiler never consumes manifest pass rows |
| R6.4 mandatory verifier after plugin passes | missing | depends on R3.3/R5 |
| R6.5 `.txt` frontend + JSON lane backend e2e | missing | no custom format registry/backend artifact surface |

### R7 — provider, create and hosted-agent tier

| Step | State | Evidence / gap |
|---|---|---|
| R7.1 real provider seam | done | `f42334ff`, `e2392893`; JTD wire, config, endpoint/redirect/proxy/body/timeout/redaction tests |
| R7.2 CLI agent handler + output contract | done | `26929050`; strict AgentResult JTD, prepared prompt/world resolution, ResultPlan, optional provider path, create/install/reinstall/update e2e and shared safe filesystem cell |
| R7.3 hosted outbox/delegated resume | done | `1dd5e1f5`, generated reports `eae4494e`; durable run/outbox, exact task ownership, candidate-state atomicity, phase/slot reconciliation, command-level progress, no-spend sequential resume and independent final freeze |
| R7.4 MCP lifecycle surfaces | done | architecture `ee2bc67f`; first wave `94f30aa9`, `88600508`, `87c2bab8`, `daf6eb31`, `17d94f8f`; A4 `31ca1e7d` / `0225ce41` / `970520d4`; A5 `7e330974` / `93177db6`; A6 `7bd335e2` / `b3ff308c`, `e1121d9b`, `b4d91749`, `8debdf2e`, `c82012c3`; A7–A8 tasks cut `cf5ec17d`, `ee741f2e` / `d338e880`, `30534ff9` / `7b9732f0`, `df678d7b` / `c62177fe`; A0–A8 panel `a4253de7` all green; A9 ports `5506cf88` / `9732ba38` / `18a797b2`; A10 projection `53e84790`; A11 plan `2560ee57` / `78ea8cc5`; A12 application `053b7e37` / `ba874cdf`; A13 trace `3f01e2dc` / `afdd3adc`; A14 selected-world prompt resolver `cd793ca9` / `2b08c818`; A15a package source `5df76260` / `23044cfd`, selected-member repair `da2ff985`, B-109 `b615ebe5`; A15b two-stage lease-first default command `0ef2f8f5` / `a4336ea2`, scoped clean debt B-110 `fc279fbe`; A15c hosted backend `1027ca5e`, strict MCP run/parity `9c340df8`, map `a0276a0d`, selected-resolver debt B-111 `65b145f0`; full-panel ratchet decision `602ef5e8`, Windows mandatory-lock repair `9fc6c2bb` / map `6f074e66`. Final panel on exact tree `6f074e66` ran 54 dynamic gates: workspace tests + clippy, host check 0 errors, conform 27 known / 0 new, clean codegen/specmap/wire, all package/MCP suites, both user-home tripwires and markup 508/0; ordered tail `self-check: all green` |
| R7.5 external orchestration substrate | done | P0 `41abb4db`…`e8e67280`; P1 `6d843467`, `d3a9d59b`, `55937044`; P2/A1 `1dac7531` / `65b0cba6`; A2 one-read/query/authority `5fb4246d`, `6a300cf8`, `a1095d8e` with maps `ba9a38f5`, `0d1fa300`, `5b7231ee`; A3 adapter `ea031767` / maps through `8a05d344`; A4a–A4c1 through `2cabc7a7`; A5 writer/funnel `594734a3` / `63c35c85`, laws `38a5c8f1`, map `97919def`; P3 surfaces `1ff0ad64`, fake external PDSA `e9301051`, skill projection `fdb1c465`, map `46f9321b`; boundary repairs `5eeb4283`, `8b871fb1`, `b7515063`. Final 54-gate panel all green; specmap 6826/2281/2051 at zero suspects/orphans/unresolved. No byte cap, whole-tree hashing, fingerprint relabelling, prose, `unmet`, duplicate evidence command, coding agent, heuristic or automatic loop |

R7 live Z.AI smoke is now conclusive (2026-08-27): central `vibe create`
called the official OpenAI-compatible coding endpoint with `glm-5-turbo`, read
the claudez credential only through its token-file path, exited 0 and produced
the exact declared `LIVE_OK\n` output. No token or provider response body was
printed. Mock/security gates remain the deterministic proof; the algorithmic
system remains fully usable with no selected agent contribution or provider.

### R8 — package, build and deploy substance

| Atom | State | Evidence / gap |
|---|---|---|
| R8.1 project package-skill binding | done | `c0fa49be`, `9275f373`, `67886ea7`; strict JTD receipt, intent/recovery/CAS, exact ownership, Unicode-9 physical aliases, lossless paths, Claude/Codex/OpenCode project projections |
| R8.2a mechanism/artifact/deploy grammar | done | `2a3f3b44`; typed package/host provider pins, literal-vs-pattern path law, artifact/deploy DAGs and profiles, parse/write symmetry |
| artifact records and target DAG runtime | partial | strict `[[artifacts.build]]`/`[[artifacts.package]]` grammar/DAG in `2a3f3b44`; no persisted `ArtifactRecord`, freshness or executor registry |
| `[[mechanism]]` provider runtime and host routing | partial | strict declarations/routes/pins in `2a3f3b44`; no installed-world selection or plan/apply/verify dispatch yet |
| Cargo commissioning build provider | missing | no metadata/compiler-artifact JSON selection through lifecycle |
| fully static one-file skill | missing | no include-consumption/static safety builder |
| Agent Plugins 1.0 directory | missing | no plugin schema/package provider |
| Claude/Codex/OpenCode client projections and local deploy | partial only | project skill writer is not a portable plugin artifact or user deployment |
| deploy targets/profiles/plan/undeploy | missing | draft grammar/rulings only |
| intent/receipt/recovery for general destinations | partial precedent | package-skill receipt is safe but not the general deploy protocol |
| `deploy:vibe-bin` under `~/.vibe/bin` | missing | existing `vibe bin` is a different project-pinned launcher genre |
| deterministic Windows zip lifecycle binding | missing | pre-campaign scripts/archive recipe exist outside lifecycle |
| plugin-overridable builder/installer/deployer fixture | missing | no mechanism selection or replacement e2e |

## 4. Owner additions — preservation and implementation status

All additions are durably captured by `518be400` in
`BUILD-PACKAGE-DEPLOY-ARCHITECTURE-v0.1.md` and the R7/R8 spec-debt
continuation. They are not silently reduced to the old three-line R8 minimum.

| Owner requirement | Durable law | Implementation |
|---|---|---|
| LLM is optional paid enhancement | algorithmic baseline; `off/assist/required`; lazy provider; per-feature/run budgets | missing outside provider seam |
| Static skill artifact | exactly one validated `SKILL.md`; explicit include consumption | missing |
| Agent Plugin | 1.0 directory, portable skill/MCP subset, client projections distinct | missing |
| Classic Cargo/meta-build | provider protocol, Cargo metadata + JSON artifact messages, no autodetect | missing |
| Claude/Codex/OpenCode local install | versioned adapters; CLI/public filesystem contracts; ownership receipts | project projection only |
| VibeVM tools in `~/.vibe/bin` | immutable store + version-free launcher; separate from `vibe bin` | missing |
| Deploy profiles | named target selections; local/remote; explicit default; plan and inverse | missing |
| User-overridable mechanisms | exact pin → host route → builtin default → failure | missing |
| Download/system package managers/VibeVM OS horizon | qualified identities; desired/artifact/deployment separate; effect class + receipt | future by design; compatibility constraints active now |

## 5. Architecture decisions in force

1. One nine-phase line: dependency materialisation is `install`; user/remote/
   system placement is `deploy`; `package` mutates no destination.
2. One extension/mechanism plane. Scheduled contributions answer _when_;
   sibling mechanism providers answer _how_. Builtins are ordinary qualified
   providers and may be deliberately replaced by host routing.
3. Source/document calls are per addressed document. Documents gather once;
   closure/lane/emitted are per complete final artifact. Compatibility wrappers
   are never production whole-artifact drivers.
4. The R3 verifier is immutable, manager-owned and test-only now; R6 enables
   the same seam unconditionally. DuplicateId semantics are reused, not
   strengthened accidentally. Use/Source cycles are checked separately with
   the existing contract-only exception; Embed is strictly acyclic; the union
   is not one recursion graph.
5. Pull the R6.2 JTD compiler-IR projection before R3.4. It has six tagged
   carriers (`source-document`, `document-document`, `documents-artifact`,
   `closure-artifact`, `lane-artifact`, `emitted-artifact`) and is also the
   trace snapshot document. No handwritten trace DTO or serde on domain IR.
6. Compiler registry collection is extracted to a lower pure crate and reused
   by lifecycle and workspace; no dependency cycle and no second collector.
7. R4 transforms run after the untransformed reference oracle. Empty transform
   plans preserve exact historical bytes; active ordered plan/config/provider
   identity enters fingerprints and an honest artifact header.
8. Native wire is C+JSON, JTD-first. `vibe-ext` is safe author SDK; a separate
   host crate quarantines libloading/unsafe. `panic=abort` extension builds
   compile-refuse.
9. R7 exact provider id is `openai-compatible`; project provider/model merge
   per field over user config; nonempty project env credential beats user token
   file; absolute operator token paths are legal; `~` is literal. Keyed HTTPS,
   keyless loopback HTTP only, redirects off, no body/secret diagnostics.
10. Hosted resume extends existing lifecycle state with one durable run id and
    exact state-owned outbox task. A candidate `LifecycleState` reconciles
    slot debt and its ordered target continuation, validates and lands
    atomically before replacing memory; cancellation forgets state before exact
    task cleanup. Phase and slot parks carry typed scopes and reconcile only
    against their own current plans. Install/update/reinstall emit one hosted
    command root with boundary-measured progress; sequential agent rows reuse
    only engine-recorded exact-fingerprint outputs and never call the provider.
    MCP is a second adapter over these files, not a second mailbox.
11. Automatic package skill binding is project-only. User/client installation
    is explicit deploy, never an implicit side effect of package.
12. Every external mutation has plan, effect class, intent, independent verify
    and receipt. A third observed digest refuses; inverse removes only verified
    owned state. Future OS resources reuse this law.
13. R3.4 observes the existing pass manager; it does not create a trace-only IR.
    One lifecycle/install `run_id` owns one recorder across every dirty unit and
    workspace-node artifact. Each successful invocation serialises the accepted
    R6 compiler-IR carrier, repeated document calls receive distinct sequence
    numbers, failed/verifier-rejected calls retain timing without certifying a
    snapshot, and pass/artifact names use the exact reversible Windows-safe
    full/short filename contract landed in `6f4a717d`. Trace-side encode/write
    failure is recorded as `snapshot-failed` but never changes the compiler's
    verdict; a later boot-transaction failure may still make the root run
    `failed` after every pass succeeded. The no-trace path keeps the existing
    public wrappers and bytes. Cooperating trace writers serialize under one
    project lock; retention revalidates opaque entry identity immediately
    before removal and makes no impossible claim about an uncooperative process
    racing the final portable unlink. `PossiblyPublished` index bytes are
    re-read exactly; snapshot residues are conservatively charged. Concurrent
    scopes admit one soft-ceiling crossing: an already-encoded loser is
    `snapshot-failed` with no file, and every later decision stands down before
    encode.
    Detailed implementation boundary:
    [`COMPILER-IR-TRACE-ARCHITECTURE-v0.1.md`](COMPILER-IR-TRACE-ARCHITECTURE-v0.1.md).
14. R6.3 keeps one `compiler_ir/e1` JTD and one domain projection. Before a
    foreign native invocation ships, request and reply channel roles must be
    represented honestly: plugins are foreign readers of requests while the
    host remains the strict reader of replies. This may not be solved by
    duplicating the IR DTO/schema or by silently making the host permissive.
15. R4.0 is a new pure registry-kernel crate below `vibe-spec`,
    `vibe-workspace` and `vibe-lifecycle` (`kernel -> vibe-core`; all three
    higher crates consume it). Manifest grammar stays in `vibe-core`, execution
    stays in lifecycle, and behavior-bearing pass/backend registries stay in
    `vibe-spec`. The kernel owns provider/world rows, ordering, controls,
    selectors and views. R4.1 adds one `vibe-workspace` ordered-world adapter:
    root-lock iteration, substitution in place and resolver-order appends;
    `read_dir` may report orphans but never orders or populates the extension
    world. One filesystem snapshot feeds owner-scoped views: the node manifest
    activates its node lane, and each package manifest activates that package's
    own unit lane; dependency controls are retained but inert in other owners.
    Detailed extraction boundary:
    [`R4-REGISTRY-KERNEL-ARCHITECTURE-v0.1.md`](R4-REGISTRY-KERNEL-ARCHITECTURE-v0.1.md).
16. R4's untransformed emitter remains the reference oracle. An emitted
    transform therefore needs a manager-owned constructor that recomputes
    provenance/digests after the oracle; mutating `EmittedArtifact.bytes` in
    place is forbidden. An active-plan-only transforms header must be accepted
    by the emitted-tape validators. Per-unit lanes hash their own owner-plan
    digest into the boot-graph fingerprint (empty plan = absent frame); node
    lanes intentionally have no freshness fingerprint and always recompute,
    carrying plan identity in header/provenance while equal transactional bytes
    preserve mtime. Per-unit lanes joined the crash-safe artifact transaction
    at `91142777` / `ab68d145`; every byte-changing transform must ride that
    same manager-owned publication. XML-minify needs explicit REDs for
    hoisted top-level `#use`, all-elided streams, comment/CDATA boundaries,
    semantic `from_xml` parity and a strictly-smaller real lane; it may never
    bless non-XML text by weakening the strict kernel. Document selector
    subjects must be carried from typed provider/path identity, not recovered
    by parsing display strings. Compatibility fragments do not run tier-1
    transforms.
17. R5 uses three JTD-first native roots (context, reply, manifest) while
    sharing lifecycle subshapes through the vocabulary/generated shared-module
    mechanism, never copied DTOs. Point stays an open string on the wire and is
    parsed by the host's closed `ExtensionPoint` vocabulary. Invoke request
    bytes are host-borrowed; successful response bytes are plugin-allocated and
    freed exactly once through `vibe_ext_free`; failure returns null/zero.
    `vibe_ext_manifest()` returns a NUL-terminated UTF-8 static pointer valid for
    the loaded library lifetime, copied immediately and never freed. The safe
    SDK compile-refuses `panic=abort`; catch-unwind lives inside the cdylib.
    Phase-native loading can land from the lifecycle envelope, while native
    compiler parity depends on the accepted R6 whole-IR payload/conversion and
    may not invent an interim compile DTO.
18. The compiler wire's open `artifact_target` is already a valid epoch-1
    identity, not a future-only spelling. R6.2b therefore gives
    `ArtifactTarget` an owned, validated custom backend id and round-trips every
    valid carrier without an `UnsupportedCustomTarget` carve-out. This does not
    install or execute a backend: R6.3 separately owns registration, selection
    and invocation. Leaking attacker-controlled strings, interning them in a
    process-global table or consulting the runtime registry during decode are
    forbidden.
19. Project-output path policy has one dependency chain. `vibe-core` owns
    literal component legality (including the ASCII-only Windows device
    vocabulary); `vibe-safefs` owns NFC→Unicode-9 full-fold→NFC physical
    identity, lossless opaque OS-unit comparison and capability/no-follow
    mutation; `vibe-mcp` keeps only receipt vocabulary and exact-spelling
    ownership. Folded identity may refuse a collision or portable rename but
    never authorises an update/removal. Agent outputs, package skills and
    future deploy providers reuse this cell rather than copying it.
20. Lifecycle `compile_trace = true` is a sticky request bit, not proof that a
    recorder opened. A fresh command identity may create its trace run; an
    adopted identity and a state-proven superseded identity use a shared
    `open-existing` path that takes the normal cooperative lock and validates
    the existing directory/index but never creates a missing run or performs a
    fresh retention sweep. Thus a previously unavailable trace cannot become a
    misleading mid-run history on resume, and displacement cannot manufacture
    an empty phantom trace just to mark it superseded. Canonical detail:
    `COMPILER-IR-TRACE-ARCHITECTURE-v0.1.md` §5.3 / acceptance 23–24.
21. A fresh-unit trace observes the safely opened existing output and computes
    its canonical digest before it declares a skipped scope. Successful
    observation then declares+skips before the fresh return; observation
    refusal is only a bounded warning and declares no scope. Publishing first
    and leaving `pending` on read refusal would prevent an otherwise successful
    run from durably finalising `ok` and leak a non-retainable running trace.
22. R3.4 command ownership is a private non-Clone CLI session wrapper plus a
    closed typed `success|parked|failed` exit. One consuming funnel finalises
    or suspends the recorder, drops its last handle, attaches only the owned
    generated trace report, and then renders. Existing inner install/lifecycle
    failure emitters become typed report drafts; trace-disabled schema choice,
    plan ordering and historically silent failures remain exact, while a
    requested trace makes its registered command root observable. Secondary
    trace/report refusal never replaces the original error object. Chained
    clean opens only after the wipe; clean-only stays exempt and validate-only
    may carry a zero-scope trace. Canonical detail: compiler-trace architecture
    §5.3.1.
23. A command failure never serialises the rich `anyhow::Error` into compile
    trace state: script/binary/provider errors can contain captured stderr,
    response bodies or secrets, and a byte cap is not redaction. The trace root
    receives only fixed `command failed`; the same original error object is
    returned unchanged. The writer's existing streaming bound becomes the one
    whole-message clamp for `TraceWarning`/startup/report notices (field caps do
    not cover Display prefixes); CLI copies neither cap nor formatter. Failed
    report emission is the explicit `old-policy OR trace-requested` bit, never
    inferred from whether validation retained the optional member.
24. One command uses one selected-manifest byte snapshot. The raw manifest
    `Result` decides activation without consuming its error; the exact valid
    value is the selected-node override used by workspace discovery. The
    prepared state carries invalid-manifest / loaded / first-discovery-failure
    as distinct variants, so validate-only, install and later world planning
    cannot retry into a different answer. Canonical selected root and user
    config obey the same one-epoch law.
25. Install has explicit pre-apply and post-write workspace epochs. Prepared
    planning, freshness and slot lifecycle use the caller's pre-apply value.
    Apply step 7 returns the Workspace it already rebuilt through the additive
    `PreparedApplyReport`, leaving public `ApplyReport` unchanged; lifecycle
    world collection reuses that value and reads only current lock/slot
    artifacts. Ambient rediscovery between identity, trace, solve and
    contribution collection is forbidden.
26. A satisfied slot continuation does not end the command early. Fresh and
    Ready resumes carry their real lifecycle handle/rows into the authored
    post-durability callback. Ready report order is current-apply rows, resumed
    rows, then phase rows; its ordinary closure-diff/progress/hooks tail runs on
    both paths. A missing Ready callback, reversed merge or one-carrier-only
    merge is mutation-tested.
27. Direct-install failures after durability belong to the Lifecycle report
    family even when world planning fails before dispatch. Slot/resume rows are
    frozen before the first fallible callback operation and prepended once to
    carried handler failures. The trace retains only fixed `command failed`;
    the original typed error, exit code and terminal text remain unchanged.
28. Update and Reinstall are first-class trace owners, not compatibility
    wrappers. Each owns one selected manifest/config/root/workspace epoch and
    one `prepare → execute → finalize → render` funnel across every branch.
    Operational workspace root and selected report identity remain distinct
    typed values; offline posture is resolved once. Reinstall's selected-node
    `project` field intentionally corrects the old member-invocation
    workspace-root spelling; it is the one accepted trace-off wire migration,
    not attributed to enabling trace.
29. A resume failure crosses the shared install substrate as a neutral measured
    carrier: exact original error, progress, resolved count and ordered rows,
    no report family. The outer Install/Lifecycle/Update/Reinstall owner chooses
    its registered root and historical emission bit. When two lifecycles meet,
    order is `current pass → resumed pass`; destructive row ownership transfers
    only after the resumed outcome and handoff are validated.
30. Persisted continuation and compile trace share the exact-identity law. Only
    the lifecycle `run_id` that parked a continuation may service it. A
    displaced run terminalises only an existing state-proven trace, produces
    one bounded structural notice, cancels rather than inherits the old debt
    and never manufactures a missing trace. Adopted missing trace stays
    `unavailable` with no partial history.
31. Report capability governs notice routing. Install/Lifecycle can absorb
    owner notices; Update/Reinstall schemas cannot, so the adapter routes the
    bounded residue exactly once without inventing a member. Normal-force
    Reinstall keeps full internal progress for park/failure/resume but preserves
    its regenerated-only successful trace-off JSON projection.
32. Owner ruling 2026-08-27: lifecycle is an external-agent framework, not a
    built-in coding agent. It supplies phases, contributions, evidence, durable
    handoff and CLI/MCP adapters; Plan/Act policy and PDSA repetition remain in
    an external human/agent orchestrator. Requirements/spec IR may be exposed
    as optional read-only evidence tied to exact tree/run identity, never as
    lifecycle's hidden planner. A reference agent is a separate future
    campaign.
33. R7.5 fact guidance reads status-bearing `vibe-specdoc` facts plus the
    consumer-owned `vibe-facts` overlay; compiler IR fact nodes are not a status
    source. Identity is the full `spec://…#fact` address. Authoring status,
    consumer adoption, specmap edge provenance/provider freshness and lifecycle
    status remain separate typed fields; gap/staleness are typed observations,
    not a heuristic next-task recommendation. Current specmap relations are an
    optional read-only enrichment; their absence changes no lifecycle result.
34. Prompt resolution is one credential-free lower value. The executing
    provider root is self; every other coordinate resolves only through the
    carried lock-selected world. Recursive `#embed` is supported; live
    `#use`/`#source` is reported unsupported after expansion. User config,
    credential reading, transport construction and completion remain in the
    CLI paid half; hosted surfaces reuse the resolver and cannot pay.
35. Package-source construction is one algorithmic composition in
    `vibe-package-source`: the same resolver/cell registry serves CLI and MCP,
    while each surface projects its own grammar and injects its own short-name
    policy. The hosted default admits no package arguments but still resolves
    declared real dependencies through project-local, embedded and declared
    registries; zero-dependency lifecycle runs construct no source at all.
    Provider/model/token configuration has no dependency path into this crate.
    Until conform accepts multiple `(crate, registry-file)` pairs, B-109's
    exact-set/source-fence RED is the explicit interim protection.
36. Workspace state root and selected operation root remain different typed
    facts through a continuation. Reinstall slot resume reads/writes lifecycle
    state under the lease's workspace root, but derives its selected manifest,
    handler host and selected-node agreement from the canonical
    `ReinstallIdentity.selected_project_root`; it never reparses the persisted
    relative spelling and never collapses a member invocation to the root.
37. A default phase command is a two-stage capability. Stage one canonicalises
    the selected node and acquires the workspace lease using only read-only
    locator discovery; the surface loads its policy after that acquisition.
    Stage two derives the inclusive phases/string chain internally, takes one
    selected-manifest/workspace snapshot, selects one identity and constructs
    one metadata value. Callers supply requested phase, effective policy,
    posture and ports, but cannot supply chain, phase list, selection, lease,
    metadata or trace identity. The prepared value exposes separate selected
    and workspace roots, holds the lease through execution, and the surface
    retains an owner through trace finalisation and presentation.
38. `PhaseOutcome` stays neutral until the surface. CLI default and clean paths
    share one classifier which alone preserves the historical
    InstallBarrier-vs-Lifecycle report-family choice; MCP will project the same
    neutral outcome into its generated lifecycle root in A15c. Clean itself is
    not exposed over MCP. Its pre-wipe/post-wipe composer remains a named,
    mechanically fenced debt B-110 rather than a forgeable generic setter.
39. LLM enhancement mode and agent workload are different types of choice.
    A core subsystem enhancement defaults `off`, offers explicit
    `off|assist|required`, preserves its algorithmic implementation and binds a
    provider only at the lazy paid step. An explicitly activated
    `handler=agent` contribution has no algorithmic twin: it executes, parks or
    fails honestly; provider presence still activates nothing.
40. Verification identity is scoped exact, not a repository hash and not an
    alias for the execution freshness fingerprint. The latter also includes
    requested chain, mode, world and accumulated artifacts. R7.5 refactors one
    declared-input walk to return both values: a canonical link-free input
    manifest and the existing command-specific fingerprint. Artifacts carry
    independent file/tree witnesses. Arbitrary caches/dependency stores are
    never hashed merely to mint identity.
41. Evidence comparison has exactly five words:
    `matched|stale|missing|unavailable|unstable`; only matched passes. Command
    success and verify-handler outcome stay separate. Authoring status,
    consumer adoption, optional relation-provider state/provenance and exact
    lifecycle evidence are also independent observations. No `unmet`,
    `fulfilled`, `verified` or boolean equivalent exists; their join is
    external orchestration policy.
42. Requirements is one new library capability, not a CLI/MCP composition.
    `vibe-requirements` sits above `vibe-workspace`/`vibe-facts`, owns the
    generated report and accepts an optional relation-provider trait;
    `vibe-trace` implements the current/carried specmap adapter without any
    lifecycle/specmap Cargo edge. CLI `vibe requirements` and MCP
    `requirements_query` are thin. Evidence remains on existing
    `vibe verify --json` / `lifecycle_run(verify)`; separate evidence commands
    are rejected as an identity/timing split.
43. The prototype terminology ban is superseded narrowly. `lifecycle` and
    `phase` are canonical VibeVM terms; `goal` remains prior-art. `plugin` is a
    human umbrella for a package-supplied extension/mechanism implementation,
    compiler plugin or external ecosystem artifact, never persisted identity
    or a seventh kind. Machine nouns and the six-kind package register stay
    unchanged.
44. Evidence measurement attribution is an indivisible pair:
    `measured_run_id` is present exactly when `measured` is present. The status
    matrix separately makes `unavailable` the only honest no-measurement row.
    Byte counts use canonical unsigned-decimal strings and are never narrowed
    through a machine integer; input witnesses carry the file/byte pair while
    artifact witnesses carry neither.
45. Requirements base-source identity is the package coordinate with one kind
    value, not `(kind, package)`: the same coordinate cannot occur once as host
    and once as package because `relation_sources` keys by coordinate alone
    and its provenance law must recover exactly one kind. Every relation source
    binds one enumerated base result; provenance validation never silently
    skips an unknown source.
46. The dependency-clean `vibe-facts` extraction keeps the frozen
    `(source root, package coordinate, source kind)` scanner API. It walks the
    public `vibe-core` layout, loads Markdown/XML through `vibe-specdoc` and
    canonicalises the extension away; slot records/derived manifests stay a
    materialisation optimisation, not an address dependency. This supports
    in-place package slots without creating a `vibe-facts → vibe-workspace`
    back-edge.
47. Hosted resume and stale evidence are adjacent but different proofs. A
    hosted resume re-enters the inclusive chain and legitimately reruns an
    invalidated deterministic predecessor before accepting the delegated row,
    so it may match immediately. Stale→recompute instead comes from a local
    create mutation later in one uninterrupted invocation; only the external
    second invocation recomputes. No resume law is weakened to manufacture a
    demo.
48. Verify selects current-plan input declarations plus every current
    accumulated artifact. Removed successful declarations are freshness-cache
    residue, not permanent audit rows; unwitnessed current artifacts stay
    visible as `unavailable`. Evidence stability is detection-bound (two equal
    no-follow reads and equal handle identity/length, not an atomic tree claim),
    hardlinks keep legacy fingerprint bytes while evidence refuses them, and
    the generated comparison must survive stale-stop and later-handler-failure
    carriers. `evidence_id` is one schema-order length-framed digest, never a
    JSON/Rust-layout hash.
49. Requirements source bytes are read once below the query: the pivot projects
    caller-owned raw text, facts returns only `{path,bytes,sha256}` witnesses,
    and registry parses the same bytes it witnesses. Source/scope/observation
    ids have distinct domains and labeled length frames; run-id moves the
    observation, clock does not, registry bytes move source scope. Malformed
    lock/registry abort, orphaned wins over missing slot when entries survive.
50. RelationProvider returns only `Available|Stale|Unavailable|Invalid` and
    address-associated edges. The query library derives current/carried and
    provenance from source kind, validates provider shape through the wire
    owner's edge grammar and emits explicit not-requested rows when disabled.
    Surfaces inject the optional lifecycle run id; the metadata crate has no
    lifecycle/specmap/LLM/network dependency.
51. Carried relation authority is lock hash → slot-record source hash → exact
    owned `package.specmap.json` row → one capability-relative no-follow,
    single-link, identity-rechecked byte that is both hashed and parsed. This
    proves the published map byte without rebuilding transformed/private
    source. Host config namespace must equal its coordinate; all edge files are
    workspace-root-relative. The legacy streaming payload hasher remains
    streaming; an in-memory byte twin is parity-pinned.

## 6. Physical state and loss prevention

- The final R3.2/R7.1/R8.1/R3.4 branch commits are patch-equivalent or
  superseded by `main`. At this checkpoint `git worktree list` contains only
  `main`; no campaign branch carries an unintegrated product diff.
- No campaign stash exists. Worktrees are rolling recovery state, not an
  end-of-campaign archive: once an atom is accepted/integrated and its unique
  report decisions have a durable home, remove that worktree immediately and
  prune the registry. Active dirty worktrees remain protected. Authoritative
  operating law: PROP-055 `ROLLING-WORKTREE-GC`.
- The accepted R3.4 install/lifecycle activation worktree measured
  **17,706,580,701 bytes** immediately before removal; after integration,
  durable decision capture and merged-tree gates it was removed and
  `git worktree prune` left only `main`.
- The accepted R7.4 A12 worktree was fast-forwarded through `ba874cdf`, its
  unique design/review reports were archived under local `cache/r74-a12/`, and
  the worktree plus its warm target were reclaimed immediately. Git removed
  the registry entry first; the residual long-path tree was then removed by an
  exact-path `git -c core.longpaths=true clean -ffdx` fallback. Only `main`
  remains registered.
- A13 ran directly on the already-active `main` checkout through plain
  `claude --model opus --effort max`; central review used the same-cwd `-c`
  correction loop, then reran every decisive gate before `3f01e2dc`. No new
  worktree or rebuildable target was created; the report remains local under
  `cache/r74-a13/` and only `main` is registered.
- A14 and A15a also ran directly on the active `main` checkout. Primary
  `claudez` completed the A15a extraction and two same-cwd review corrections;
  a separate `claudez2` repair lane returned its own 429 before editing, so the
  central Ultra context implemented and mutation-proved the selected-member
  resume repair. No worktree/target clone was created; local reports remain
  under `cache/r74-a15a-package-source/`, and only `main` is registered.
- A15b likewise ran directly on `main`: primary `claudez` implemented the
  closed packet, ordinary `claude -c --model opus --effort max` performed an
  independent read-only architecture review, and the same `claudez -c` thread
  applied two central correction rounds. Central acceptance then reran the
  exact gates and external E0451 probes. No new worktree/target clone exists;
  reports remain local under `cache/r74-a15b-command/`.
- A15c ran directly on `main`. The primary `claudez` thread completed recon
  and then returned the account's five-hour 429 before implementation; root
  retained the packet and took over rather than retrying. Ordinary
  `claude -c --model opus --effort max` supplied the independent architecture
  review whose clock, Agent-mode parity, best-effort seed, test isolation,
  capture-marker and source-fence corrections all landed. Central acceptance
  read every product file, reran the exact gates and restored seven mutations.
  The only non-product observation — the selected resolver's semantically
  unused workspace-root authority — is B-111 in `BACKLOG.md`; quota/failover is
  already the general PROP-055 law. No worktree/target clone was created.
- The R7.4 boundary panel first stopped at the handwritten-derive ratchet: the
  private strict MCP argument decoder is now the named non-wire exception
  `602ef5e8`. Its next run exposed a real Windows mandatory-byte-lock race in
  the pre-existing in-slot `.gitignore` fast read. Root reproduced the exact
  `os error 33` deterministically under a held peer lock; Opus/max independently
  reconstructed the same timeline. The typed `Absent | Contended | Complete`
  repair `9fc6c2bb` sends only raw Win32 lock violation into the existing
  serialising path, while every other I/O error remains fail-closed; map
  `6f074e66`. The final exact-tree panel then ran all 54 dynamic gates through
  `self-check: all green` (workspace step 917s, 448/448 `vibe-workspace`, home
  tripwires 180s/175s, markup 508/0). This is accepted boundary safety work,
  not hidden R7.4 scope substitution.
- `cache/` is untracked and unignored. It is not a product home. Important
  decisions from its R3.3, R4–R5 and R6–R8 reports are now represented in this
  ledger; future accepted decisions go directly here/spec-debt/code comments.
- `main` was mirrored at `2bb53f95` after the final R3.4 docs/specmap
  checkpoint. Build caches are reclaimed continuously: the first cleanup
  returned 164.6 GiB from root plus roughly
  372 GiB from inactive worktrees, and accepted R6.2a/R7.2 worktrees returned a
  further roughly 53 GiB immediately after integration. R6.2b then returned
  another 10,977,020,458 bytes immediately after its equivalent branch commit
  was proven present on `main`. The accepted R3.4 durable-writer worktree then
  returned 9,593,199,382 bytes after patch-id equivalence, durable decision
  capture and final main-tree exact gates were proved. The accepted R3.4
  wire/state worktree measured 24,735,991,892 bytes immediately before the same
  proof-and-reclaim sequence.
  The accepted R3.4 borrowed-threading worktree then measured 8,781,245,020
  bytes before its 25 blob-equal files plus one reviewed cross-atom test
  adaptation were proved on `main` and it was reclaimed under the same law.
  The accepted R3.4 command-core worktree measured 8,159,333,593 bytes before
  its equivalent product commit, two final reviews, exact main-tree gates and
  durable decision capture were proved; it was then reclaimed immediately.
  The R3.4 quality-tail proof/tests/errors worktrees returned 2,851,895,144,
  160,679,184 and 160,682,567 bytes respectively. Finally, every one of the
  active Update/Reinstall branch's eleven commits was patch-equivalent on
  `main`, its nine unique reports/corrections were archived, and the last warm
  worktree returned **66,787,222,941 bytes**. The registry was pruned and only
  `main` remains.
- The complete R7.4 checkpoint through `77752cd8` was then mirrored
  fast-forward-only to GitVerse and GitHub (main + tags, both targets synced).
  Only `main` is registered. After every A15c/panel finding was classified into
  code/tests, B-111 and this ledger, the two exact untracked report caches were
  reclaimed (52,638 bytes); no product or unique decision lived there.

## 7. Dependency plan and parallel lanes

### Serialization 0 — restore a truthful durable baseline (complete)

1. Land this ledger and refresh TZ/WAL/CONTINUE/TASKS pointers. **Done.**
2. Finish one unchanged full panel, host check, specmap and boot-byte no-op.
   **Done at `b90cd209`.**
3. Mirror the accumulated batch. **Done to GitVerse and GitHub.**

### Lane A — compiler and native/pass tiers (longest)

1. R3.3 verifier. **Done.**
2. Compiler IR JTD epoch and strict domain conversion substrate. **Done.**
3. R3.4 trace/timings using that same wire: observer/pre-encode budget seam,
   atomic writer + newest-nine retention, one recorder through workspace/CLI,
   then presentation/e2e. **Done through final all-green panel and GC.**
4. Pure registry extraction. **Done `6af1b86f` / `8531cf82`.**
5. R4.1 staged positions/owner views/header/oracle/fingerprint.
6. R4.2 XML minify binding and full RED corpus.
7. R4.3 analyzer wire/CLI.
8. R5 native wire + SDK; loader; build/prebuilt; bootstrap; parity.
9. R6 executable grammar/positions/frontends/backends/mandatory verifier/e2e.

### Lane B — create and hosted agent (parallel after baseline)

1. Agent output contract and generated agent-result wire; CLI provider path.
2. Feature-level algorithmic enhancement policy (`off/assist/required`) and
   lazy provider/budget accounting where a real enhancement consumes it.
3. Lifecycle run-id wire; outbox/delegated resume; invoked-by adapter.
4. MCP run/tasks adapters and standalone/hosted e2e. **R7.4 complete** through
   A15c and the all-green 54-gate boundary panel; application, trace, prompt,
   package-source and hosted run composition are shared and exact-run owned.
5. Neutral external-orchestration evidence/query substrate: exact tree/run
   identity, typed verify gaps/staleness, optional read-only `vibe-specdoc` +
   `vibe-facts` observations/current-specmap relations and
   a fake-orchestrator/PDSA reference scenario. An external long-running agent
   may consume those facts to understand which requirements remain unmet and
   where to move; lifecycle supplies evidence, never that choice. **R7.5 P0–P3
   are complete through the all-green 54-gate boundary panel.** No agent policy
   or auto-loop.

### Lane C — artifact/build/package/deploy (parallel, manifest edits serialized)

1. Artifact records/target DAG may proceed in parallel after R2.
2. Mechanism world/selection does **not** create a Lane-C registry: after Lane
   A lands R4.0, extend that same `vibe-extension-registry` kernel with the
   already-landed `[[mechanism]]` declarations, host routes and exact pins.
3. Cargo provider and `[[binary]]` compatibility lowering.
4. Static-skill and Agent Plugin package providers in parallel.
5. Client projections and general deploy planner/intent/receipt/recovery.
6. `vibe-bin`, profiles/plan/undeploy, plugin replacement fixture and Windows
   zip provider.

### Final serialization

Apply every spec-debt amendment/status with landed evidence, repair the
campaign path/format migration and stability instrument, run both owner scenarios, independent audit, final
unchanged full panel, and mirror. The epic is not complete before every row in
§3 is `done` or explicitly `future` by the owner design.
