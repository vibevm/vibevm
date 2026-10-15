# M-08-C.2 independent full candidate review

**Findings: one P2; correction required before a clear C2 verdict.** No P1 found. Source review covers the full injected-root task, not CLI/MCP or C3. No build/test, product edit, Git mutation, model/process action or acceptance was performed. Implementation remains the actual managed worker's work. Local candidate/evidence files do not prove normal collection or WorkAccepted; Root owns those observations.

## P2 — One physical document can receive two different cooperating locks

Location: `crates/vibe-lifecycle/src/mechanism/deploy/generic/plan.rs:30–38`, consumed at `generic/apply.rs:40–42`.

`GenericResource::lock` encodes User destinations as `home:<relative>` and Project destinations as an absolute path. `DeployState::lock_destinations` hashes these different strings into different physical lock files. The new public host accepts both compositions below with the same settings capability:

- Host A: project `/p1`, user `/shared`, settings `/state`; approved User JSON member at `doc.json`.
- Host B: project `/shared`, user `/u2`, settings `/state`; approved Project JSON member at `doc.json`.

Every host's three roots are individually disjoint, so constructor checks permit this. Both target `/shared/doc.json`, but A locks `home:doc.json` and B locks `/shared/doc.json`. Their deployment IDs differ because the project root identity differs. Distinct new logical members in the same existing document can both be planned/approved from the same before-snapshot, enter their critical sections concurrently and perform read-modify-write without a shared physical document lock. The receipt census does not serialize the interval before either receipt is finalized.

This is a concrete source-derived counterexample to C2's second acceptance criterion and its shared-owner integration contract. It does not require symlink races, hostile ABA, a new frontend or C3. Incumbent `SafeFS::write_snapshot_guarded` expressly requires the caller's cooperating lease and states that its final comparison-to-rename window is not atomic content CAS. Consequently those snapshot checks cannot replace the missing shared lock; concurrent publications can lose the other admitted member. No race was executed by this reviewer; the differing lock identities and admitted topology are directly established by code.

Minimum correction: ensure every admitted spelling/scope of the same physical document participates in one common lock identity, while retaining coordination with incumbent provider lock identities. Do not fix this by merely adding another receipt check or claiming SafeFS provides atomic CAS. A focused fixture using two valid hosts and one shared document should demonstrate actual blocking before both critical sections, followed by stale-plan refusal/replanning or safe ordered coexistence. Reuse the existing lock owner; no new lock engine is needed.

The present contention test is meaningful but covers only a single host and User spelling. Its two logical members have the same generated lock string by construction, so its pass does not exercise this counterexample.

## Full contract coverage otherwise reviewed

All five new generic source/test files were read, together with every changed existing Rust/spec body and necessary incumbent transaction, lock, guarded-publication and ownership seams. Root-approved error and SafeFS test amendments match their exact questions/answers. The new module is publicly reachable through lifecycle exports; it is not test-only.

- C1 canonical map remains inert. Capture obtains the host's selected/enabled current observation and compares request, provider and descriptor digest; private captured/admitted/plan/grant fields cannot be reconstructed from serialized DTOs. Payload admission and explicit approval are separate. Original source/provider/declaration identities, root tokens, current principal/policy, protected controls, payload/resource/document content and prior ownership are retained and compared at issue/apply. The trusted authority is an explicit embedding contract, not a resolver or descriptor-supplied assertion.
- All four closed shapes are implemented. Tree expansion enumerates complete selected file members; typed JSON/TOML reconciliation preserves literal segmented keys and foreign values. Duplicate JSON keys, invalid/empty existing JSON, scalar ancestors, unsupported nonfinite TOML payloads, traversal/reserved components, aliases, file/tree ancestors and member-prefix conflicts refuse. Complete protected categories are required; path/physical alias checks prevent their projected overwrite. No universal policy filename or persistent grant store was introduced.
- Plans expose actual resulting configuration and admitted payload, including launch content. Approval matches the full immutable plan; missing/wrong/expanded grants and selected/source/policy/root changes refuse before infrastructure. No generic execution path has a process capability. The absent launch marker is finite fixture evidence, supplemented by source inspection, not a fabricated syscall proof.
- Apply uses incumbent state/deployment/destination locks, receipt census, Transaction::apply, pending sidecar, intent, checkpoint and finalized receipt. Ownership is checked again under locks. Whole-document merges are computed once and stale snapshots refuse rather than refresh. The lock identity finding above is the remaining gap in that composition.
- Absent destinations use create-new publication; existing documents use guarded snapshot publication. Directory creation and possibly-published failures are classified as destination effects, infrastructure errors remain separate, and failed landed publication retains actual intent/pending state. Generic resource-set migration, inverse/recovery/revocation explicitly refuse pending C3. No new requirement for those deferred operations is imposed here.

## Evidence and preservation

Before/after review hashes match all **13 candidate files**. All **51 artifact-manifest entries** and **46 source-observation entries** match their actual files. All **57 saved census pairs** were parsed, hashes checked and before/after maps compared equal. These are bounded membership/byte observations, not global isolation or adversarial atomicity proof. See `sourcehashes.json`.

Final logs contain **16 distinct nonignored passing cases**: V1=12, V3=1, V4=2, V5=1. Four V1 runs do not increase that denominator. V2 is correctly not applicable because no projection module was introduced. V6 contains separate successful scoped formatting and whitespace commands; V7 XML/preservation succeeds. The original V4 Windows-literal-on-Linux failure remains exit101, 0 passed/1 failed; the approved native Path fixture correction retains case/ancestor/sibling assertions and adds same-prefix sibling rejection. No SafeFS production change or additional non-UTF8 test was required.

Actual final test logs and bound names were inspected, not rerun. The evidence provides final source hashes and worker chronology, but no per-command consumed-source digest manifest; exact final-build binding therefore additionally relies on that recorded chronology. The last disclosed source amendment is SafeFS lib-test-only and does not alter the lifecycle dependency's production body. This limitation is not promoted to an invented extra global gate.

Removing exactly the new PROP054 line independently reconstructs prior SHA256 `cd594468e7b3cad5a5edf92855e01fcabe613426182f6fe7a60ecc03f47db2e5`; XML parses. Old C1/A2/B1 clauses are preserved. Existing non-test code changes outside generic are additive exports, state observation/identity helpers, generic ownership comparison and the approved distinct error variant. C1 request/map, upper discovery, existing vibe-bin admission and SafeFS production remain hash-equal; no wire/schema/formats or old-e1 changes are in the candidate. There is no newly triggered codegen/wire gate.

## Limits and handback

Linux injected-root generic semantics are the commissioned candidate boundary. Cross-platform execution, hostile content CAS, C3 inverse/recovery and upper CLI/MCP integration are not claimed or demanded. Preserve the entire candidate and its evidence; correct the single physical-lock identity issue through the managed worker, with focused proof and final affected-source binding. No acceptance/count change follows from this review.

VibeVM owns these package/context/projection rules; Zap retains task semantics/planning/review; PodBay retains managed process lifecycle/I/O. No neighboring project was read or changed for implementation.
