# agent-log — update-packages

Fixture: `hello-vibe` · cwd: `work/hello-vibe` · binary: host `target/debug/vibe.exe` (`vibe 1.0.0`)
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
| 1 | `vibe outdated` | **1** |
| 2 | `vibe update --all` | 0 |
| 3 | `diff` of `vibe.lock` before/after | 0 (identical) |

## What happened

`vibe outdated` refuses on a project that `vibe init` created:

```
error: no registry configured. Add a `[[registry]]` entry to `vibe.toml` or run
`vibe outdated` against a project that has one.
```

`vibe install` in the same project resolves fine — it uses the user-level
`~/.vibe/registry.toml` that the first `vibe` run seeds (`vibespecs`,
`vibespecs-gitverse`). `vibe outdated` and `vibe registry list` do not read that level.

## Decisions

- The skill forbids improvising registry configuration ("Do not 'fix' empty search
  results by adding new registries… report the error back verbatim"), so the run stopped
  at the error rather than running `vibe registry add`.
- `vibe update --all` still ran (the prompt asks for it) and reported
  `vibe.lock is fresh — skipping resolution`; the lock diff is empty, so there was
  nothing to summarise.

## Unclear from the prompt alone

- The prompt presupposes a project whose registries `vibe outdated` can see. Nothing in
  the prompt tells the agent that a project needs its own `[[registry]]` block for this,
  and the page's own `needs` only asks for "network access to the project's registries".
