# M06C1 partial independent review

**Unaccepted/UNSATISFIED partial candidate. Two P2 semantic blockers beyond the known CLI perimeter failure.** No tests/builds, model/process actions, source edits, commits or acceptance. Root has scheduled the command_progress owner correction separately.

## P2 M06C1-P2-001 — compare confuses declared resources with present content

`crates/vibe-orchestrator/src/machine_operation/boot_debugger/dispatch.rs:188-213` builds both comparison maps from all Snapshot.resources, including entries with digest=None because active bytes are absent. M16 keeps the same admitted declaration set across captures. Thus create/delete/move are never genuine added/removed membership in this calculation.

Concrete example: admit b.md while absent, freeze old, open an actual registered client Create overlay for b.md and retain its new frozen capture. Old map has b.md→None; new map has b.md→Some(hash). Current result says changed=[b.md], added=[], removed=[]. Delete similarly becomes changed; Move becomes two changed paths. These are real current-overlay operations, not hypothetical preview or desktop UI work.

Minimum correction: build content membership from actual active presence, distinguishing explicitly absent and unavailable if needed; changed is for content present on both sides with differing digest. One actual overlay/retain_capture/Compare regression can exercise create plus delete/move and old-snapshot stability. No new capture authority is needed.

## P2 M06C1-P2-002 — available graph positions/source joins are dropped

`crates/vibe-orchestrator/src/machine_operation/boot_debugger.rs:213-227` copies only contribution, occurrence, numeric source, path/origin, requested and parent/root into generated Node. It drops GraphOccurrence.artifact_lines and never exposes the graph source identity catalogue. Host Resource rows use another identity space: with vibe.toml and STATIC.md admitted, graph source0 is not host resource0. Explain returns occurrence indices only; tree page references do not supply the missing range either.

A closed nested embed marker already gives M06B1 an exact source identity, captured emitted-artifact line span and parent. The new structured route loses that known position. Raw chunk access would force the client to reconstruct compiler semantics, contrary to the shared captured contract. This violates C1's source-position binding requirement and preserved PROP072 CAPTURED-GRAPH law; absence of mappings elsewhere does not authorize dropping an available one.

Minimum correction: extend the registered result with the existing source identity join and optional captured range, carrying artifact resource/digest/snapshot and explicit basis. **artifact_lines means one-based lines of the captured emitted artifact, not authored source lines.** Do not invent authored offsets or ancestry; emit typed unavailable for genuinely absent mappings. Convert to byte/editor positions only through a mapping the existing owner actually possesses. A nested-marker shared-route case plus an unmapped-case refusal/absence assertion is sufficient; no new parser or broad provenance redesign is requested.

## Known separate blocker and evidence

CLI source exists but its target failed E0004 at command_progress.rs:137; the original packet omitted that exhaustive policy owner. Root already corrected metadata and scheduled the same managed task. This report does not duplicate that repair as a newly discovered semantic defect or treat structured --json output as sufficient input support.

Actual distinct passed evidence is **8**: workspace2, wire2, orchestrator3, MCP1. CLI0 executed. Repeated route/MCP runs do not add cases. All9 referenced Cargo log hashes match producer report. The actual two configured headless MCP contexts share one host Arc and return matching result identities; cursor/filter/context/epoch refusal, raw reassembly, replay-gap wrapper and inspection-only cancel are meaningfully tested. The new overlay retain_capture seam has no dedicated current positive regression, which is exactly where the first defect escaped.

Current route reuses M16 capture/PageSession/stream validators and does not invoke compiler/build/provider effects. Cancellation stops inspection only. Finite capture limits, missing mapping gaps, token/billing unavailable values are explicit. Four strict generated roots and nullable readers exist, but shape checks are not evidence for missing semantic joins. check-codegen1 and wire-diff1 remain factual failures; no full-format green verdict.

All old PROP072 child tag/attribute/text content survives the additive protocol clause. No seven-law deletion or retirement occurred. No desktop M06C2 requirement or advanced hardening gate is added by these findings.

## Source state

77 paths in the producer patch manifest matched their hashes when inspected; final observation found no drift. This is observed stability during the bounded review, **not a claim that the native collector/correction lifecycle was terminal**. JSON retains exact final hashes. Candidate report29591bytes SHA256 `a8abdd58f9e8b11d22e1036344df05d68fb2068d5e05dcf11076c1858ee449b5`; patch200270bytes SHA256 `ecef3e3bf0e32d38caa0553b0b8705ab86a9c7ded4fdcf5a53fe58e5f77ab598` match published artifacts.

No original raw reasoning/native-events/secrets or historical chat inventory was read. Root alone accepts after the same managed correction addresses the known CLI seam and these two semantic cases.
