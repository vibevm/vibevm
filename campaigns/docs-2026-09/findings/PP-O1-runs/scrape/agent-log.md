# agent-log — scrape

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
| 0 | `vibe scrape --help`, `vibe scrape contract --help` | 0 |
| 1 | `vibe scrape --plan` | **1** (`default scrape contract … is absent`) |
| 2 | `vibe scrape --output ../product-clean` | **1** (`absolute path contains a dot component`) |
| 3 | `vibe scrape contract create` | **2** (no such subcommand) |
| 4 | `vibe scrape contract init` | 0 |
| 5 | `vibe scrape --plan` | **1** (`scrape plan is blocked by 23 finding(s)`) |
| 6 | `vibe scrape --output ../product-clean` (verbatim, again) | **1** (same dot-component refusal) |
| 7 | diagnostic, not the prompt: `--output <absolute path>` | **1** (same 23 blockers) |

## What happened

Three separate walls:

1. There is no default contract. `vibe scrape --plan` on a fresh project says
   `not a Vibe project: default scrape contract vibevm/scrape/contract.toml is absent`,
   although the page's `needs` offers "a scrape contract, **or the default one**".
   `vibe scrape contract init` writes one.
2. `--output ../product-clean` — the path the prompt dictates — is refused outright:
   `pinning absent scrape export output …\hello-vibe\../product-clean: absolute path
   contains a dot component`. The relative prefix is never normalised.
3. With a contract in place the plan is blocked by 23 findings, all of the shape
   `modified-policy-refusal [vibe.toml]: Unknown content is refused by its modification
   policy` — every vibe-owned file in a *pristine* `vibe init` + `vibe install` project.
   The project is not a git repository, so the contract cannot classify anything as
   unmodified; nothing in the prompt or its `needs` says scrape wants version control.

Nothing was exported, so `../product-clean` never existed and both asserts pass on an
absence.

## Unclear from the prompt alone

- "prove that the copy builds without vibe" has no meaning for `hello-vibe`: the fixture
  has no native build tool and no sources. The `needs` line asks for "the project's native
  build tool on the `PATH`", which the reader's first project will not have.
