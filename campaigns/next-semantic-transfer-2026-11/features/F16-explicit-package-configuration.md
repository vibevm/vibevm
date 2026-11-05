# F16 — Explicit package configuration through ordinary install and replay

Status: root-reviewed and accepted at the bounded runtime described here. Implementation commit: `f4f965fd9f961eaa154f63d9a6de5b3904f0feb9`. F04/common configuration and the original M05 contracts are not closed by this increment.

## User result

Ordinary authored requires input now selects optional dependencies through the existing solver and installer. The real graph, package slots, hooks and generated boot follow the selection. Mandatory legacy dependencies remain mandatory. Two real workspace members share one configured package instance; compatible additive inputs merge, conflicting exact inputs name the consumers and refuse without semantic project changes. Original intent is retained separately from effective features, including an exact→additive edit that leaves the graph unchanged.

A new process reuses that meaning. Bare install can use a fresh materialized lock without opening an unavailable registry; reinstall can rebuild deleted boot without that registry. A missing slot can be repaired from its exact store; changed store bytes or missing retained configuration refuse. An ON→OFF transition keeps a dependency required by an independent root. Source bytes stay complete rather than being pruned by the selection.

## Reuse and changes

- Reuse common feature/choice normalization and merge, the existing resolvo engine, actual workspace consumer identities, current source/cache owners and the existing metadata/slot publication machinery.
- Add explicit optional boolean metadata to the existing requires declaration, preserving omission versus explicit false across four source spellings in parser round-trips; actual workspace version.var finalization is separately tested.
- One operation-local snapshot retains candidate metadata, evaluations and failed observations across solve/retry/projection; actual fetched manifests must match it. Selected unresolved questions, malformed definitions and candidate-specific incompatibility remain distinct.
- Lock8 retains original inputs, definition/declarations and derived selected dependencies/causes under the existing lock owner. Strict legacy7 reading and legacy-only7 writing remain. Prospective and durable validation checks exact targets, original version/kind and selected effect kind assertions.
- Apply compares planned versus rediscovered original consumers before publication, including a newly added bare consumer of a configured target. It does not silently re-evaluate a changed request.
- Shared source-free Fresh eligibility precedes source construction; one existing held-world/admission/publication/resume/callback branch still applies the result.

## Defects found by integration

Actual REDs included lost rich-record reconstruction on stale replan, successful replacement of a failed metadata observation, an incremental legacy in-place path bypassing configured-source admission, and applying a plan after a sibling acquired new configuration input. All are distinct from fixture mistakes (single JSON document assumption, stdout-only diagnostic assumption, recommendation grammar, private test helper, stale latest-schema expectations).

Compatibility tests also exposed main's remap priority tie-break losing loaded-source order and legacy empty-world regeneration leaving an unreadable old lock for its callback. Fixes preserve source-local provenance and use the existing atomic publication owner. They are not reasons to copy a general prototype runtime or introduce another solver/store.

## Accepted evidence and qualifications

Observed runtime panels: public package CLI13; lock17; resolver8; actual version.var finalization1; local metadata-to-fetch drift1; incumbent visibility8; core remap35 including3 regressions; CLI remap6; source cells31; freshness24; orchestrator32; root constraints26; prepared planning9; real in-place5; metadata guard5; selected legacy CLI install/update/reinstall/uninstall controls. Repeated runs and moved assertions are not additive coverage. Root verified the worker's 91 source hashes, 76 log hashes and 15 command-receipt hashes. The acceptance archive is in the current user-local context evidence; the committed tests and this boundary are the project record.

Static gates passed: scoped formatting, strict clippy in nine scopes, conformity with zero new findings, and 510 public doctests. Root added three traceability attributes in two files with byte-identical function bodies; their formatting/clippy/conform passed. Runtime gates preceded only two inert static edits (a type alias and unchanged function relocation) plus those metadata attributes. Final specmap/check: 9086 units, 5193 tagged items, 4353 edges, zero gated orphans/unresolved host edges, 43 existing warnings. Root normal codegen corrected the generated Lockfile recoverability flag; the other 183 inspected generated files and 146 captured inputs stayed unchanged. Registry tests2 and wire-diff passed. No publication/push is authorized by this campaign slice.

## Explicit remaining work

This first runtime supports explicit non-automatic workspace input and ordinary local-registry copy sources. Candidate-conditioned transitive inputs/forwarding, defaults/profile receipts, parameters, subskill/config-data consumers, broader source modes and interactive/external question flows remain required work. Affected configured removal and scoped maintenance without a faithful consumer refuse before effects; unrelated legacy removal remains supported. Mixed configured/in-place operations explicitly refuse before checkout preparation; the old unconfigured Git path remains supported. Existing application-installer configuration is its own accepted capability.

Canonical sources: PROP-069; PROP-002#LF-CONFIGURED-STATE; architecture A0–A5 in ARCHITECTURE.md. Compatibility/change account: formats/breaks/008.md and FMT-021. No whole historical-task count is inferred from this one runtime slice.
