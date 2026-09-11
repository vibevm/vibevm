# agent-log — set-up-a-workspace

Fixture: `hello-vibe-empty` · cwd: `work/hello-vibe` · binary: host `target/debug/vibe.exe` (`vibe 1.0.0`)
Isolation: `VIBE_SETTINGS` / `VIBE_REGISTRY_CACHE` / `VIBEVM_SEARCH_CACHE_DIR` inside the
sandbox (native `C:\…` spelling) plus `NO_COLOR=1`; `VIBE_OFFLINE`, `VIBE_UNATTENDED`,
`VIBE_INVOKED_BY`, `VIBE_NO_DEFAULT_REGISTRY`, `VIBETERM`, `VIBEFRAME` unset.

Standing decision for every run: `--invoked-by` was **not** passed. The skill recommends
it, but `--agent-mode` defaults to `auto`, which resolves to `agent` as soon as an
invoked-by value is present — that would change what the lifecycle commands do. The
sandbox rule "the isolation environment does not change behaviour" wins.

## Commands, in order

| # | command | exit |
|---|---|---|
| 0 | `vibe init package --help` | 0 |
| 1 | append `[workspace]` / `members = ["packages/*"]` to `vibe.toml` | 0 |
| 2 | `vibe init package org.acme/notes-flow packages/notes-flow` | 0 |
| 3 | `vibe init package org.acme/notes-docs packages/notes-docs` | 0 |
| 4 | `vibe install --assume-yes` | 0 (`3 nodes up to date`) |
| 5 | `cat vibe.lock`, `ls packages/*/vibe.lock`, `vibe check` | 0 |

## What happened

`vibe init package <pkgref> <path>` does not create a workspace member. For each member
it first scaffolds a **whole nested project** in that folder — `vibe.toml` with a
`[project]` table, its own `vibe.lock`, `.vibe/`, `.gitignore`, `CLAUDE.md`, `AGENTS.md`,
`GEMINI.md`, a boot lane — and only then writes the package itself one level down, at
`packages/<m>/vibevm/vibepacks/org.acme/<m>/v0.1.0/vibe.toml`.

So against the page's `outcome`:

- "each member folder carries its own `vibe.toml` with a `[package]` table" — the member's
  `vibe.toml` carries `[project]`, not `[package]`;
- "one `vibe.lock` at the root records the resolution" — the root lock has only `[meta]`,
  and each member has its own `vibe.lock`, which the page's third assert forbids.

The root `vibe install` does see the workspace (`3 nodes up to date`).

## Unclear from the prompt alone

- "where notes-docs documents notes-flow" cannot be expressed: `kind = "doc"` is rejected
  (`unknown variant 'doc', expected one of flow, feat, stack, tool, mcp, lang`), and
  `vibe init package` gives every member `kind = "tool"`. That half of the prompt waits
  for phase 2.
- The prompt does not say what makes a folder a member; the `[workspace] members` glob was
  taken from the fixture recipe.
