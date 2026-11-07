# Project install:plan — held source observations

Accepted bounded project-event slice. This delivers the real project event consumer of SourceEvidence, not project plan-only, source policy or complete VIBE-NEEDS1.

## Behavior

The existing project `install:plan` event keeps `command` and `packages[{package,version}]`, adding optional `source_evidence` to each row. Facts and coordinates come directly from the same held acquisition in PlannedInstall; observing them does not resolve a source, reread current settings or apply an in-place candidate. Conditional/transitive members use the same final held closure.

The neutral source iterator belongs to vibe-install. InstallNarration borrows the existing plan; the CLI alone constructs the generated report envelope. Human/quiet output and confirmation remain unchanged. The narration's Debug representation continues to expose only the old resolution rows, not the entire private request/manifest/plan.

Preparation for in-place updates captures the current registry alias together with held source/ref/commit. That alias feeds both reporting and the resulting lock instead of being copied from the previous lock. Canonical URL and differing actual Git origin remain distinct; the existing sanitizer is reused. Observation does not materialise or acquire again. No transport, resolution or placement policy changes.

## Event timing and schema correction

Narration remains before confirmation, but machine output remains deferred by the existing output owner. Parked execution emits its handoff document without an extra plan. Existing fresh/empty paths do not gain a synthetic plan event. The event is not an API interception point before apply, and planning may already acquire/cache source data.

The historical declared epoch1 schema required `plans` and fields the actual producer did not emit. Current `cli-install-plan` now points to epoch2 at `schemas/install/e2/plan.jtd.json`, matching actual `command/packages` output. The original schema bytes remain unchanged under the explicit `cli-install-plan-v1` registration. This is a declared-schema correction; no runtime protocol field or command rename is introduced. Older actual `packages` rows without source_evidence decode as unknown in the current permissive reader.

## Verification boundary

Observed candidate gates: prepared Git23; incremental in-place6; wire2; CLI23 (hooks3, source2, full golden5, hosted/park4, trace7, failed-update2); helper doctests1+1. Source controls include actual store reuse after source removal and a legacy lock without source_kind, retaining known alias while leaving unknown kind/commit absent. Strict Clippy, nine Conform scopes with zero findings and codegen idempotence over200 generated files pass. The gate-driven test split preserves23 prepared-source scenarios. Root verifies63 source/50 evidence hashes and the final specification map separately. Runtime evidence is from Linux/local-Git fixtures, not native Windows/macOS qualification.

Meaningful REDs were captured only after successful installation: old generated reader failed on missing `plans`, and registry source evidence was absent. The earlier malformed dependency-array fixture was a test setup error, not product evidence. Review caught and corrected the widened derived Debug surface; no runtime leak is claimed.

## Still open

Project no-write preview, per-requirement registry pins, hosting preferences/exclusions, hosting classification and skipped-candidate reasons remain separate work. This event does not retrospectively invent absent source facts, prove publisher authenticity, or mean installation succeeded merely because a plan exists. Old113 task counters are unchanged by this new owner-needs slice.
