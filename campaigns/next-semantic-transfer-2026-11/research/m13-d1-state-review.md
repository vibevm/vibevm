# M13D1 state review

Read-only source review; no tests, builds, Git, network, worktrees or workers were run. Root owns acceptance. Source facts below are not runtime reproduction claims.

## P2 — live publication receipt is discarded before the domain's final observation

Location: `crates/vibe-workspace/src/migration_application/recovery.rs:456-485`, `target_write`, both Apply and Restore.

Observed source facts: the transactional-owned helper returns `OwnedPublication`. The success arm reads only `write.publication`; `write.receipt` is dropped when that arm ends. After the `after_target_publish`/`after_target_restore` boundary, the domain calls `observed` again but discards its snapshot and checks only the classified bytes/mode state. It can then persist Published/Restored. SafeFS itself DOES compare its receipt in `file/project_methods.rs:298-308`, and the existing `observed_receipt_mismatch_then_fresh_recovery_retains_explicit_limits` test covers replacement at that inner check. This review is not claiming that inner check is missing.

Concrete untested scenario: helper verification succeeds; the target name is replaced with another regular single-link object containing identical bytes/mode before the domain's final `observed` call. That call obtains the replacement identity, but the identity is ignored and the domain reports Completed and records the terminal checkpoint. The live-invocation receipt was available and could distinguish that object. This differs from accepted cross-restart byte-state recovery and does not request hostile CAS or immunity after the last observation.

Suggested bounded correction: keep the success receipt alive through the existing final observation, compare `matches_snapshot`, and report Uncertain/PublicationIo on mismatch while preserving Publishing/Restoring and target data. A post-helper/domain-pre-observation seam would discriminate this case; the current inner SafeFS hook does not. No test execution claim.

## Other reviewed boundaries

- Strict operation admission replays approved planning JSON, compares all three planner IDs, matches the separate grant and rejects effectful frozen targets before storage creation.
- Fresh apply retains before.bin, checkpoints Prepared then Publishing, confirms retained-image synchronization and reobserves source identity/bytes/mode before owned publication.
- Supported recovery matrix preserves missing/third/impossible states, refuses Complete from Restoring, and leaves Restored terminal replay as NoOp. Different approved intent under the same operation ID refuses.
- Inspect/recover acquire existing-only store and lock; no missing-store bootstrap. Inspect and terminal replay avoid retained-image synchronization. Tests define these properties, but were not executed here.
- Source tests cover 16 apply and 8 restore process-exit cuts, foreign stages, orphan/foreign images, exact reader/CRLF/UTF-8/Unix-mode preservation, actual retained-sync fault hook, and sticky uncertainty. They are definitions, not fresh execution evidence or power-loss proof.
- Loaded intent reports `target_object_history_unproven` and historical durability Unconfirmed. Same-byte replacement after restart is an expressly accepted limitation, not a finding.
- Runtime keeps Uncertain effects sticky and preserves prior completed checkpoint effects across later before-publication failures. Reports retain evidence if their validator rejects construction.
- Per-grant observation ceilings also constrain encoded checkpoints; default has no new fixed 8 MiB quota. No stronger whole-request allocation/power-loss/native-platform guarantee was inferred.
- Considered loaded Publishing/Restoring reuse without rewriting the phase; not raised as a finding: the contract explicitly qualifies historical synchronization and surviving process-recovery evidence, and permits the reduced persistence tier. A stronger power-loss architecture is outside this packet.

## Scope and drift

Full named migration owners/tests were read, plus three current JTD roots, FMT-026, selected PROP-044 migration facts and necessary SafeFS owned publication bodies. No C2 adapter or unrelated repository audit. The initial root `FORMAT-CHANGES.md` lookup was absent; the actual campaign path was found and read. All ten requested migration source hashes were unchanged between initial and final captures; hashes are in the sibling JSON. SafeFS reads are bounded owner evidence, not a complete native filesystem audit.

## Root resolution checkpoint

Backend reproduced the precise post-helper case: RED returned Completed after a same-byte/mode replacement. The success receipt now remains alive through the domain's final snapshot comparison; mismatch/unavailable observation reports Uncertain/PublicationIo and unconfirmed durability. GREEN covers this case alongside the distinct earlier-helper mismatch followed by qualified fresh recovery. This is an integration correction, not a claim that SafeFS's inner receipt verification was absent. Final frozen integration gates remain pending.
