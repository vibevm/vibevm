# agent-log — build-package-deploy

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
| 0 | `vibe deploy --help` (note: it does **not** document `--profile`) | 0 |
| 1 | `vibe deploy --plan --profile local` | **1** (`this project declares no deploy profiles`) |
| 2 | `vibe package --help` | 0 |
| 3..8 | six attempts to declare `[deploy.profiles.local]`, each driven by the previous parse error | **1** each |
| 9 | manifest restored to the fixture state | 0 |
| 10 | `vibe deployments` | 0 (`no deployments recorded on this machine`) |

## The declaration attempts, and what each error taught

| attempt | error |
|---|---|
| `targets = []` | `field targets is empty; a profile is a nonempty ordered selection` |
| `[deploy.targets.hello]` | `unknown field targets, expected one of default_profile, target, profiles` |
| `[[deploy.target]]` empty | `missing field id` |
| `name = "hello"` | `unknown field name, expected one of id, artifact, mechanism, provider, when, depends_on, config` |
| `id = "hello"` | `missing field artifact` |
| `id` + `mechanism` | `missing field artifact` |

`artifact` points into a build/package graph that a `vibe init` project does not have, and
neither `vibe deploy --help` nor `vibe package --help` says anything about artifacts,
mechanisms or providers. Per the packet this is where the attempt stops: a working
`local` profile is not reachable from `--help` alone.

## Decisions

- "Stop and ask me before the deploy and before the undeploy" was honoured: no deploy and
  no undeploy were run. The manifest was restored so the asserts run against the honest
  state an agent could reach.

## Unclear from the prompt alone

- The prompt names a profile that the project must already declare, and the page's `needs`
  asks for "a project whose manifest declares build and package targets and a deploy
  profile named `local`". Neither the prompt nor the CLI tells the agent how to write one.
- `--profile` is accepted by `vibe deploy` but absent from its `--help`.
