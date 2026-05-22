# `vibe install` — resolve and apply packages

Installs one or more packages into the current project. Every install runs through the four-stage pipeline pinned in [`VIBEVM-SPEC.md` §5.6](../../VIBEVM-SPEC.md): **resolve → plan → confirm → apply**. Installs are transitive — when a package's `[requires]` lists other packages, the depsolver pulls them in automatically.

`vibe install` is **workspace-aware** ([PROP-009 §2.7](../../spec/modules/vibe-workspace/PROP-009-loading-model.md)). Run anywhere inside a workspace it discovers the absolute root, runs one unified resolution across every member's `[requires]`, materialises each resolved package once into the workspace-root `vibedeps/` tree, and regenerates the boot artifacts for every entry-point node. A standalone single-package project is a degenerate (zero-member) workspace and follows the identical path. See [the loading model](../loading-model.md) for what is produced.

Two-file model. `vibe.toml` carries the *declaration* — `[requires].packages` lists every pkgref the project depends on directly. `vibe.lock` carries the *materialisation* — exact resolved versions, content hashes, transitive graph. Same shape as Cargo (`Cargo.toml` / `Cargo.lock`), npm (`package.json` / `package-lock.json`), Poetry, Bundler.

`vibe install <pkgref>` does two things: it resolves and applies the package as before, AND it appends the user-supplied pkgref to `vibe.toml` `[requires].packages` (de-duplicated by `(kind, name)`; a re-install with a new constraint replaces the old entry). `vibe install` without arguments reads `[requires].packages` and installs every entry — the cargo `cargo build` / npm `npm install` shape, useful when cloning a vibevm project from git for the first time.

## Usage

```
vibe install [<pkgref> ...] [--path <dir>] [--registry <path>]
             [--assume-yes]
             [--json | --quiet]
```

## Pkgref syntax

A package reference is `<kind>:<name>[@<version>]`. Version syntax follows Cargo / npm / Poetry conventions — bare semver is shorthand for caret, use `=` for strict equal:

| Form | Meaning |
| --- | --- |
| `flow:wal` | Latest stable; manifest stores caret of resolved version (e.g. `flow:wal@^0.1.0`). |
| `flow:wal@0.3.0` | Caret shorthand — equivalent to `^0.3.0`; matches `>=0.3.0, <0.4.0` (pre-1.0 rules). |
| `flow:wal@^0.3` | Same caret, written explicitly. |
| `flow:wal@~0.3.1` | Tilde range: `>=0.3.1, <0.4.0`. |
| `flow:wal@=0.3.0` | Strict equal — only that version. |
| `flow:wal@>=0.2, <1.0` | Arbitrary `semver::VersionReq` syntax. |

`<kind>` is one of `flow`, `feat`, `stack`, `tool`. `<name>` is kebab-case.

## Flags

| Flag | Description | Default |
| --- | --- | --- |
| `--path <dir>` | Project directory containing `vibe.toml`. | `.` |
| `--registry <path>` | Use a local-directory registry. Overrides `[[registry]]` in `vibe.toml`. | use the configured registry |
| `--assume-yes` | Skip the interactive confirmation prompt. **Required** when stdin is not a TTY (CI, scripts). Aliased to `--yes`. The global `--unattended` flag (or `VIBE_UNATTENDED` env-var) has the same effect — pick whichever reads better in your context. | off |
| `--json` | Emit two structured documents: the plan (command `"install:plan"`) before confirmation, the report (command `"install"`) after apply. Schemas: [`schemas/install_plan.jtd.json`](../../schemas/install_plan.jtd.json), [`schemas/install_report.jtd.json`](../../schemas/install_report.jtd.json). When `--json` is set, confirmation is auto-approved (the assumption is the consumer is a script). | off |
| `--quiet` | One-line summary after apply. Conflicts with `--json`. | off |
| `--exact` | Pin the resolved version exactly (`=x.y.z`) in `vibe.toml` `[requires].packages` instead of the default caret. Same shape as npm's `--save-exact`. Overrides whatever constraint the CLI form carried. | off |
| `--auth-required` | Strict authentication gate: a 401 / 403 against an `auth = "none"` (public) registry halts the install instead of walking to the next registry. Useful in CI / cron where the operator wants to gate "private install must come from the private registry; if its 401 leaks through to a public fallback, fail loudly." Per-registry `auth = "token-env"` / `"credential-helper"` halts on 401 regardless of this flag. See [`registry-auth.md`](../registry-auth.md). | off |
| `--git <URL>` | Add a git-source declaration for the single positional pkgref — fetches the package directly from this git URL rather than resolving it through `[[registry]]`. PROP-002 §2.4.1. Requires exactly one of `--tag`, `--branch`, or `--rev`. Cannot be combined with `--exact` or `--registry`. See [`git-source-dependencies.md`](../git-source-dependencies.md). | unset |
| `--tag <TAG>` | Git tag to pin against when `--git <url>` is set. Mutually exclusive with `--branch` / `--rev`. Immutable; force-pushed tag rewrite caught as `IntegrityError` on next install via content-hash. | unset |
| `--branch <BRANCH>` | Git branch to track when `--git <url>` is set. Mutually exclusive with `--tag` / `--rev`. Mutable: `vibe install` (no `update`) sticks to the lockfile-pinned commit; `vibe update` re-walks branch HEAD. | unset |
| `--rev <REV>` | Git commit SHA to pin against when `--git <url>` is set. Mutually exclusive with `--tag` / `--branch`. Most strict; the lockfile records the same SHA. | unset |
| `--git-auth <AUTH>` | Auth regime for the `--git <url>` target — same enum as `[[registry]] auth`: `none` / `token-env` / `credential-helper` / `ssh`. | `none` |
| `--git-token-env <ENV_VAR>` | Env-var name when `--git-auth token-env`. Default derived from URL host. | derived |

