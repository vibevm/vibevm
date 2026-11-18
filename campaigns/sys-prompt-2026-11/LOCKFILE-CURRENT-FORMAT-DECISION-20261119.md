# Lockfile format consolidation — release history and proposed policy

Genre: architecture decision proposal. Date2026-11-19. Status: history verified; exact execution placement and final contract pending affected-consumer inventory. This does not authorize a schema-header edit, restore schema6 admission or change runtime code during research.

## Owner input

> " их lock-файлы имеют schema6, а текущий Lockfile::admit принимает только schema7–11" - подумай какие версии локфайла возникли после 1.0.0. Вероятно нам нужно сразу переводить всех на самый свежий формат локфайла, а не хранить совместимость между десятком разных версий

## Verified release history

`v1.0.0` resolves to9e70e2e0f85fa5117ac18fe57ad7f4cc3e50bb25 (2026-09-27). Its `crates/vibe-core/src/manifest/lockfile.rs` declares only schema7 supported and rejects other versions; regeneration is the remedy. Schema6 predates this release.

After that release: schema8 common-package configuration (6d3d63d85,2026-11-06); schema9 remap source bindings (863fbbf37,2026-11-08); schema10 template records (4e2860880,2026-11-10); schema11 digestless templates (3ad3abf2a,2026-11-11). Current `Lockfile::admit` reads7/8/9/10/11, while configuration::for_write chooses the lowest sufficient schema. Format-registry epoch1 is a different number/domain; do not confuse it with lock schema11.

## Recommended target

One current canonical lock schema for ordinary read/write/runtime behavior, currently11, rather than retaining five historical runtime variants and a lowest-possible-schema writer. Maintain strict current-field/graph/provenance validation and unknown-future refusal. Update owned maintained locks/fixtures through the actual current operation/resolution owner, not a version-number substitution.

A bounded explicit transition/regeneration procedure may be needed for current owned inputs. It must retain meaningful pinned package versions, source identity, selected roots, configuration and template/remap records where those values exist. Missing historical original configuration cannot be fabricated from today's defaults. Source reselection or package version changes are not implied by a format conversion; bind any required source change separately. Read-only discovery must not rewrite locks.

Do not add a permanent old-format compatibility subsystem solely to perform this one transition. Do not turn a rejected malformed/rich/unknown-future lock into an empty world. Distinguish an owned reproducible old simple fixture which can be regenerated from its manifest/admitted sources from a meaningful state record needing explicit disposition before replacement.

## Finite affected-consumer inventory before implementation

- Current schema admission/writer selector and version-conditioned validation, generated/foreign format readers and protected unknown-state behavior.
- Owned tracked lock corpus and required source/manifest availability, especially the four SYS-TAILS nested roots with schema6 and obsolete machine paths.
- Tests of rich-state/no-op/offline source replay, remap/template records, atomic publication/rollback and unsupported schema refusal.
- Owning contract facts requiring lowest-version writes or old reads: PROP-002 LF-CONFIGURED-STATE and PROP-075 template lock/digest facts, plus actual format discipline. Amend these coherently, not merely implementation constants or comments.

Builder reports this inventory read-only; architect decides a bounded coherent change and its placement. Global-repair child2 already owns legacy consolidation, but a small necessary current-format transition for SYS-TAILS must not silently turn into an entire new migration campaign. Preserve the owner's route and judge actual dependency rather than doing all hypothetical format cleanup before delivering SYS-PROMPT.

## Completion evidence for an accepted implementation

Current writer emits the same current schema for simple/configured/remap/template locks; current reader validates it strictly; prior unsupported input yields actionable regeneration/transition handling without state loss. Maintained owned locks are actually updated with source/graph evidence, not manually bumped. Affected ordinary install/update/reinstall/no-op/offline consumers remain correct; historical tests are intentionally migrated with modern equivalent proof, not skipped to manufacture green. Exact candidate review and focused required checks precede plan/tree acceptance.
