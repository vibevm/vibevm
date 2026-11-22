# Four owned router roots — reconstruction proposal

Date2026-11-19. Builder source HEAD `7cd10d049846a6cd227cc838e9793b666a04ca0f`, branch `task/user-project-delivery`, `/data/git/vibevm`. Read-only proposal following architect acceptance of `SYS-TAILS-ROUTER-PREPARATION-20261119.md`. No resolver, build, test, network, installation, materialization or lock mutation executed. This document is not a new lock or an accepted source epoch.

## Recommended intent

Reconstruct these owned example/package development worlds intentionally from their current authored manifests and the existing local canonical registry `/data/git/vibevm/vibevm/vibepacks`. Do not preserve obsolete Windows file URLs/schema6 as a compatibility requirement. Do not restore a permanent schema6 reader, edit a header, fabricate missing configuration or imply that current source bytes satisfy old hashes.

The four roots are:

1. `/data/git/vibevm/research/rust-demo`
2. `/data/git/vibevm/research/ts-demo`
3. `/data/git/vibevm/vibevm/vibepacks/org.vibevm.fractality/delegation-rules/v1.0.0`
4. `/data/git/vibevm/vibevm/vibepacks/org.vibevm.fractality/fractality/v1.0.0`

## Declared candidate inputs and version deltas

Read the existing lock rows as historical TOML data, not as current admitted locks. For every old coordinate, inspected conventional local registry version directories and package manifest identity/version. All70 old rows across the four locks have an identity-matching local1.0.0 manifest; these represent34 distinct coordinates. No other version directory with a matching manifest identity was observed for those coordinates. This is a finite local-source census, not a solver result or proof of complete admission.

| Root | Declared roots retained unchanged | Old rows | Proposed version delta |
|---|---|---:|---|
| rust-demo | `stack:org.vibevm.ai-native/rust-ai-native@^1.0.0` |4| None intended; four existing local1.0.0 candidates. |
| ts-demo | `stack:org.vibevm.ai-native/typescript-ai-native@^1.0.0` |4| None intended; four existing local1.0.0 candidates. |
| delegation-rules | private `flow:org.vibevm.world/redbook@^1.0.0`, private `stack:org.vibevm.ai-native/rust-ai-native@^1.0.0` |31| None intended;31 existing local1.0.0 candidates. |
| fractality | the preceding two private roots plus private `flow:org.vibevm.world/wal-specspaces@^1.0.0` |31| None intended;31 existing local1.0.0 candidates. |

Existing local manifest edges were inspected, not solved: Rust/TS aggregators require their lang/MCP packages at=1.0.0; lang requires core-ai-native^1.0.0; MCP requires corresponding lang=1.0.0. Redbook names21 exact1.0.0 flows; git-practices names four exact1.0.0 practices; wal-specspaces requires wal=1.0.0. The inspected edges name available historical coordinates. Do not claim that this alone proves current recommends/configuration/admission/compiler/lifecycle behavior or an exactly unchanged solved closure. Actual resolution must report any extra/missing row or changed version before integration rather than silently accept it.

## Source/hash/format delta policy

For each resulting row, the proposed source is the explicitly selected local registry package directory:

`file:///data/git/vibevm/vibevm/vibepacks/<group>/<name>/v1.0.0`

This replaces old `file:///C:/Users/olegc/git/v/vibevm/vibevm/vibepacks/...` addresses with a newly declared local-source epoch. These paths are machine-local development inputs, not a claim of cross-machine portable lockfiles. If portability is required, the architect must select a different actual source rather than disguise local paths as portable.

Every old lock `content_hash` remains historical; each new source admission hash is **NOT COMPUTED / NOT ADMITTED** in this preparation. Exact new hashes cannot be predicted from version equality. Source bytes have evolved, and canonical router regeneration itself may affect package payload bytes. The later receipt must list old hash → actual new hash per row, plus the exact canonical source snapshot; do not reuse an old hash simply because version1.0.0 is unchanged.

Hash owner: `vibe-registry/src/local_registry.rs::fetch` calls the registry `compute_content_hash`; `vibe-registry/src/shippable.rs::compute_content_hash` defaults to recipe0 `sha256:`. `vibe-index::compute_content_hash` instead defaults to recipe1 `sha256-tree/1:`. Do not substitute the index default, an ad-hoc Python tree hash or a manifest-only SHA for lock admission. No existing current-source hash-only executable was certified here, so this report deliberately leaves new values unknown rather than presenting a second hash implementation.

The current writer (`vibe-core/src/manifest/lockfile/configuration.rs::for_write`) selects the lowest supported lossless schema7–11. Therefore ordinary reconstruction today does NOT promise schema11. A latest-only reader/writer consolidation is a separate pending architecture/policy change; do not hand-stamp11 or block all owned fixture recovery on inventing a schema6 compatibility runtime.

## Future execution subject and sequence — not yet executed

1. Accept the exact four-root reconstruction intent and source/version-delta policy. Preserve original lock/source/boot bytes as evidence outside future test-subject roots; no destructive reset or Git metadata repair.
2. Bind the current-source Vibe binary, applicable full fractality boot/standing rules and isolated execution packet. All new task/example roots physically under `/data`. Do not use stale PATH Vibe or read ordinary credentials/config to make the run work.
3. Use fresh owned reconstruction projects with reviewed authored inputs and explicit exclusive local registry, process-local offline posture and isolated settings/cache. This is a deliberately NEW resolution world, not a replay of schema6. The command family already exists: `"$VIBE" --offline install --registry /data/git/vibevm/vibevm/vibepacks --path "$ROOT" --assume-yes`. Exact packet must establish how fresh root authorship, allowed lifecycle contributions and scoped integration are handled before execution; this preparation does not launch it or silently remove an original lock to force an empty-world install.
4. Review actual lock rows/versions/source provenance/hash deltas, compiler/admission results, generated scoped routers, preserved authored bytes and affected boot/INDEX output. Demos require actual materialization; canonical roots must not copy obsolete dependency trees as if admitted. Any lifecycle/compiler preparation or build required by current packages remains an explicit effect, not hidden inside a claimed read-only regeneration.
5. Integrate only reviewed root-owned generated outputs/current admitted locks and necessary materialized state through existing owners. No whole-host install, blanket directory copy or manual generated-block patch. Retain failed candidates and report truthful partial effects.

## Derived delegation-rules refresh is a distinct subject

The three host files at `vibevm/vibedeps/org.vibevm.fractality.delegation-rules/1.0.0/{AGENTS,CLAUDE,GEMINI}.md` are not one of the four normative roots. Canonical regeneration changes the source candidate; refreshing the host copy needs an explicitly admitted canonical package hash/source and the ordinary materialization owner, which invokes `vibedeps/instruction_block.rs` neutrality. A raw file copy is not acceptable.

Use a separate explicit materialization subject/packet for this host dependency refresh after canonical bytes are final. Its before/after host lock row, affected dependency closure and authored-text preservation must be reviewed. Existing targeted `update ... --sources-only` keeps versions but intentionally reselects source; it is not pure boot regeneration. Whole-host `reinstall --force` refetches the complete lock closure and remains outside the bounded approved preparation. Internal `rederive_package` is not an invented public selective reinstall API.

## Stopping condition

The next decision is finite: authorize/review intentional owned-world reconstruction with no intended version movement and explicit source/hash changes, or identify a concrete current owner limitation. No permanent old-format machinery, new migration framework, provider proof campaign or global format consolidation is included. This report supplies current candidate availability and honest unknowns; exact resolved/hash deltas remain required evidence from the later authorized operation.
