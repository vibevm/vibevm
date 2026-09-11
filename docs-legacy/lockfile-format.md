# `vibe.lock` — schema reference

Authoritative reference for the `vibe.lock` file at the root of every vibevm project. The lockfile is the source of truth for what is installed; `vibe list` reads it, `vibe uninstall` reads it to find the `vibedeps/` slot to remove, `vibe reinstall` recomputes the materialised state from it, `vibe registry sync` walks it to refresh per-package clones. **It is committed to git.**

The file is TOML 1.0. Schema is defined by [`crates/vibe-core/src/manifest/lockfile.rs`](../crates/vibe-core/src/manifest/lockfile.rs); spec text in [`VIBEVM-SPEC.md` §7.4](../VIBEVM-SPEC.md). Identity model in [`PROP-002 §2.1`](../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#identity).

## Top-level shape

```toml
[meta]
generated_by      = "vibe 0.2.0-dev"
generated_at      = "2026-04-25T12:00:00Z"
schema_version    = 5
solver            = "resolvo-0.x"                   # optional
root_dependencies = ["org.vibevm.world/wal", "org.vibevm/rust-cli"]  # optional, may be empty

[[package]]
# ... per-package fields, repeated per installed package
```

Every key is `deny_unknown_fields` — a stray field is a hard parse error, surfaced at the next `vibe install` / `vibe update` so a typo does not silently strand state.

## `[meta]` fields

| Field | Type | Required | Semantics |
| --- | --- | --- | --- |
| `generated_by` | string | yes | Identity of the writer. Production: `vibe <version>`. Tests / fixtures: anything; checked only as a debugging breadcrumb, never parsed for behaviour. |
| `generated_at` | string | yes | RFC-3339 UTC timestamp at the moment the lockfile was written. Updated on every successful `register_installed` / `unregister_installed` call. |
| `schema_version` | uint | yes | Lockfile-format major version. Must be `5` — `Lockfile::read` rejects any other value. Earlier schema versions (1–4) are not read; the fix for an old lockfile is always to regenerate it with `vibe install`. vibevm is pre-release, so there is no on-disk migration path. |
| `solver` | string | no | Identity of the depsolver that produced this lockfile, e.g. `"resolvo-0.x"` or `"naive-1"`. Lets a future re-resolve compare-and-replay deterministically. Absent for pre-resolver Phase-A installs (the install pipeline didn't drive a solver). |
| `root_dependencies` | array of pkgref strings | no | Packages the user directly asked for (`vibe install <pkgref>` arguments), distinct from transitives the solver pulled in. Drives `vibe uninstall` semantics: removing a root drops its entry; removing a pure transitive is rejected. Empty (absent) when no `vibe install` has run yet, or when every install was via legacy paths that didn't track roots. |

## `[[package]]` entries

Each `[[package]]` block describes one installed package.

| Field | Type | Required | Semantics |
| --- | --- | --- | --- |
| `kind` | enum (`flow`, `feat`, `stack`, `tool`) | yes | Package kind per [VIBEVM-SPEC §4.1](../VIBEVM-SPEC.md). Metadata, not identity ([PROP-008 §2.3](../spec/modules/vibe-registry/PROP-008-qualified-naming.md)). |
| `name` | string | yes | Kebab-case package name (no group / kind prefix). |
| `group` | string | yes | Reverse-FQDN qualifier (`org.vibevm`). With `name` it forms the package identity — `(group, name)` is unique. [PROP-008 §2.1](../spec/modules/vibe-registry/PROP-008-qualified-naming.md). |
| `version` | semver string | yes | Resolved exact version (`"0.3.0"`). Never a constraint. |
| `registry` | string | no | The `[[registry]].name` (from `vibe.toml`) that served this package. `None` for `LocalRegistry` (`--registry <path>`), the legacy monorepo `GitRegistry`, and override-resolved entries. The single field that lets `vibe registry sync` look up which `[[registry]]` to dispatch through. |
| `source_kind` | enum (`registry`, `git`, `override`, `path`) | yes | Which resolution path produced this entry: `registry` = standard `[[registry]]` walk; `git` = a `[requires.packages]` git-source declaration; `override` = an `[[override]]` patch; `path` = a `path`-source sibling / workspace member. |
| `source_url` | string | yes | URL the content was fetched from on the install that produced this entry. Informational — see [identity model](#identity-model). |
| `source_ref` | string | no | Git ref the content was fetched at — typically `v<version>` for per-package registries; the override's ref for `[[override]]` resolutions. `None` for non-git sources (`file://...`, M0 local-directory installs). |
| `resolved_commit` | string | no | Commit hash the ref resolved to at install time. Lets a future `vibe check` detect silent tag rewrites (commit changed but `(group, name, version)` stayed the same — a force-pushed tag). Reserved; populated by the resolver when `git rev-parse` plumbing wires through. Absent today. |
| `content_hash` | string (`sha256:<hex>`) | yes | Hash over the deterministically-ordered file tree. The **identity** half of the `(group, name, version, content_hash)` tuple — see [identity model](#identity-model). |
| `boot_snippet` | string | no | **Retired by the loading model.** Formerly the `NN-`-prefixed boot-snippet filename. Under [PROP-009](../spec/modules/vibe-workspace/PROP-009-loading-model.md) `vibe` owns boot ordering by `category` and generates `INDEX.md` / `INLINE.md`, so a package no longer pins a boot filename — the field is left `None`. The struct slot is retained for schema-v5 compatibility. |
| `files_written` | array of strings | yes (may be empty) | **Retired by the loading model.** Formerly the list of every file an install wrote into the project. Under PROP-009 a package is materialised verbatim into a `vibedeps/` slot — there is no per-file write list to record — so the field is left empty. The struct slot is retained for schema-v5 compatibility. |
| `dependencies` | array of pkgref strings | no, default `[]` | Transitive deps the solver chose, pinned to exact versions (`"org.vibevm.world/atomic-commits@=0.1.0"`). Reproduces the resolved graph on a fresh install from this lockfile. Empty for a package with no dependencies. |
| `overridden` | bool | no, default `false` | True iff this package was resolved through `[[override]]` rather than the registry layer. `vibe list --overrides` filters on this; the deliberate-divergence escape hatch (`--trust-mirror`, M1.6) keys off it. |

## Identity model

Per [PROP-002 §2.1](../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#identity) and [PROP-008 §2.2](../spec/modules/vibe-registry/PROP-008-qualified-naming.md), package identity is the tuple `(group, name, version, content_hash)` — `kind` is metadata and not part of it. `source_url` is **informational** — switching mirrors, migrating between hosts, or rotating an override target produces different `source_url` values for the same identity, and the lockfile is **not** churned on those changes. `vibe install` cross-source content drift detection lives at this exact boundary: when the lockfile pins a `content_hash` and a fresh fetch produces a different one for the same `(group, name, version)`, install refuses with [`InstallError::ContentDrift`](../crates/vibe-install/src/lib.rs).

This is the structural property whose absence trapped Nix on GitHub: in Nix, `flake.lock` keys on URL + rev, so any change to the URL forces a lockfile rewrite. vibevm explicitly avoids that — the URL is a routing detail, not an identity, and the lockfile carries identity directly via `content_hash`.

## Field ordering and stability

The TOML serializer emits fields in declaration order — `[meta]` first with its own field order, then `[[package]]` blocks in install order. **`[[package]]` order is part of the lockfile contract.** Round-tripping through `read()` then `write()` preserves it; reading then re-writing without changes produces a byte-identical file (line-ending normalisation aside, which `.gitattributes` pins to LF).

`vibe install` of a new package appends; `vibe uninstall` removes the matching entry without renumbering; `vibe install` on a content-drift hit refuses entirely (no partial write). There is no entry sorting at write time — that would churn diffs unnecessarily.

## Schema versioning

`Lockfile::read` accepts only `schema_version = 5`. An older lockfile (schema 1–4) is **rejected** — there is no read-path migration. The fix is always to regenerate: delete `vibe.lock` (or just run `vibe install`) and let the resolver write a fresh v5 file. vibevm is pre-release, so no on-disk migration path is provided and none is needed.

## Tooling examples

Every `vibe.lock` is jq-friendly when piped through a TOML→JSON converter; below uses [`taplo`](https://taplo.tamasfe.dev) and `jaq`. (You can substitute any TOML/JSON tooling.)

```bash
# What's installed?
taplo get --output-format json vibe.lock | jq -r '.package[] | "\(.kind):\(.name)@\(.version)"'

# Find override-pinned packages.
taplo get --output-format json vibe.lock | jq '.package[] | select(.overridden == true)'

# Build a manifest of what to refresh ahead of an offline session.
taplo get --output-format json vibe.lock \
    | jq -r '.package[] | select(.registry != null) | "\(.registry)\t\(.kind)-\(.name)\t\(.source_ref)"'

# Sanity-check that no entry's source_url contains "anarchic/vibespecs"
# after live-migration to the per-package shape.
taplo get --output-format json vibe.lock \
    | jq -r '.package[] | select(.source_url | contains("anarchic/vibespecs")) | "\(.kind):\(.name)"'
```

For machine-to-machine consumption of `vibe list --json`, prefer the JTD schema at [`schemas/list_report.jtd.json`](../schemas/list_report.jtd.json) — same fields surface there with the same constraints.

## Worked example

A project that asked for two flows directly and pulled in one transitive dep:

```toml
[meta]
generated_by      = "vibe 0.2.0-dev"
generated_at      = "2026-04-25T12:34:56Z"
schema_version    = 5
solver            = "naive-1"
root_dependencies = ["org.vibevm.world/wal", "org.vibevm.world/atomic-commits"]

[[package]]
kind            = "flow"
name            = "wal"
group           = "org.vibevm"
version         = "0.1.0"
registry        = "vibespecs"
source_kind     = "registry"
source_url      = "git@gitverse.ru:vibespecs/flow-wal.git"
source_ref      = "v0.1.0"
content_hash    = "sha256:7d8f…b1"
files_written   = []
dependencies    = ["org.vibevm.world/atomic-commits@=0.1.0"]

[[package]]
kind            = "flow"
name            = "atomic-commits"
group           = "org.vibevm"
version         = "0.1.0"
registry        = "vibespecs"
source_kind     = "registry"
source_url      = "git@gitverse.ru:vibespecs/flow-atomic-commits.git"
source_ref      = "v0.1.0"
content_hash    = "sha256:1c4e…02"
files_written   = []
```

Reading from this:

- Both packages came from the same registry (`vibespecs`), so `vibe registry sync` will refresh both via one `MultiRegistryResolver` instance.
- `flow:wal` declared a transitive `flow:atomic-commits@^0.1`; the solver pinned it to exact `=0.1.0` (the only version available). Re-resolving against this lockfile picks the same version even if `vibespecs` later tags `v0.1.1`.
- `flow:atomic-commits` is **not** in `root_dependencies` — `vibe uninstall flow:atomic-commits` would refuse (it's a transitive, not a root). To remove it, `vibe uninstall flow:wal` first; future `vibe update --prune` will then orphan-collect `flow:atomic-commits` if no other root reaches it.
- `files_written` is empty and `boot_snippet` is absent on both entries: under the [loading model](loading-model.md) each package is materialised verbatim into a `vibedeps/` slot and boot ordering is computed — there is no per-file write list and no pinned boot filename. The slots here are `vibedeps/flow-wal/0.1.0/` and `vibedeps/flow-atomic-commits/0.1.0/`.
- `content_hash` values are the identity. A re-fetch that produces a different hash for the same `(org.vibevm, wal, 0.1.0)` would trigger `InstallError::ContentDrift` on the next install of an already-locked entry.

## Related

- [`VIBEVM-SPEC.md` §7.4](../VIBEVM-SPEC.md) — the spec-level lockfile schema.
- [`PROP-002 §2.7`](../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#lockfile) — the design lock for the lockfile fields.
- [`PROP-002 §2.1`](../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#identity) — the identity model that drives the `content_hash` field.
- [`crates/vibe-core/src/manifest/lockfile.rs`](../crates/vibe-core/src/manifest/lockfile.rs) — the Rust source of truth.
- [`schemas/list_report.jtd.json`](../schemas/list_report.jtd.json) — the JTD wire shape that surfaces these fields through `vibe list --json`.
