# agent-log — ship-tools

Fixture: `workspace-root` · cwd: `work/` · binary: host `target/debug/vibe.exe` (`vibe 1.0.0`)
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
| 0 | `vibe bin --help`, `vibe bin build --help`, `[[binary]]` schema probe | 0 / 1 |
| 1 | author `packages/notes-tools/vibe.toml` with `[[binary]] name/crate/description` | 0 |
| 2 | author `packages/notes-tools/Cargo.toml` (workspace, member `crates/notes-check`) | 0 |
| 3 | `cargo init --vcs none --bin notes-check` under `packages/notes-tools/crates` | 0 |
| 4 | `vibe check --path packages/notes-tools` | 0 |
| 5 | `vibe bin list --path packages/notes-tools` | **2** (`unexpected argument '--path'`) |
| 6 | `vibe bin list` with cwd = the package | 0, but `no installed package declares a [[binary]]` |
| 7 | `vibe bin build --assume-yes` | **1** (`no installed package declares a [[binary]]`) |
| 8 | `vibe bin exec notes-check -- --help` | **1** (`no installed package declares a binary notes-check (declared: [])`) |

## What happened

Three independent blocks:

1. `vibe bin list` has **no `--path` flag** (`--json`, `--quiet`, `--invoked-by`,
   `--agent-mode` only), so the page's first assert dies in argument parsing with exit 2.
   `vibe bin exec` silently swallows `--path` instead of rejecting it.
2. `vibe bin list | build | exec` read the **lockfile** — "every `[[binary]]` declared by
   the project's *installed packages*". A `[[binary]]` declared by the package you are
   working in is invisible to them, so "declare it, build it through vibe, run it through
   `vibe bin exec`" cannot be carried out inside the package that declares it.
3. `cargo init --vcs none` was used deliberately (no git repository is created by this
   run). `cargo` is present (1.93.1); the crate was never built, because `vibe bin build`
   never got as far as a build.

## Decisions

- The prompt says "In the package `packages/notes-tools`", but the fixture has no such
  package, so it was authored (manifest, boot snippet, Cargo workspace, binary crate) —
  the `needs` line ("a package with a Cargo workspace at its root and a binary crate")
  says that is the expected starting state.
- `crate = "crates/notes-check"` (a path relative to the package root) was a guess: the
  schema only says the field is called `crate`.

## Unclear from the prompt alone

- Whether `crates/notes-check` is relative to the project root or to the package.
- How a package's own `[[binary]]` is supposed to become visible to `vibe bin` — the
  prompt implies declaring is enough, and it is not.
