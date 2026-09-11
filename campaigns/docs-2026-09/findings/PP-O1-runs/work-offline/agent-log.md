# agent-log — work-offline

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
| 1 | `vibe cache add org.vibevm.world/wal org.vibevm.world/multi-user-planning` | **1** |
| 1a | `vibe cache add org.vibevm.world/multi-user-planning` (isolating the failure) | **1** |
| 1b | *deviation* `vibe cache add --offline org.vibevm.world/multi-user-planning --path <host>` | 0 |
| 2 | `vibe cache list` | 0 (2 packages) |
| 3 | `vibe cache check` | 0 (`2 ok, 0 mismatched, 0 unrecorded`) |
| 4 | `vibe install --offline --assume-yes` | 0 |

## What happened

Step 1 failed on the long package name, not on the network:

```
git clone … org.vibevm.world.multi-user-planning.git …\home\registries\4f0def76f4089d61\packages\org.vibevm.world.multi-user-planning\clone exited with status 128
fatal: '$GIT_DIR' too big
```

`org.vibevm.world/wal` clones into the same tree without trouble; the extra 17
characters of the package name push the clone path past the Windows limit. vibe does not
set `core.longpaths` for the registry cache.

## Decisions

- The packet authorises exactly one fallback here: warm the extra package from the host's
  in-tree registry with `--offline --path <host root>`. Recorded as a deviation.

## Unclear from the prompt alone

- "Fetch into the machine store everything the project needs" has no command. `vibe cache
  add` takes package references, not "whatever this project requires", so the agent has
  to read `[requires.packages]` itself and name each entry.
- The last step proves nothing in this fixture: the project is already installed, so
  `vibe install --offline` answers `vibe.lock is fresh — skipping resolution`. A real
  offline-install proof needs a project whose `vibedeps/` is empty — and that path is
  broken (see the report's anomaly 1).
