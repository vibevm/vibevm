# G4 partial source review

**No concrete blocking source defect found in this bounded partial: P1=0, P2=0. Full original G4 remains OPEN.** Preserve the useful Linux implementation/evidence; do not mark the Windows goal complete. No further test or implementation iteration is requested by this review.

## Source/evidence binding

Read original M-06-G.4, supplied preparation/check proposals, final published report, all7 frozen files and narrow actual callers/locked tempfile definition. Relevant 90-user build rule read; no full boot/steward. Source HEAD4d45cf26ae8ca1d901784c0f534c7db7846673b3 matches freeze; all7 hashes match at completion. Artifact SHA/length match: report0b1253d0…c7ee/52850B, patch1399cf4a…cf07/18254B. Patch paths and current tracked/untracked changes equal the7-file freeze; read-only reverse check exits0.

## Actual supported behavior

Shared reconcile_slot_with_policy invokes the new read-only preflight first, before existing record inspection/migration, new slot creation, stale-file deletion or publication. Both legacy and descriptor reconciliation enter it. Probes include the slot, record, selected payloads and representative record/payload temporary basenames. They classify only actual native length codes (Windows206, Linux36, Darwin63), not localized message text, arbitrary missing parents, permissions or logical nesting. Later reconciliation errors retain an explicit potentially-partial stage; unrelated errors keep their incumbent handling.

Diagnostic measurement is UTF-16 code units on Windows and encoded bytes on POSIX, excluding NUL. It names the path/stage/native code and distinguishes root shortening from an overlong component. It honestly states that the native error alone supplies no exact effective API/filesystem/component limit; no guessed260 threshold or filename rewrite was added. Read-only probes can miss a limit behind a missing prefix and do not promise later I/O success or all-slot transactionality. The unchanged record writer can still report its detailed string error on unexpected later failure. These are declared partial limits, not successful pre-effect proof for every platform.

Copy/hardlink fallback, incoming hash validation, sync/persist versus persist_noclobber, strict owned replacement, no-follow/record checks and foreign-neighbor cleanup rules remain in their existing owners. A probe performs no write or new authority grant. The change does not pretend the preexisting std::fs/tempfile placement has stronger no-follow/transaction guarantees than it already had. Record-last still cannot imply rollback/no effects when an earlier write failed.

## Real caller and test meaning

The fixture calls public apply_resolution_with_configuration_recovery with an actual Workspace/ResolvedDep through normal materialization and boot regeneration. The production install caller is vibe-install/src/apply.rs; descriptor/transformed materializers share reconciliation, but those paths and full CLI fetch/resolution were not separately executed by this report. Synthetic selected source hash is explicitly fixture identity, not verified registry-fetch proof.

Published default-backend Linux selection reports3passed/0failed: ordinary short-root collision/owned cleanup/retry preserving neighbor; combined long root/slot/spec path installs exact bytes/record; real Linux OS36 rejection at a4207-byte destination leaves the old record/stale file/neighbor unchanged and installs the same selected bytes after root shortening. Native cfg(windows) absence is not a pass or a failed Linux test. Scoped six-Rust formatting and diff-check pass; initial formatting failure was corrected and retained. No schema/generator/source-index change indicates a conditional codegen/specmap gate. Reviewer reran none of these checks.

## Compatibility/performance tradeoff

Locked tempfile3.27.0 implements rand_bytes(8) as8 alphanumeric filename characters, not8 cryptographic entropy bytes. Both real builders use that length, matching the representative zero-spelled staging probes. Its prior default was6, so staging basenames grow2 ASCII characters. That small near-limit tradeoff can be reconsidered with native evidence; it is not grounds to discard this partial or spawn another implementation round. Preflight adds O(files) metadata probes (roughly2 per payload plus fixed probes); runtime cost was not measured and no performance improvement/negligibility is claimed.

## Full original task gaps, separately from source defects

Native Windows execution remains OPEN: legal long-root+long-spec positive, actual unsupported native negative/new-slot pre-effect behavior, exact effective host/API/component limit and practical numeric root shortening. Windows/macOS branches were neither executed nor cross-compiled. The Windows test currently requires successful installation; it does not yet implement a native unsupported-host refusal oracle. Native proof must use actual native length units/stages, not Linux/Wine/cfg compilation as a substitute. These gaps were already original criteria; they are not optional new review requirements or fabricated test failures. No native workaround is justified without the actual reproduction.

PROP007 only adds the materialization-paths clause; removing its3new lines yields exact HEAD bytes. Prior ROOT-DISCOVERY/nesting/ONE-LOCKFILE facts remain, as do governing PROP009 two-tree ownership and PROP035 materialized-source/compiler routing/embed laws. The additive work clause explicitly leaves native proof open and does not waive the original task.

Root owns partial retention/commit/mirror, remaining platform work and eventual acceptance. Reviewer made no product/Git/API/runtime/model/process/target mutations, ran no tests/builds, read no native prefix/transcripts/credentials and created no worker/worktree.
