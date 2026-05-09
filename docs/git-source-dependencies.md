# Git-source dependencies — whole-repo-as-package

vibevm normally resolves dependencies through a `[[registry]]` org — `flow:wal` becomes `<org>/flow-wal` per the registry's `naming` convention. M1.15 adds a second shape: declare a dep as a **git-source**, pointing at any single git repository where the package's `vibe-package.toml` lives at the repository root. Same pattern as Cargo's `[dependencies] foo = { git = "..." }`, npm's `git+https://...#tag`, Poetry's `foo = { git = "..." }`, Bundler's `gem 'foo', git: '...'`. Spec: [PROP-002 §2.4.1](../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#git-source).

## When to use

- **Internal / private package without a registry org.** A team has one private vibevm package they want to share across projects; standing up a multi-package `[[registry]]` org is overkill.
- **Active development against a fork.** The fork is the canonical source while an upstream PR is in flight. (Distinct from `[[override]]`, which is a *patch* on top of an existing declaration — git-source is the declaration itself.)
- **Cross-organisation pulls.** A project consumes a package whose author lives in a different git org than any registered registry.

If the same private host serves three or more packages, declare them through a `[[registry]] auth = "token-env"` instead — the org-level shape is more ergonomic for multi-package use cases. Git-source is for one-off declarations.

## Wire form

`[requires.packages]` is a TOML table. Each entry maps a bare pkgref `<kind>:<name>` to either a constraint string (registry-resolved) or an inline-table (registry-resolved with options or git-source).

```toml
[requires.packages]
# Registry-resolved (the default M1.13 shape):
"flow:wal"        = "^0.3"
"stack:rust-cli"  = "^0.1"

# Git-source variants:
"flow:internal-helper" = { git = "git@gitlab.acme.example:specs/internal-helper",
                           tag = "v0.1.0" }
"flow:experimental"    = { git = "https://github.com/me/flow-experimental",
                           branch = "main" }
"flow:wal-fork"        = { git = "https://github.com/me/flow-wal-fork",
                           rev = "abc12345" }
"flow:secret"          = { git = "https://gitlab.acme.example/x/y",
                           tag = "v1.0",
                           auth = "token-env",
                           token_env = "VIBEVM_TARGET_TOKEN" }
"flow:checked"         = { git = "https://x/y",
                           tag = "v0.1.0",
                           version = "^0.1" }
```

The pre-M1.15 array form `packages = ["flow:wal@^0.3", ...]` continues to parse; on the next manifest write (any command that mutates `vibe.toml`) it is rewritten as the table form. There is no version fence — both shapes are read forever; only the table form is written.

### Inline-table fields

| Field | Required | Meaning |
|---|---|---|
| `git` | required for git-source | Full git URL of the single-package repo. Same URL grammar as `[[registry]] url` — `git@host:…`, `ssh://`, `https://`, `file://`. |
| `tag` | one of these required | Immutable git tag. Force-pushed tag rewrite caught as `IntegrityError` on next install via content-hash. |
| `branch` |  | Mutable git branch. `vibe install` (no `update`) sticks to the lockfile-pinned commit; `vibe update` re-walks branch HEAD. |
| `rev` |  | Commit SHA. Most strict; lockfile records the same SHA. |
| `version` | optional | Verification-only constraint. After resolving the package version from the git ref, `[package].version` in the manifest must satisfy this constraint; mismatch is `VersionMismatch`. |
| `auth` | optional | Per-source auth regime. Same enum as `[[registry]] auth` (`none` / `token-env` / `credential-helper` / `ssh`). Default `none`. |
| `token_env` | optional | Env-var name when `auth = "token-env"`. Default derived from URL host (`https://gitlab.company.com/...` → `VIBEVM_REGISTRY_TOKEN_GITLAB_COMPANY_COM`). |

**Exactly one** of `tag` / `rev` / `branch` is required for a git-source declaration. Zero is rejected at parse time as `requires exactly one of`. Two or more rejected as `must specify exactly one of`.

## Adding a git-source via CLI

Symmetric to the registry-resolved `vibe install flow:wal@^0.3` shape — adds the declaration to `vibe.toml` then resolves and installs.

```bash
# Pin to an immutable tag:
vibe install flow:internal-helper \
  --git https://github.com/me/flow-internal-helper \
  --tag v0.1.0

# Track a branch HEAD (mutable; vibe update re-walks):
vibe install flow:experimental \
  --git https://github.com/me/flow-experimental \
  --branch main

# Pin to an exact commit SHA:
vibe install flow:fork \
  --git https://github.com/me/flow-wal-fork \
  --rev abc12345

# Private target with explicit token:
vibe install flow:secret \
  --git https://gitlab.acme.example/specs/secret \
  --tag v1.0 \
  --git-auth token-env \
  --git-token-env VIBEVM_REGISTRY_TOKEN_GITLAB_ACME_EXAMPLE
```

Constraints:

- Exactly one positional pkgref `<kind>:<name>`. The version constraint goes in the value, not on the CLI; for git-source the resolution is determined by `--tag` / `--branch` / `--rev` regardless of any constraint.
- `--exact` rejected (constraint shape is orthogonal to git-source).
- `--registry` rejected (git-source bypasses the registry layer).
- Exactly one of `--tag` / `--branch` / `--rev` required.
- `--git-token-env` only meaningful with `--git-auth token-env`.

## Resolution order

When the resolver looks up a pkgref, the source is decided in this order:

1. **`[[override]]`** — if a matching override exists, it short-circuits everything (existing PROP-002 §2.4 semantics).
2. **Git-source declaration** in `[requires.packages]` — if the value carries a `git` field, the resolver fetches directly from that URL at the declared ref. `[[registry]]` is not consulted for this pkgref.
3. **Registry-resolved declaration** — bare string or `{ version = "..." }` table — falls through to the priority-ordered registry walk.

Override > git-source > registry reflects the semantic "override is intentional patch on top of a declared dependency", same as Cargo's `[patch]` overriding `[dependencies] foo = { git = "..." }`.

## Identity

Identity is `(kind, name, version, content_hash)` per PROP-002 §2.1. The hash is computed over the **target** package content, not over the URL. Two consumers that pull the same git-source from different mirrors and produce the same content hash are bit-identical installs. Force-pushed tag rewrite caught as `IntegrityError` on the next install.

The pkgref `<kind>:<name>` declared in `[requires.packages]` must match the `[package]` section in the repo's `vibe-package.toml` at the resolved ref. Mismatch — e.g. you typed `flow:internal` but the repo declares `feat:internal` — is rejected as `PackageIdentityMismatch` ("refusing to install"). This means a malicious git-source cannot impersonate `flow:wal` if its manifest declares it as `feat:auth`.

## Mutability and `vibe update`

Tags and revs are immutable by definition; force-push detected via content-hash. Branches are explicitly mutable:

- `vibe install` (no `update`) resolves from the lockfile's `resolved_commit` and does NOT re-walk branch HEAD. Reproducible-from-lockfile is the contract.
- `vibe update` re-walks every branch-declared git-source. If HEAD has moved, the resolution updates and the lockfile's `resolved_commit` advances.

This matches Cargo's behaviour (`cargo build` does not bump branch deps; `cargo update` does).

## Auth — explicit per-source

Per-source `auth` is **explicit**, not host-derived. The resolver does not look at `[[registry]] auth` for the same host and apply it transitively to a git-source pointing at that host — too magical, creates implicit ordering dependencies between sections of the manifest.

If a project has multiple packages from the same private host, the ergonomic shape is to declare them through `[[registry]]` with shared auth. Git-source is the right tool when there is one package, when the host is different from any declared registry, or when the auth needs to be different from the rest of the project.

The token-discipline contract from `docs/registry-auth.md` (read once, in-memory, scrubbed from `.git/config` after bootstrap) applies identically to git-source — same M1.14 plumbing.

## Lockfile

A git-source-resolved package surfaces in `vibe.lock` with `source_kind = "git"`, the actual fetch URL in `source_url`, and the resolved commit:

```toml
[[package]]
kind            = "flow"
name            = "internal-helper"
version         = "0.1.0"
source_kind     = "git"                                            # M1.15
source_url      = "git@gitlab.acme.example:specs/internal-helper"
source_ref      = "v0.1.0"                                         # tag/branch name/rev
resolved_commit = "abc123…def"
content_hash    = "sha256:…"
overridden      = false
```

The `source_kind` field discriminates between the three resolution paths:

- `"registry"` — standard `[[registry]]` walk.
- `"git"` — git-source declaration.
- `"override"` — `[[override]]`-resolved patch.

For pre-M1.15 lockfiles, `source_kind` is absent; consumers can assume `"override"` if `overridden = true` else `"registry"` until the lockfile is rewritten on next install.

## Transitive dependencies

A git-source package's own `[requires]` declarations are resolved through the consuming project's `[[registry]]` (same path as override-resolved packages). A git-source package may itself declare git-source dependencies — they recursively resolve through the same path with cycle detection inheriting the existing solver's protection. There is no "git-source registry" concept; the consumer project's manifest is the authoritative resolution surface for transitives.

## Comparison with `[[override]]`

Both declare a git URL + ref for a pkgref. The difference is semantic:

| | `[requires.packages]` git-source | `[[override]]` |
|---|---|---|
| Role | Primary declaration | Patch on top of an existing declaration |
| Pairs with | A bare `[requires.packages]` entry? No — git-source IS the declaration | A `[requires.packages]` entry — override patches it |
| Lockfile marker | `source_kind = "git"` | `source_kind = "override"`, `overridden = true` |
| Typical lifetime | Long-lived (project's normal architecture) | Short-lived (awaiting upstream PR / hotfix) |
| `vibe list --overrides` | Not surfaced (it is a normal dependency) | Surfaced |
| Removal | `vibe uninstall <pkgref>` drops the entry | Drop the `[[override]]` block; the underlying dependency comes back |

A project may use both: declare `flow:internal` through `[requires.packages]` git-source (the architecture), and override `flow:wal` through `[[override]]` while waiting for an upstream fix (the patch). The override always wins.

## Out of scope

- Multiple git-source entries against the same `(kind, name)` with different URLs (parallel forks). Rejected as `DuplicateDeclaration` — the operator picks one URL.
- Mirror chain for git-source. `[[mirror]]` is registry-only by design (PROP-002 §2.3); git-source has no fall-through walk.
- `vibe registry test` probing for git-source declarations. Registry-only diagnostic by design.

## Related

- [`commands/install.md`](commands/install.md) — full reference for `vibe install` flags including the M1.15 git-source affordances.
- [`registry-auth.md`](registry-auth.md) — per-registry auth regime; the same enum is used by git-source `auth =`.
- [`PROP-002 §2.4.1`](../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#git-source) — the architectural decision and the wire-grammar contract.
