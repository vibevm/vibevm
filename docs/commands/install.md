# `vibe install` — resolve and apply packages

Installs one or more packages into the current project. Every install runs through the four-stage pipeline pinned in [`VIBEVM-SPEC.md` §5.6](../../VIBEVM-SPEC.md): **resolve → plan → confirm → apply**. Installs are transitive — when a package's `[requires]` lists other packages, the depsolver pulls them in automatically.

## Usage

```
vibe install <pkgref> [<pkgref> ...] [--path <dir>] [--registry <path>]
             [--assume-yes]
             [--json | --quiet]
```

## Pkgref syntax

A package reference is `<kind>:<name>[@<version>]`:

| Form | Meaning |
| --- | --- |
| `flow:wal` | Latest stable. |
| `flow:wal@0.3.0` | Exact version. |
| `flow:wal@^0.3` | Highest matching version per semver caret rules. |
| `flow:wal@>=0.2, <1.0` | Compound semver constraint. |

`<kind>` is one of `flow`, `feat`, `stack`, `tool`. `<name>` is kebab-case.

## Flags

| Flag | Description | Default |
| --- | --- | --- |
| `--path <dir>` | Project directory containing `vibe.toml`. | `.` |
| `--registry <path>` | Use a local-directory registry. Overrides `[[registry]]` in `vibe.toml`. | use the configured registry |
| `--assume-yes` | Skip the interactive confirmation prompt. **Required** when stdin is not a TTY (CI, scripts). Aliased to `--yes`. | off |
| `--json` | Emit two structured documents: the plan (command `"install:plan"`) before confirmation, the report (command `"install"`) after apply. Schemas: [`schemas/install_plan.jtd.json`](../../schemas/install_plan.jtd.json), [`schemas/install_report.jtd.json`](../../schemas/install_report.jtd.json). When `--json` is set, confirmation is auto-approved (the assumption is the consumer is a script). | off |
| `--quiet` | One-line summary after apply. Conflicts with `--json`. | off |

## Pipeline

1. **Resolve.** Each top-level pkgref is parsed; the project's `[[registry]]` array is consulted in priority order; `[[override]]` entries short-circuit the registry layer for specific pkgrefs. The depsolver expands transitive dependencies.
2. **Plan.** For each resolved package, the install computes the file-level diff: which files would be created, which boot snippet (if any) it contributes, any conflicts against already-installed packages or the user-owned-paths guard.
3. **Confirm.** Unless `--assume-yes` or `--json` is set, the operator sees the combined plan and confirms interactively. Decline → exit code `5`.
4. **Apply.** Files are written; the lockfile is updated atomically. On a partial failure, written files are rolled back best-effort and the error is surfaced.

## What gets written

Per package, every entry in the package's `vibe-package.toml` `[writes].files` list is materialised verbatim under the project root (mirror layout — see [`VIBEVM-SPEC.md` §13.1](../../VIBEVM-SPEC.md)). The optional `[boot_snippet]` lands at `spec/boot/<filename>`.

User-owned files (`spec/boot/00-core.md`, `spec/boot/90-user.md`, `spec/WAL.md`, `VIBEVM-SPEC.md`, `refs/book/**`, any `00-` or `90-` boot file) are never written. Any package whose declared writes target a user-owned path is rejected at plan time with exit code `3`.

## Lockfile

The lockfile (`vibe.lock`) is updated after every successful apply, in schema v2 shape ([`VIBEVM-SPEC.md` §7.4](../../VIBEVM-SPEC.md)):

- `[meta].schema_version = 2`
- `[meta].root_dependencies` carries the user-typed pkgrefs (distinct from transitives the solver pulled in).
- Per `[[package]]`: `kind`, `name`, `version`, `registry` (matching `[[registry]].name`), `source_url`, `source_ref`, `resolved_commit`, `content_hash` (the *identity* of the install), `boot_snippet`, `files_written`, `dependencies`, `overridden`.

A v1 lockfile from a pre-M1.1-revision install is read transparently (serde aliasing) and rewritten in v2 shape on the next apply.

## Examples

Install one flow from the configured registry:

```bash
vibe install flow:wal
```

Install three flows in one transaction:

```bash
vibe install flow:wal flow:sync-from-code flow:atomic-commits
```

Pin an exact version:

```bash
vibe install stack:rust-cli@0.1.0
```

Install from a local fixture directory (M0 path, used by tests):

```bash
vibe install flow:wal --registry ./fixtures/registry --assume-yes
```

CI use — non-interactive, machine-readable output:

```bash
vibe --json install flow:wal --assume-yes \
    | jq '.installed[].package'
```

## Exit codes

- `0` — success.
- `1` — generic error (parse, network, manifest invalid, etc.).
- `3` — plan-time conflict (boot-snippet collision, write-to-user-owned-path, two installs share a target file).
- `5` — operator declined the interactive confirmation.

## Related

- [`vibe list`](list.md) — show what's installed.
- [`vibe uninstall`](uninstall.md) — reverse an install.
- [`vibe registry sync`](registry-sync.md) — refresh per-package clones for installed packages.
- [authoring guides](../README.md) — how to write a new package.
- [`PROP-002`](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md) — registry resolution model (priority, mirrors, overrides).
