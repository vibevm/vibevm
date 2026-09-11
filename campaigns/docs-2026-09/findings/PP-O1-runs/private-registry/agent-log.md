# agent-log — private-registry

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
| 0 | `vibe registry add --help`; `vibe registry list` (probe: **no** registries in `vibe.toml`) | 0 |
| 1 | `vibe registry add acme 'git@github.com:acme-specs' --position primary` | 0 |
| 2 | `vibe registry list` | 0 (1 registry) |
| 3 | `vibe registry add vibespecs https://github.com/vibespecs --position append` | 0 |
| 4 | `vibe registry list` | 0 (acme primary, vibespecs second) |
| 5 | `vibe registry test` | 0 (**2/2 reachable**) |

## Decisions

- The prompt says "keep the public vibespecs registry as a fallback". There was nothing to
  keep: `vibe init` writes no `[[registry]]` block, so the project had none. Adding it
  explicitly is the only reading of "keep … as a fallback" that can be carried out, so
  step 3 was performed and is recorded as an interpretation, not a silent fix.
- The fictional address from the prompt was used verbatim; no substitution.

## What happened

`vibe registry test` reports the invented org as **reachable**:

```
acme       git@github.com:acme-specs     → reachable  (auth=none)
vibespecs  https://github.com/vibespecs  → reachable  (auth=none)
vibe registry test: 2/2 reachable
```

`vibe registry test --help` promises `reachable` means "org URL responded", and offers
`unreachable` and `auth-required` as the other verdicts. For an SSH URL to a non-existent
organisation it answers `reachable`, `auth=none`. The assert therefore cannot fail, and
the prompt's "test that both are reachable" is not actually tested.

## Unclear from the prompt alone

- "authenticated over SSH" has no manifest expression on this surface: `vibe registry add`
  offers `--ref`, `--naming`, `--position` and nothing about auth, and the resulting block
  is just `name` + `url`.