## Pipeline

1. **Resolve.** Each top-level pkgref is parsed; the project's `[[registry]]` array is consulted in priority order; `[[override]]` entries short-circuit the registry layer for specific pkgrefs. The depsolver runs one unified resolution across the whole workspace and expands transitive dependencies.
2. **Plan.** The plan's unit is **the set of packages to materialise into `vibedeps/` plus the boot artifacts to regenerate** — not a per-file write list. Plan-time validation also classifies every entry-point node's `<vibevm>` instruction-file block ([PROP-012 §2.5](../../spec/modules/vibe-workspace/PROP-012-managed-redirect-block.md)); a malformed block aborts the operation before any mutation.
3. **Confirm.** Unless `--assume-yes` or `--json` is set, the operator sees the combined plan and confirms interactively. Decline → exit code `5`.
4. **Apply.** Each resolved package's published tree is materialised verbatim into its `vibedeps/` slot; the boot artifacts (`spec/boot/INLINE.md`, `spec/boot/INDEX.md`) are regenerated for every entry-point node; the `<vibevm>` redirect block is spliced into each instruction file; the lockfile is updated. Stale `vibedeps/` slots no longer in the resolution are pruned.

## Incremental install

`vibe install` is incremental ([PROP-011](../../spec/modules/vibe-workspace/PROP-011-incremental-install.md)) — it does the least work a change requires.

- **Fresh lockfile → no resolution.** Before the depsolver runs, a bare `vibe install` (no pkgref arguments) checks whether `vibe.lock` is still a correct resolution of every node's `[requires]`. When it is, the depsolver is skipped entirely — no registry walk, no network — and the run goes straight to a whole-tree boot regeneration. This makes `vibe install` **lockfile-respecting**: an unchanged `[requires]` honours the versions `vibe.lock` pins, with no silent drift inside a `^` constraint. The `--json` report of a skipped install carries `"unchanged": true` and no preceding plan.
- **Changed lockfile → minimum churn.** When `[requires]` *has* changed, `vibe install` re-resolves, but holds the locked version of every dependency the change did not touch — only the changed dependency and its subtree move. Moving an untouched version is `vibe update`'s job.
- **Materialise only the diff.** A `vibedeps/` slot already present for the resolved version is not re-copied — versions are immutable, so its content is correct. Only a new or version-bumped dependency is materialised.

### `slot_integrity` — the materialisation strategy

The materialise-diff skip is governed by an `[install]` key in the vibevm user config (`~/.config/vibe/config.toml`, or `%APPDATA%\vibe\config.toml` on Windows):

```toml
[install]
slot_integrity = "trust-presence"   # default — or "verify"
```

- `trust-presence` (default) — trust a slot present for the resolved version; skip the re-copy. Fast.
- `verify` — re-materialise every slot from source on every install, overwriting a hand-edited or corrupted one. Trades the skip for a per-install guarantee.

To re-fetch and re-materialise the whole tree once — the repair for a corrupted `vibedeps/` subtree — use [`vibe reinstall --force`](reinstall.md).

## What gets written

`vibe install` writes to two places, and **never to a node's authored `spec/`** — the C++-`#include` rule ([PROP-009 §2.1](../../spec/modules/vibe-workspace/PROP-009-loading-model.md#two-trees)):

