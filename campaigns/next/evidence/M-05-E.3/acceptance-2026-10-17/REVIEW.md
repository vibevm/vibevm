# Independent second E3 review — original scope

## Recommendation

**Recommend normal Root acceptance of original E3 with the explicit limits below. 0 new P1 / 0 new P2. Both prior P2 defects are fixed.** The candidate contains materially more behavior than the first delivery. Its producer's Unsatisfied advisory is not itself an acceptance invariant; Root should judge the original task, actual source and evidence.

I am correcting the scope implication in my first review: a complete forward-apply resume engine, universal in-place/OR removal and exhaustive native/compiler fault coverage were too broad to list as unconditional E3 completion requirements. The original task does not name them. This does not turn retained journals into complete rollback or weaken whole-world semantic planning/shared-dependency protection.

No implementation, test/build, model, service/API, store or Git mutation was performed in this review. Changed code was read from the frozen snapshot; unchanged called owners from base `62b61fe6c7c43bbcb1dc24bf03c4fd95bd19179a`. All 18 byte lengths/hashes match; all 18 files independently reconstruct from base plus the 138,042-byte published patch. Patch SHA-256 is `2a8ec852e059b2132d157b63bc8889da9bd69065f015b828cbff5cf16ae35840`; managed-report SHA-256 is `34cff17d1a37baf81dea70eeb7a3ba2af0307f4b6b1c589fff9c322c4f600ab6`.

## Original mandate versus added expectations

The original `campaigns/next/tasks/M-05-E.json`, entry M-05-E.3, says:

- Goal: “Validate whole-world effects before writing and protect shared dependencies.”
- Steps: complete solve/trust/collision/effect plan, before-images and request/catalog preconditions; journal manifest/lock/owned-slot/boot prefixes with truthful non-reversible hook reporting; desired/reverse closure separating explicit/shared roots before deletion.
- Acceptance 0: “Failure injection covers each manifest/lock/slot/boot boundary and retained recovery state.”
- Acceptance 1: “Removal plan explains affected dependents before any deletion.”
- Safe stop: “Preserve partial journal and known ownership; never prune an incompletely solved world.”

The original negative case forbids *claiming complete rollback without proof*. It does not require proving complete rollback for every failed external/native effect. It does not say forward-apply resume, automatic inverse compensation of every published boot, process termination inside every syscall, or every legacy materialization/witness combination must be newly implemented.

The governing base PROP-003 has no M05E3 section. Its existing C1/C3/D1/D2/E1/E2 laws still require immutable input/selected-edge identity, complete workspace-root intent, exact explicit/default authority and full-slot preservation. The patch only adds the E3 section; prior laws are unchanged. Newly authored E3 prose is useful documentation of design and limits, not independent prior authorization to expand the original task. The local rule against accidental design also expressly says it is not permission to grow scope or anticipate every case.

| Topic | Original E3 judgment |
| --- | --- |
| Semantic whole-world plan, drift refusal, retained before-images, shared closure | Required; implemented on admitted routes and exercised |
| Real manifest/slot/boot/lock boundary faults with retained state | Required; now exercised through actual writers |
| Accurate hook uncertainty/no false complete rollback | Required; actual failed hooks and retained uncertainty exercised |
| Branch switch and unused-root removal preserving shared dependency | Required positive cases; now exercised |
| Guarded local restoration/third-state refusal | Useful concrete recovery beyond journal-only first delivery; implemented |
| Forward-apply resume of serialized desired world/source authority | Useful future capability; not explicit original criterion |
| Automatic inverse boot compensation | Useful future capability; current changed-boot safe stop is truthful and preserves before/checkpoint images |
| Exhaustive native/compiler/syscall fault matrix | Not an original gate; execution remains fallible and journalled |
| All in-place/OR/unprepared configured CLI removal routes | Expansion debt; fail-closed limitations must stay visible, not be mislabeled unsat or supported |

## Source findings and actual behavior

**The first data-loss finding is fixed.** [portable_relative_path](/fast/git/v/research/2026-10-17-m05e3-second-review/source/crates/vibe-workspace/src/configuration_transaction.rs:97) uses native components, rejects non-UTF8/backslash/colon forms and does not normalize aliases. [Whole-slot ownership](/fast/git/v/research/2026-10-17-m05e3-second-review/source/crates/vibe-workspace/src/vibedeps/removal.rs:23) uses that same checked projection for files and directories; unknown state refuses before deletion. The actual CLI fixture at uninstall.rs:688 creates recorded `a/b` plus unrecorded `a\b`, and separately an invalid-UTF8 name, then calls `uninstall::run` and proves manifest/lock/user bytes remain. Capture has its own refusal case.

**The mutable deletion proof is fixed.** [RemovalPlan](/fast/git/v/research/2026-10-17-m05e3-second-review/source/crates/vibe-install/src/removal.rs:27) keeps `removed` private and exposes an immutable borrowed accessor. The private manifest/lock and deletion membership remain bound. Public explanation values cannot change deletion membership. No compile-fail test is needed to establish Rust privacy here.

**The semantic plan is a real pre-write boundary.** Common apply checks exact manifest/lock preconditions, current authored request, captured catalog and payload identity before mutation; only the explicit reconfiguration invocation can replace prior authored intent. It prepares strict conversion, structural boot/collision composition and selected-export capture validation before entering the manifest phase. Normal lifecycle setup derives its slot plan before common apply. Removal validates the full old selected graph, desired workspace-root closure and ownership before deletion, then projects the exact retained materialized world and lowers compatibility owner plans in [preflight_removal](/fast/git/v/research/2026-10-17-m05e3-second-review/source/crates/vibe-workspace/src/install/bootgen.rs:436) before its first manifest/lock/slot write.

