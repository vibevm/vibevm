# agent-log — give-your-agent-the-skill

Fixture: `hello-vibe (see deviation)` · cwd: `work/hello-vibe` · binary: host `target/debug/vibe.exe` (`vibe 1.0.0`)
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
| 0 | `vibe mcp install --help` | 0 |
| 1 | `vibe mcp install --auto --yes --dry-run` | 0 |
| 2 | `vibe mcp status` | 0 |
| 3 | `vibe skill list` | 0 |

## Hard limit honoured

`vibe mcp install` was run **only** with `--dry-run`; nothing was written to
`~/.claude*`, `~/.codex*`, `~/.gemini*`, `~/.cursor*`. A recursive listing of all four
trees was taken before and after: the only differences are the live harnesses' own files
(this session's transcript `.jsonl`, Claude Code's own `.claude.json` rotation, Codex's
sqlite WALs). `~/.claude/skills/vibevm/SKILL.md` is byte-for-byte and mtime-for-mtime
unchanged, and `vibe mcp status` still reports it as `would-update`.

The dry run resolved four agents — claude, claude-desktop, opencode, codex — and named
every file it would write, project scope and user scope, which is exactly what the prompt
asks to be shown.

## Deviation

The `hello-vibe` fixture could not be completed in this sandbox. Both the online clone and
the in-tree-registry install of `org.vibevm.world/wal` fail on Windows path length under
this sandbox name:

```
fatal: '$GIT_DIR' too big
error: I/O error on …\vibedeps\org.vibevm.world.wal\1.0.0\vibevm\vibespecs\skills\wal-status\SKILL.md: The system cannot find the path specified. (os error 3)
```

The run therefore executed against an empty-but-valid project. Neither assert depends on
the package — but `vibe skill list` consequently reports
`(no skills declared by the project or installed packages)` instead of `wal-status`.

## Unclear from the prompt alone

- "Wire vibe into every coding agent" does not say which scope. `--auto` covers both, and
  the reader cannot tell from the prompt that project scope and user scope are separate
  writes.
- Result recorded as **conditional**: the asserts are read-only reports, and they cannot
  distinguish "installed" from "would install".
