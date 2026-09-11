# agent-log — read-docs-locally

Fixture: `— (waits for phase 2)` · cwd: `work/` · binary: host `target/debug/vibe.exe` (`vibe 1.0.0`)
Isolation: `VIBE_SETTINGS` / `VIBE_REGISTRY_CACHE` / `VIBEVM_SEARCH_CACHE_DIR` inside the
sandbox (native `C:\…` spelling) plus `NO_COLOR=1`; `VIBE_OFFLINE`, `VIBE_UNATTENDED`,
`VIBE_INVOKED_BY`, `VIBE_NO_DEFAULT_REGISTRY`, `VIBETERM`, `VIBEFRAME` unset.

Standing decision for every run: `--invoked-by` was **not** passed. The skill recommends
it, but `--agent-mode` defaults to `auto`, which resolves to `agent` as soon as an
invoked-by value is present — that would change what the lifecycle commands do. The
sandbox rule "the isolation environment does not change behaviour" wins.

## Not run — waits for phase 2

`vibe doc` does not exist in this binary:

```
$ vibe doc --help
error: unrecognized subcommand 'doc'
[exit 2]
```

`kind = "doc"` is likewise unknown to the manifest schema
(`unknown variant 'doc', expected one of flow, feat, stack, tool, mcp, lang`), so the
package `org.vibevm.core/vibevm-docs` cannot be resolved into the store either.

The one assert that does not need `vibe doc` was executed in an empty sandbox for the
record:

| # | assert | exit |
|---|---|---|
| 1 | `vibe cache list --quiet` | 0 (`store is empty`) |

That assert would pass on any machine, empty store included: it does not check that
`org.vibevm.core/vibevm-docs` is present, which is what the prompt's `outcome` claims.
