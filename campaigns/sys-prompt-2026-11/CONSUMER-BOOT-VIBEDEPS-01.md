# Universal consumer boot: no manual dependency edits

Genre: owner clarification / bounded implementation brief. Date2026-11-19.
Owner clarification: «под загрузочными файлами я имел в виду те файлы, которые грузятся в рамках бутлейна у любого проекта. Чтобы не только мы в ходе разработки vibevm не правили vibedeps, но и любой проект, который использует vibevm.»

## Scope

This corrects the earlier insufficient host-only boot00-core change. The rule belongs in the shared boot delivery produced by Vibe for ANY consumer, not only the vibevm repository or consumers that install a particular optional development flow. Governing contract: PROP-009#CONSUMER-BOOT-DEPENDENCY-OWNERSHIP, referring to PROP-022 dependency ownership/recovery facts from commit1ad27f007.

Builder owns implementation, tests and integration; architect owns this contract/review. Use the existing generated router owner `crates/vibe-workspace/src/boot_artifacts/redirect/router.rs` and actual launch transport substitution/continuation owner. Do not add a universal package dependency, second boot file generator or new mandatory lifecycle. Do not manually alter consumer/dependency generated blocks.

## Required instruction

Use this compact general rule in the shared consumer boot:

> **Edit sources, not materializations. Never hand-edit `vibedeps`; change its canonical inputs and let Vibe regenerate it.**

Do not expand it into a checklist of payload/metadata/overlay variants or recovery exceptions in every consumer's boot. The rule states ownership, not automatic deletion or execution at session start. Exact replay, pinned sources, in-place/live/user-state protection and recovery mechanics remain in the owning contract/documentation, not duplicated in the common boot.

The wording should make ownership immediately actionable without requiring every consumer to resolve a source-repository-only spec URI. A canonical URI may accompany it, not replace the usable instruction.

## Finite acceptance

- A freshly initialized ordinary consumer and a dependency-bearing consumer both receive the rule through shared generated boot, without installing this repository's own 00-core snippet or a special flow.
- Existing managed redirect regeneration preserves all authored prefix/suffix bytes, remains byte-stable/idempotent on repeat, and all supported instruction filenames use the same shared owner.
- Native read delivery and launch-delivered boot both retain the rule exactly as intended when managed-block substitution occurs. Do not accidentally lose it from the agent lane just because it lives in the router for classic sessions.
- Rule presence does not execute install/delete, mutate HOME/personal projects or leak worker/other-project scope. Retain no-static-lane and no-opt-in behavior.
- Focused generator/transport/consumer tests are enough; do not re-run the old live provider/cache matrix merely to prove static wording delivery.
- Update the existing SYS boot/common consumer task scope with this owner clarification and actual evidence before refreshing the tree. No automatic reopening of every accepted package or unrelated new campaign.

Existing already-regenerated SYS-TAILS roots may need another ordinary idempotent publication only after the shared generator change is accepted. Reuse valid graph/materialization evidence; do not redo source reconstruction or build tools solely to add this boot instruction.

## Recipe compatibility decision

The executable addendum [CONSUMER-BOOT-RECIPE-02.md](CONSUMER-BOOT-RECIPE-02.md) resolves transport-versioning: default recipe 2, frozen recipe 1 contract bytes, and separately delivered ownership policy for explicit recipe 1. Its owner/file sequence, focused tests and exclusions govern implementation; this is a decided architecture, not another request for builder planning. Neither brief grants a coding-lane fallback.
