# agent-log — install-a-package

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
| 0 | `ls -a` (probe: `vibe.toml` present → skill Section B) | 0 |
| 1 | `vibe install org.vibevm.world/wal --assume-yes` | 0 |
| 2 | `vibe list` | 0 |
| 3 | `vibe tree --plain` | 0 |

## Decisions

- cwd is the project, so no `--path`. `--assume-yes` per the skill (no tty).
- "Which version was recorded" → `vibe list` (`flow wal 1.0.0`); the lock also pins
  `content_hash` and `resolved_commit`.

## Unclear from the prompt alone

- Nothing blocking.
- Observation for the page: `vibe list` prints `BOOT SNIPPET  —` for the package while
  `vibe tree` prints load `dynamic`. The two views disagree about whether the package
  contributes a snippet.
