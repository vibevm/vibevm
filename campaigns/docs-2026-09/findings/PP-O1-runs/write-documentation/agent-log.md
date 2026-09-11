# agent-log — write-documentation

Fixture: `— (waits for phase 2)` · cwd: `work/` · binary: host `target/debug/vibe.exe` (`vibe 1.0.0`)
Isolation: `VIBE_SETTINGS` / `VIBE_REGISTRY_CACHE` / `VIBEVM_SEARCH_CACHE_DIR` inside the
sandbox (native `C:\…` spelling) plus `NO_COLOR=1`; `VIBE_OFFLINE`, `VIBE_UNATTENDED`,
`VIBE_INVOKED_BY`, `VIBE_NO_DEFAULT_REGISTRY`, `VIBETERM`, `VIBEFRAME` unset.

Standing decision for every run: `--invoked-by` was **not** passed. The skill recommends
it, but `--agent-mode` defaults to `auto`, which resolves to `agent` as soon as an
invoked-by value is present — that would change what the lifecycle commands do. The
sandbox rule "the isolation environment does not change behaviour" wins.

## Not run — waits for phase 2

The prompt needs `kind = "doc"`, `[[documents]]` and `vibe doc check --citations`. None
exists in this binary. Asserts executed in an empty sandbox, for the record:

| # | assert | exit |
|---|---|---|
| 1 | `grep -q "kind = \"doc\"" vibevm/vibepacks/org.acme/notes-flow-docs/v0.1.0/vibe.toml` | 2 (no such file) |
| 2 | `vibe doc check --citations --path vibevm/vibepacks/org.acme/notes-flow-docs/v0.1.0` | 2 (`unrecognized subcommand 'doc'`) |

(The page moved this prompt to the in-tree `vibevm/vibepacks/…` layout while the packet was
being run; the asserts above are the current ones.)

Evidence for the schema half:

```
$ vibe check --path <probe with kind = "doc">
unknown variant `doc`, expected one of `flow`, `feat`, `stack`, `tool`, `mcp`, `lang`
in `package.kind`
```
