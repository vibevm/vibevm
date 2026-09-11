# agent-log — install-vibe

Fixture: `— (not sandboxed)` · cwd: `%USERPROFILE%` · binary: host `target/debug/vibe.exe` (`vibe 1.0.0`)
Isolation: `VIBE_SETTINGS` / `VIBE_REGISTRY_CACHE` / `VIBEVM_SEARCH_CACHE_DIR` inside the
sandbox (native `C:\…` spelling) plus `NO_COLOR=1`; `VIBE_OFFLINE`, `VIBE_UNATTENDED`,
`VIBE_INVOKED_BY`, `VIBE_NO_DEFAULT_REGISTRY`, `VIBETERM`, `VIBEFRAME` unset.

Standing decision for every run: `--invoked-by` was **not** passed. The skill recommends
it, but `--agent-mode` defaults to `auto`, which resolves to `agent` as soon as an
invoked-by value is present — that would change what the lifecycle commands do. The
sandbox rule "the isolation environment does not change behaviour" wins.

## Not run

The prompt installs vibe on the machine. Per the packet it is **not executed in a
sandbox**: it would change this machine. Its two asserts were executed on the machine as
it is, against the `vibe` already on `PATH` (`C:\Users\olegc\.vibe\opt\bin\vibe`).

| # | assert | exit |
|---|---|---|
| 1 | `vibe --version` | 0 (`vibe 1.0.0`) |
| 2 | `vibe self doctor` | 0 (`all good.`) |

`vibe self doctor` was run **without** `--fix`; its own help says `--fix` is what applies
changes. A tripwire over `%USERPROFILE%\.vibe` taken immediately before and immediately
after these two commands is byte-identical (46 514 entries both times), so the read-only
claim holds on this machine.

`vibe self doctor` output, verbatim:

```
vibe self doctor
  → ok   git 2.52.0
  → ok   cargo 1.93.1
  → ok   rustc 1.93.1
  → also MSVC Build Tools (Desktop development with C++) — https://visualstudio.microsoft.com/visual-cpp-build-tools/
  → ok   shim dir C:\Users\olegc\.vibe\opt\bin (on PATH)
  → ok   active branch:main #70
  → ok   embedded registry C:\Users\olegc\git\v\vibevm\vibevm\vibepacks (source install; precedence embedded-first)
all good.
```

## Unclear from the prompt alone

- The prompt's `outcome` hard-codes `vibe 1.0.0`. That matches today's binary and will
  rot: the page asserts a version, not a shape.
- "otherwise by building the source checkout with its first-run script" does not name the
  script, so a non-Windows reader has nothing to run.
