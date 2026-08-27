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
All accepted R3.4 worktrees and fan-out worktrees were reviewed, archived and
reclaimed under rolling GC; **only `main` remains**. Missing later waves were
never implemented.

The reusable decisions from untracked architecture/review reports are
synthesised below. Those reports are evidence inputs, not authority and not the
only durable home of a decision.
[`RETROSPECTIVE-SPEC-HARVEST-2026-08-27.md`](RETROSPECTIVE-SPEC-HARVEST-2026-08-27.md)
is the tracked index/queue for report-derived candidates not yet owned by a
later implementation row; every future wave drains its named candidates rather
than depending on untracked `cache/` archaeology.

## 2. Evidence standard and current baseline

- Integration checkpoint: `main` at `3656a889` contains the complete accepted
  R3.4 chain, final command-root oracle migration, conform/document REDs and
  derived retrieval map. `git cherry main wt/r34-update-reinstall` marked all
  eleven branch commits patch-equivalent before that last worktree was removed.
- The final unchanged tree passed the whole self-check through
  `self-check: all green` and `SELF_CHECK_EXIT=0` on 2026-08-28. The script's
  lexical denominator advertised 47 while dynamic loop calls executed 54
  gates; the denominator defect is recorded in PROP-055, and every emitted
  gate including the final whole-run user-home tripwire passed.
- Final host `vibe check`: 0 errors, 5 warnings, 0 info. Final workspace
  conform: 27 visible acknowledged findings, 0 frozen and **0 new**. The
  panel's full workspace tests, clippy `-D warnings`, codegen, wire-diff,
  package/MCP suites, self-traces and markup validation all passed.
- Post-checkpoint `cargo xtask specmap`: 6786 units, 2073 tagged code items, 1856 edges,
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
| R4.0 one pure registry below lifecycle/workspace | missing | required to avoid a dependency cycle and a duplicate extension machine |
| R4.1 four positions, host activation, header, fingerprint, reference oracle | missing | no transform descriptors/passes/header binding |
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
| R7.4 MCP lifecycle surfaces | in progress | architecture `ee2bc67f`; first wave landed: typed executed MCP output `94f30aa9`, bounded capability reads `88600508`, CLI orchestration goldens `87c2bab8`, minimal projection cut `daf6eb31`, lifecycle-tasks JTD/corpus `17d94f8f`, combined specmap `1ada3df8`; A4 pinned transactional state active, then lease/selected/tasks adapter/orchestrator/run |
| R7.5 external orchestration substrate | missing | owner ruling reaffirmed 2026-08-28: structured lifecycle evidence + exact tree/run identity + optional read-only requirements/spec-IR facts which an external long-running agent may use to see unmet/stale requirements and choose direction; no coding agent or automatic loop inside lifecycle; reference agent is a future campaign |

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
    selectors and views. Workspace adapters supply the authoritative root lock
    order; unsorted materialised-directory enumeration is never ordering input.
    Detailed extraction boundary:
    [`R4-REGISTRY-KERNEL-ARCHITECTURE-v0.1.md`](R4-REGISTRY-KERNEL-ARCHITECTURE-v0.1.md).
16. R4's untransformed emitter remains the reference oracle. An emitted
    transform therefore needs a manager-owned constructor that recomputes
    provenance/digests after the oracle; mutating `EmittedArtifact.bytes` in
    place is forbidden. An active-plan-only transforms header must be accepted
    by the emitted-tape validators and enter both node and per-unit
    fingerprints. Per-unit lanes must join the crash-safe artifact transaction
    before byte-changing transforms ship. XML-minify needs explicit REDs for
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
4. Pure registry extraction.
5. R4.1 staged positions/header/oracle/fingerprint.
6. R4.2 XML minify binding and full RED corpus.
7. R4.3 analyzer wire/CLI.
8. R5 native wire + SDK; loader; build/prebuilt; bootstrap; parity.
9. R6 executable grammar/positions/frontends/backends/mandatory verifier/e2e.

### Lane B — create and hosted agent (parallel after baseline)

1. Agent output contract and generated agent-result wire; CLI provider path.
2. Feature-level algorithmic enhancement policy (`off/assist/required`) and
   lazy provider/budget accounting where a real enhancement consumes it.
3. Lifecycle run-id wire; outbox/delegated resume; invoked-by adapter.
4. MCP run/tasks adapters and standalone/hosted e2e. **R7.4 in progress:**
   bounded safefs read / wire / ToolOutput / projection / CLI-characterization
   first wave is landed and exactly gated; state I/O is active, then lease,
   selected-node and tasks adapter; shared orchestrator + run adapter last.
   Full panels only at the two coherent boundaries.
5. Neutral external-orchestration evidence/query substrate: exact tree/run
   identity, stale/unmet verify evidence, optional read-only spec-IR facts and
   a fake-orchestrator/PDSA reference scenario. An external long-running agent
   may consume those facts to understand which requirements remain unmet and
   where to move; lifecycle supplies evidence, never that choice. **Then
   R7.5.** No agent policy or auto-loop.

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
