# agent-log — publish-a-package

Fixture: `package-notes` · cwd: `work/` · binary: host `target/debug/vibe.exe` (`vibe 1.0.0`)
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
| 0 | `vibe registry publish --help`; `vibe registry list` | 0 |
| 1 | `vibe registry publish packages/notes --dry-run` | **1** |
| — | stopped at the prompt's question; no real push attempted | — |

## Decisions

- The prompt says "Ask me before the actual push", and the packet restricts publishing to
  `--dry-run`. So only the dry run was executed, and the run stops at the question.
- No token was set, exported, or looked for.

## What happened

Even the dry run refuses:

```
Publishing …\work\packages\notes → registry `local` (`…\work\registry`) [dry-run]
error: loading publish token: publish refused: no token available for host `C`.
```

Two causes, both worth the page's attention:

1. The fixture's first registry is a **directory path**. `vibe registry add` accepted it
   and parsed its host as the drive letter `C`, with `adapter: none — vibe registry
   publish won't dispatch here`. So "publish to the first registry of this project" can
   never succeed on this fixture.
2. `--dry-run` demands a publish token before it will plan anything. A dry run that
   cannot run without credentials is not a dry run.

## Unclear from the prompt alone

- "using the publish token already in my environment" — the prompt does not name the
  variable; `vibe registry publish` reveals it only in the failure text
  (`VIBEVM_PUBLISH_TOKEN`, or `~/.vibe/<host-prefix>.publish.token`).
- The second half ("create a scratch project elsewhere and install the published package")
  is unreachable while the first half fails.
