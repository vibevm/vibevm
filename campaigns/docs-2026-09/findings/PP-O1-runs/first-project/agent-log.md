# agent-log — first-project

Fixture: `empty` · cwd: `work/` · binary: host `target/debug/vibe.exe` (`vibe 1.0.0`)
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
| 0 | `ls -a` (probe: no `vibe.toml` here → skill Section A) | 0 |
| 1 | `vibe init hello-vibe` | 0 |
| 2 | `vibe install org.vibevm.world/wal --path hello-vibe --assume-yes` | 0 |
| 3 | `vibe tree --plain --path hello-vibe` | 0 |

## Decisions

- The prompt says "from the default registry", so the install carries **no** `--offline`.
  It resolved over the network: the lock records `registry = "vibespecs"`,
  `source_url = https://github.com/vibespecs/org.vibevm.world.wal.git`.
- "The reading list the agent gets at session start" → `vibe tree --plain`. The skill
  names `vibe tree` as the boot-lane view; `--plain` because there is no tty.

## Unclear from the prompt alone

- Nothing blocking. Minor: "show me the reading list" does not name a command; `vibe tree`
  is a reasonable guess, and its output for a one-package project is one row
  (`org.vibevm.world/wal dynamic . . .`) — it never names the snippet file, so a reader
  comparing it with the page's `outcome` ("`vibe tree` shows its boot snippet") may not
  recognise the answer.
