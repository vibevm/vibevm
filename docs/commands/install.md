# `vibe install` — resolve and apply packages

Installs one or more packages into the current project. Every install runs through the four-stage pipeline pinned in [`VIBEVM-SPEC.md` §5.6](../../VIBEVM-SPEC.md): **resolve → plan → confirm → apply**. Installs are transitive — when a package's `[requires]` lists other packages, the depsolver pulls them in automatically.

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

1. **Resolve.** Each top-level pkgref is parsed; the project's `[[registry]]` array is consulted in priority order; `[[override]]` entries short-circuit the registry layer for specific pkgrefs. The depsolver expands transitive dependencies.
2. **Plan.** For each resolved package, the install computes the file-level diff: which files would be created, which boot snippet (if any) it contributes, any conflicts against already-installed packages or the user-owned-paths guard.
3. **Confirm.** Unless `--assume-yes` or `--json` is set, the operator sees the combined plan and confirms interactively. Decline → exit code `5`.
4. **Apply.** Files are written; the lockfile is updated atomically. On a partial failure, written files are rolled back best-effort and the error is surfaced.

## What gets written

Per package, every entry in the package's `vibe-package.toml` `[writes].files` list is materialised verbatim under the project root (mirror layout — see [`VIBEVM-SPEC.md` §13.1](../../VIBEVM-SPEC.md)). The optional `[boot_snippet]` lands at `spec/boot/<filename>`.

User-owned files (`spec/boot/00-core.md`, `spec/boot/90-user.md`, `spec/WAL.md`, `VIBEVM-SPEC.md`, `refs/book/**`, any `00-` or `90-` boot file) are never written. Any package whose declared writes target a user-owned path is rejected at plan time with exit code `3`.

## Manifest and lockfile updates

After a successful apply, `vibe install` writes:

- `vibe.toml` `[requires].packages` — appends each user-supplied pkgref (CLI args), de-duplicated by `(kind, name)`. Constraint shape rules:
  - CLI had no version (`flow:wal`) → manifest gets caret of resolved version (`flow:wal@^0.1.0`). Cargo / npm / Poetry default.
  - CLI had explicit constraint (`flow:wal@^0.1`, `@~0.1.0`, `@=0.1.0`, `@>=0.1, <0.3`, ...) → preserved verbatim; we don't tighten what the operator typed.
  - `--exact` flag set → always `=<resolved-version>`, overriding both above.

  A repeat install with a new constraint replaces the old entry. A no-arguments install (install-from-manifest mode) leaves the section untouched — the manifest was already authoritative for that input.
- `vibe.lock` — schema v2 shape ([`VIBEVM-SPEC.md` §7.4](../../VIBEVM-SPEC.md)):
  - `[meta].schema_version = 2`
  - `[meta].root_dependencies` mirrors `vibe.toml` `[requires].packages` so the lockfile is a self-contained snapshot of the solve state.
  - Per `[[package]]`: `kind`, `name`, `version`, `registry` (matching `[[registry]].name`), `source_url`, `source_ref`, `resolved_commit`, `content_hash` (the *identity* of the install), `boot_snippet`, `files_written`, `dependencies`, `overridden`.

A v1 lockfile from a pre-M1.1-revision install is read transparently (serde aliasing) and rewritten in v2 shape on the next apply. A pre-`[requires]` `vibe.toml` (manifest predates the section) is migrated automatically: when a no-arguments install finds an empty `[requires]` but a non-empty `meta.root_dependencies`, the manifest is seeded from the lockfile snapshot before resolving.

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
- `3` — plan-time conflict (boot-snippet collision, write-to-user-owned-path, two installs share a target file).
- `5` — operator declined the interactive confirmation.

## Related

- [`vibe list`](list.md) — show what's installed.
- [`vibe uninstall`](uninstall.md) — reverse an install.
- [`vibe registry sync`](registry-sync.md) — refresh per-package clones for installed packages.
- [authoring guides](../README.md) — how to write a new package.
- [`PROP-002`](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md) — registry resolution model (priority, mirrors, overrides).
