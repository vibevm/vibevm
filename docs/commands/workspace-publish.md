# `vibe workspace publish` — publish a workspace's members as separate repositories

Maintainer-side command. Discovers the workspace enclosing the current directory and publishes every self-publishing member as its own repository in the registry organization — the per-package [`vibe registry publish`](registry-publish.md) flow, applied across the workspace in dependency order.

The development tree stays a single monorepo. Publishing copies a member's content into a separate registry repository; the source tree is never modified. Per [PROP-007 §2.7–§2.9](../../spec/modules/vibe-workspace/PROP-007-workspace.md).

## Usage

```
vibe workspace publish [--member <rel-path>]
                       [--dry-run]
                       [--path <project>]
                       [--json | --quiet]
```

## Flags

| Flag | Description | Default |
| --- | --- | --- |
| `--member <rel-path>` | Publish only this one node, named by its path relative to the workspace root. Still skipped, with a report, if its publish posture excludes it. | all self-publishing nodes |
| `--dry-run` | Discover, select, order and stage, then report the plan — no token is loaded, no repo is created, nothing is pushed. | off |
| `--path <project>` | A directory anywhere inside the workspace; the root is discovered by walking up from it. | `.` |
| `--json` | Structured payload (see below). | off |
| `--quiet` | One-line summary. | off |

## Which nodes are published

A node is published when it carries a `[package]` table and its `[package].publish` posture admits the workspace's primary registry:

- `publish = true` (the default) — published.
- `publish = false` — never published; a workspace-internal package.
- `publish = ["vibespecs", …]` — published only when the primary registry's name is in the list.

The workspace root is included when it is itself a `[package]` (a cargo-style root package). `--member` narrows the run to one node. A workspace where every node is `publish = false` is the legitimate "nothing leaves the machine" extreme — the command reports it and exits cleanly.

## Order — dependency-first

Selected nodes publish in topological order over their inter-member `path`-dependencies: if member `A` declares `{ path = "../B" }`, `B` publishes before `A`. A dependency cycle between members is a hard error. Nodes with no inter-member path-deps keep a stable `rel_path` order.

## What a published copy carries

Each node is published from a staged copy — never the developer's working tree. Staging copies the node's directory (excluding `.git/` and `.vibe/`) and adds:

- **`[origin]`** — a provenance marker in the copy's `vibe.toml`: `upstream` (the monorepo URL — the workspace root's `git remote get-url origin`, or the root's name when it is not a git repo), `path` (the node's path within the monorepo), `commit` (the monorepo `HEAD`, when it is a git repo), `generated_by`, `generated_at`.
- A **README banner** as the first block: this is a generated copy, pull requests here are not accepted, contribution belongs to `upstream`.
- **`.github/PULL_REQUEST_TEMPLATE.md`** — a STOP notice pointing at `upstream`.
- The repository **description** is set to a "Generated copy of `<pkgref>` — contribute at `<upstream>`" line.

## Non-atomic — stop on first failure

Publishing spans N independent host repositories; there is no transaction. On the first member's failure the command stops and reports which nodes were already published and which remain, rather than pretending to roll back. Re-running resumes — an already-published member is reused, not duplicated.

## Authentication

Same publish-token loading as [`vibe registry publish`](registry-publish.md): `VIBEVM_PUBLISH_TOKEN` env-var, then `~/.vibe/<host-prefix>.publish.token`, then the legacy host-agnostic path. Token-secrecy invariants are identical (PROP-000 §20) — the token never reaches any vibevm-produced output and is embedded into the push URL only at the moment of `git push`. `--dry-run` loads no token at all.

GitVerse `[[registry]]` publishing is refused early, the same gap as `vibe registry publish`: the GitVerse public API does not expose org-scoped repo creation. `--dry-run` still works against a GitVerse registry.

## JSON output (`--json`)

```jsonc
{
  "ok": true,
  "command": "workspace:publish",
  "dry_run": false,
  "published": [
    { "pkgref": "flow:wal", "repo_name": "flow-wal", "repo_url": "https://github.com/vibespecs/flow-wal.git", "tag": "v0.1.0" }
  ],
  "skipped": [
    { "rel_path": "packages/feat-internal", "reason": "publish = false" }
  ],
  "remaining": []
}
```

`published` lists the nodes that published, in publish order. `skipped` lists `[package]` nodes excluded by their `publish` posture. `remaining` is non-empty only after a stop-on-first-failure run — it lists the nodes that were selected and ordered but not reached (the failed node included).

## Examples

```bash
# Publish every self-publishing member, dependency-first.
vibe workspace publish

# Preview the plan — selection, order, staged content — without pushing.
vibe workspace publish --dry-run

# Publish just one member.
vibe workspace publish --member packages/flow-wal
```

## Deferred

Not yet implemented (PROP-007 §2.8 polish): `--archive` (the GitHub `archived = true` lockdown and its unarchive→push→archive re-publish cycle), `has_issues = false` at repository creation, the `published_repos = "read-only" | "open"` workspace toggle, and multi-registry fan-out (publishing one node to several registries). The `[origin]` marker, README banner, PR template and description already mark a published copy unmistakably as a generated read-only copy.

## Related

- [`vibe registry publish`](registry-publish.md) — publish a single package.
- [PROP-007 §2.7–§2.9](../../spec/modules/vibe-workspace/PROP-007-workspace.md) — selective publish and the published-package-repository model.
- [`manual-tests/M1.17-workspace-publish-smoke.md`](../../manual-tests/M1.17-workspace-publish-smoke.md) — the real-network smoke recipe.