This is semantic/structural planning of the admitted world, not a promise that every later provider, compiler, hook, I/O operation or native preset execution will succeed. Native epoch construction/lowering remains at its incumbent post-materialization point. The report's phrase “full native/effect preflight unfinished” should not be silently equated with an original requirement to execute all native compilation before product writes. The present evidence does not qualify that stronger guarantee. I found no concrete source counterexample violating the original requested positive/negative behavior that would justify another broad native/compiler gate.

**Explicit new configuration uses current authority.** [plan_reconfiguration](/fast/git/v/research/2026-10-17-m05e3-second-review/source/crates/vibe-install/src/evolution.rs:701) requires current admitted configuration, an exact catalog and the complete workspace root union, and sets a private invocation marker. It does not use assume-yes, a changed default, discovery response or old derived selection as authority. Ordinary update/exact replay retain their previous checks. Admitted removal uses the same planner and common apply/prune writers, keeps deletion membership private and prepares its affected-dependent/keep/remove explanation first.

**Recovery is materially useful but deliberately incomplete.** [ConfigurationTransaction](/fast/git/v/research/2026-10-17-m05e3-second-review/source/crates/vibe-workspace/src/configuration_transaction.rs:438) records schema-2 before-images/checkpoints and durable write-ahead state, blocks a second apply with pending recovery, and reopens only an explicitly selected operation plus caller-admitted target set. It validates all current states before local restoration, refuses third states and retains evidence/uncertainty. Ordinary writer fault fixtures actually restore manifest/lock/slot prefixes where boot stayed unchanged. When boot/redirect state changed, restore refuses before writing and retains exact evidence; it does not lie about success. Successful local restoration is repeatable. Complete rollback is always false.

There is no inverse boot publisher or forward-apply resume. Those are real limitations, not silently completed features. For the original safe stop and acceptance wording, retained historical boot images plus explicit compensation-required refusal are an acceptable stopped recovery state. Do not advertise automatic all-prefix recovery. A manual/owner-guided recovery remains necessary at that boundary.

## Meaning of the retained evidence

The final raw logs, SHA-256 values and `.exit` sidecars match the managed report:

| Final owner selection | Pass / fail / filtered | Exit |
| --- | --- | ---: |
| install `--test m_05_e_2_cases m_05_e_3_cases` | 4 / 0 / 31 | 0 |
| workspace `--lib configuration_transaction::m_05_e_3_cases` | 5 / 0 / 744 | 0 |
| orchestrator `--lib install::tests::m_05_e_3_cases` | 2 / 0 / 666 | 0 |
| CLI `--bin vibe commands::uninstall::m_05_e_3_cases` | 8 / 0 / 958 | 0 |
| E2 `--lib install::m_05_e_2_cases::update_` controls | 2 / 0 / 666 | 0 |
| global fmt | seven baseline paths, patch-disjoint | **1** |
| diff check | empty output and exit sidecar | 0 |

That is **19 distinct E3 tests + 2 controls**, including the first candidate's eight E3 cases; not 8+19, not one test per loop iteration, and not additional successes from retries. No separate E3 Cargo target is invented. The final after-run source inventory matches this frozen patch; the worker records no edits between final source change and those final runs. This is supported final-source applicability, not an independent per-run immutable compilation seal.

[Orchestrator fault coverage](/fast/git/v/research/2026-10-17-m05e3-second-review/source/crates/vibe-orchestrator/src/install/tests.rs:346) drives `execute_prepared` through the standard source constructor, real selection and lifecycle lease. [CLI fault coverage](/fast/git/v/research/2026-10-17-m05e3-second-review/source/crates/vibe-cli/src/commands/uninstall.rs:603) drives ordinary `uninstall::run`. Faults occur after durable actual manifest/lock/slot/boot checkpoints, leaving actual changed prefixes; the tests inspect/reopen retained operations. This satisfies the original boundary requirement. It does not prove all mid-syscall crash states or a real native compiler contribution.

The configured branch-switch fixture changes admitted choice a→b, verifies app no longer depends on shared while another root still does, and preserves the shared slot record. The admitted configured-removal fixture preserves exact authored configuration and a shared dependency. Unsat/NeedsSelection and catalog/payload drift refuse without manifest/lock/recovery writes. Real pre/post shell hooks write an external marker and exit 19; pre-install errors and flagged nonfatal post-install failure retain uncertainty/evidence. This establishes truthful external effects, not compensation of those effects.

Initial E0425 compilation, three CLI fixture failures and one orchestrator restoration error are retained, with later final-source nonzero passes. Their failures are not baseline. Literal global fmt remains Failed on the original seven unrelated paths; acceptance must qualify that honestly and must not require unrelated formatting repairs. No tests were rerun by review.

## Bounded acceptance statement and debt

Original acceptance 0 and 1 have concrete sufficient evidence at the requested MVP boundary. No additional mandatory implementation packet is recommended by this review. Root may accept the original Work through the normal protocol with these limitations explicit:

- safe preservation and guarded local-prefix restoration, not automatic complete rollback;
- changed boot/redirects and unknown writer states require owner compensation/inspection;
- no authoritative forward-apply resume, exhaustive native/compiler crash qualification, universal removal support or late native carriage recovery claim;
- in-place/capability/OR and unprepared configured CLI removal continue to refuse when authority/recovery support is unavailable;
- full native/pre-execution validation and more recovery automation remain useful separately scoped debt.

The worker's conservative partial flag is useful history. It is not evidence that Root must commission all of that debt before accepting the unchanged original E3 obligations. Product work remains through managed Zap; no Root-native Vibe patch is proposed. Root owns the final semantic decision and acceptance count.