- **The `vibedeps/` tree** at the absolute workspace root. Each resolved package's published tree is materialised verbatim into a slot, `vibedeps/<kind>-<name>/<version>/`. A materialised package *is* its subtree under that slot — there is no per-file write list and no `[writes]` section to author. `vibedeps/` is committed to git. A publishable package's `vibe.toml` carries a `[package]` table; that is what marks it as an installable artifact.
- **The generated boot artifacts** under each entry-point node's `spec/boot/`: `INLINE.md` (the verbatim `inline`-linked priority lane, when there are inline contributions) and `INDEX.md` (a generated TOML manifest of the computed boot sequence). Plus the managed `<vibevm>` block inside each instruction file (`CLAUDE.md` / `AGENTS.md` / `GEMINI.md`) — vibevm writes only between the markers; the rest of the file is preserved verbatim ([PROP-012](../../spec/modules/vibe-workspace/PROP-012-managed-redirect-block.md)).

A node's authored `spec/` tree — including the conventional user-owned boot files `spec/boot/00-core.md` and `spec/boot/90-user.md`, and `spec/WAL.md` — is written only by its author. `vibe install` references those files in the computed boot sequence but never rewrites them. See [the loading model](../loading-model.md) for the full picture.

## Manifest and lockfile updates

After a successful apply, `vibe install` writes:

- `vibe.toml` `[requires].packages` — appends each user-supplied pkgref (CLI args), de-duplicated by `(kind, name)`. Constraint shape rules:
  - CLI had no version (`flow:wal`) → manifest gets caret of resolved version (`flow:wal@^0.1.0`). Cargo / npm / Poetry default.
  - CLI had explicit constraint (`flow:wal@^0.1`, `@~0.1.0`, `@=0.1.0`, `@>=0.1, <0.3`, ...) → preserved verbatim; we don't tighten what the operator typed.
  - `--exact` flag set → always `=<resolved-version>`, overriding both above.

  A repeat install with a new constraint replaces the old entry. A no-arguments install (install-from-manifest mode) leaves the section untouched — the manifest was already authoritative for that input.
- `vibe.lock` — schema v4 shape ([`VIBEVM-SPEC.md` §7.4](../../VIBEVM-SPEC.md)):
  - `[meta].schema_version = 4`
  - `[meta].root_dependencies` mirrors `vibe.toml` `[requires].packages` so the lockfile is a self-contained snapshot of the solve state.
  - Per `[[package]]`: `kind`, `name`, `version`, `registry` (matching `[[registry]].name`), `source_kind` (one of `registry`, `git`, `override`, `path`), `source_url`, `source_ref`, `resolved_commit`, `content_hash` (the *identity* of the install), `dependencies`, `overridden`. Under the loading model the per-file `files_written` list and the `boot_snippet` filename field are retired (a materialised package *is* its `vibedeps/` slot, and boot ordering is computed) — see [`docs/lockfile-format.md`](../lockfile-format.md).

`Lockfile::read` accepts only `schema_version = 4`; an older lockfile (schema 1, 2, or 3) is rejected rather than migrated — regenerate it with `vibe install`. vibevm is pre-release, so there is no on-disk migration path and none is needed.

## Examples

Install one flow from the configured registry (and record it in `vibe.toml` `[requires]`):

```bash
vibe install flow:wal
```

Reproduce a project's full package set after `git clone` (reads `vibe.toml` `[requires]`):

```bash
vibe install
```

Install three flows in one transaction:

```bash
vibe install flow:wal flow:sync-from-code flow:atomic-commits
```

Pin an exact version (Cargo `=` form):

```bash
vibe install stack:rust-cli@=0.1.0
```

Use `--exact` so the manifest pins to the resolved version regardless of CLI form:

```bash
vibe install flow:wal --exact
# `vibe.toml` ends up with `flow:wal@=0.1.0` (or whatever resolved)
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
- `3` — plan-time conflict (a malformed `<vibevm>` block in an instruction file, a resolver-level package conflict).
- `5` — operator declined the interactive confirmation.

## Related

- [`vibe list`](list.md) — show what's installed.
- [`vibe uninstall`](uninstall.md) — reverse an install.
- [`vibe reinstall`](reinstall.md) — recompute `vibedeps/` and the boot artifacts without re-resolving.
- [`vibe registry sync`](registry-sync.md) — refresh per-package clones for installed packages.
- [The loading model](../loading-model.md) — the `vibedeps/` tree and the generated boot artifacts an install produces.
- [authoring guides](../README.md) — how to write a new package.
- [`PROP-002`](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md) — registry resolution model (priority, mirrors, overrides).
