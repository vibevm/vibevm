# Resolving version conflicts

**Q: A dependency deep in my tree pins a version that clashes with what I (or another dependency) want, and the install fails with an unsatisfiable-graph error. How do I fix it? Can I force a specific version for a dependency somewhere inside the tree?**

Short answer: yes. vibevm resolves to **one version per package** across the whole workspace (the unified-resolution model — see [Why one version at all?](#why-one-version-at-all) below), so genuinely incompatible constraints make the graph unsatisfiable and the build stops rather than silently installing two versions. This page is the practical ladder of fixes, from lightest to heaviest, ending with `[[override]]` — the deliberate "force this version" escape hatch.

## TL;DR

1. **Check it is actually a conflict.** Overlapping ranges resolve on their own; only truly incompatible ranges (different majors, disjoint windows) fail. resolvo tells you exactly which two constraints clash.
2. **Widen your own constraint** if you over-narrowed it.
3. **`vibe update`** the intermediate dependency to a version whose constraint is compatible.
4. **`[[override]]`** in `vibe.toml` to force a version/source for one pkgref, bypassing resolution — the cargo `[patch]` / go `replace` analogue.
5. **Fork + git-source** when the dependency's own constraint must change.
6. **`version.var` / `[workspace.versions]`** to centralise a version across **your own** workspace packages.

## Is it actually a conflict?

The resolver ([resolvo](https://github.com/prefix-dev/resolvo)) looks for a **single** version that satisfies **every** constraint on a package. If two constraints **overlap** — say the root wants `wal@^1.2` and a dependency wants `wal@^1.5`, both inside major 1 — there is **no conflict**: the resolver simply picks `1.5.x` (the newest version satisfying both). A conflict only arises when the ranges are genuinely incompatible — different majors, or otherwise disjoint windows.

When that happens, you do not get a bare `UNSAT`. resolvo reports the exact clash, e.g.:

```
package A needs C ^1 but B needs C ^2, and only C 1.0 and 2.0 exist
```

so you can see **which** two constraints disagree before deciding how to break the tie ([PROP-017 §2.4](../../spec/modules/vibe-resolver/PROP-017-resolvo-resolver.md#unsatisfiable)). Read the explanation first — it usually points straight at the fix.

> Note: the default solver is resolvo, which produces the rich explanation above. If you ran with `--solver naive`, you get the older single-pass `version conflict on …: already chose …, but a later constraint requires …` message instead — same underlying situation, less detail. See [Troubleshooting → Resolver errors](../troubleshooting.md#resolver-errors).

## The fix ladder

| # | Fix | Mechanism | Reach for it when |
| --- | --- | --- | --- |
| 1 | Widen your own constraint | edit `[requires.packages]` | you over-narrowed `X` without needing to |
| 2 | Update / downgrade the intermediate dep | [`vibe update`](../commands/update.md) | some version of that dep has a compatible constraint |
| 3 | **`[[override]]`** | [PROP-002 §2.4](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#override) | you want to **force** a version/source, tie-break deliberately |
| 4 | Fork + git-source | [PROP-002 §2.4.1](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#git-source) | the dependency's own constraint must be edited |
| 5 | `version.var` / `[workspace.versions]` | [PROP-007 §2.6](../../spec/modules/vibe-workspace/PROP-007-workspace.md) | centralising a version across **your own** packages |

### 1. Widen your own constraint

You own your root manifest. If you wrote `wal@=1.2.0` (or a tight caret) without needing byte-exact reproducibility, loosen it so a version the dependency also accepts becomes reachable. See [version syntax](../version-syntax.md) for caret / tilde / equal / range semantics — the difference between `^1.2` (patch+minor) and `=1.2.0` (exactly one version) is often the whole conflict.

### 2. Update or downgrade the intermediate dependency

The clashing constraint may live in a specific version of an intermediate dependency. If `foo@2.x` requires `wal@^2` but `foo@1.x` requires `wal@^1`, moving `foo` to a line that agrees with the rest of the graph resolves it. `vibe update` re-resolves against the registry; a downgrade is a constraint edit plus `vibe install`.

### 3. `[[override]]` — force a version

This is the direct "force a specific version for a dependency inside the tree" answer. `[[override]]` is vibevm's analogue of Cargo's `[patch]` and Go's `replace` ([PROP-002 §2.4](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#override)):

```toml
# in the root (or any enclosing workspace) vibe.toml:
[[override]]
pkgref     = "flow:wal"                    # even a transitive dependency
source_url = "git@mycompany:forks/wal"     # a fork, or the package's own registry repo
ref        = "my-fix-branch"               # tag / branch / commit — this picks the version
reason     = "awaiting upstream PR #42"    # shows up in `vibe list --overrides`
```

What it does, and why it fixes a conflict:

- **Short-circuits resolution.** For this pkgref the resolver does **not** consult `[[registry]]` at all — it fetches straight from `source_url` at `ref`. The intermediate packages' constraints on this package are no longer resolved against the registry; you are taking responsibility for the choice, exactly like `[patch]`.
- **Declared at the root/workspace, applies tree-wide.** An override reaches the whole dependency graph, which is how the root "tells a dependency deep in the tree which version to use." With nested workspaces, the nearest enclosing declaration wins.
- **Priority:** `[[override]] > path > git-source > registry` ([PROP-007 §2.2](../../spec/modules/vibe-workspace/PROP-007-workspace.md)). An override is a deliberate patch and outranks everything else.
- **Integrity is not relaxed.** The chosen content's `content_hash` is still pinned in `vibe.lock` and verified on every install; the entry is marked `overridden = true` and surfaces in `vibe list --overrides`.

### 4. Fork + git-source

When the dependency's own constraint genuinely has to change (it hard-pins something incompatible and no published version helps), fork it, edit its manifest, and pull the fork directly as a [git-source dependency](../git-source-dependencies.md):

```toml
[requires.packages]
"flow:wal" = { git = "https://github.com/me/flow-wal-fork", rev = "abc12345" }
```

This is the "in-flight upstream PR" workflow — the same use case `[[override]]`'s `reason` field documents. Use `[[override]]` when you want to redirect a **transitive** package tree-wide; use a git-source entry when it is a **direct** dependency you are actively developing against.

### 5. `version.var` / `[workspace.versions]` — centralise your own versions

If the "conflict" is really *drift between your own packages* that should all move together, name the version once and reference it ([PROP-007 §2.6](../../spec/modules/vibe-workspace/PROP-007-workspace.md)):

```toml
# in a [workspace] manifest:
[workspace.versions]
core = "0.0.1"
```

```toml
# in a member:
[requires.packages]
"org.vibevm/auth" = { version.var = "core" }
```

Resolution is bottom-up (nearest enclosing workspace wins). **This is not a way to force a third party**: it only helps when the dependency itself opted into `{ version.var = "…" }` instead of a concrete constraint. For a third party's hard-pinned constraint, use `[[override]]` (#3) or a fork (#4).

## Caveats

- **`[[override]]` pins a source + ref, and the version is whatever lives at that ref.** To force a specific *registry* version (both versions exist in the registry, but the constraints clash), point `source_url` at the package's registry repo and set `ref` to the wanted version tag. There is no version-only `[[override]] version = "=1.5"` form today — the source+ref shape covers the same need, just more explicitly.
- **`version.var` is registry-scoped and opt-in.** It is supported on registry-resolved dependencies only, and only when the dependency declared it. It centralises versions you control; it does not override a third party.
- **Never hand-edit `vibe.lock`** to silence a conflict — that defeats the integrity check. Fix it in `vibe.toml` (constraint, override, or git-source) and let the resolver regenerate the lock.

## Why one version at all?

vibevm uses **unified resolution**: one `vibe.lock`, one version per `(kind, name)` across the entire workspace (the diamond problem, resolved once). The engine is resolvo — a SAT resolver from the conda/rattler lineage — which enforces single-version-per-name automatically ([PROP-017 §3](../../spec/modules/vibe-resolver/PROP-017-resolvo-resolver.md#encoding)). This is deliberately unlike Cargo/npm, which let semver-major-incompatible versions coexist. The upside: your context is never fragmented across two copies of the same package, and there is never ambiguity about which version a spec refers to. The cost: incompatible constraints must be reconciled explicitly — which is what this page is for.

## See also

- [version-syntax.md](../version-syntax.md) — caret / tilde / equal / range operators; the `vibe.toml` (constraint) ↔ `vibe.lock` (pin) two-file model.
- [git-source-dependencies.md](../git-source-dependencies.md) — pulling a package straight from a git repo (fork workflow).
- [troubleshooting.md → Resolver errors](../troubleshooting.md#resolver-errors) — the raw error messages and their first-aid.
- [glossary.md](../glossary.md) — `override`, `pkgref`, `content_hash`, `transitive`, and other terms used here.
- Spec: [PROP-017](../../spec/modules/vibe-resolver/PROP-017-resolvo-resolver.md) (resolver), [PROP-002 §2.4](../../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#override) (overrides), [PROP-007 §2.6](../../spec/modules/vibe-workspace/PROP-007-workspace.md) (workspace versions).
