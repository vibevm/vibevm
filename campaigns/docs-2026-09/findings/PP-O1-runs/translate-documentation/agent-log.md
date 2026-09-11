# agent-log — translate-documentation

Fixture: `— (waits for phase 2)` · cwd: `work/` · binary: host `target/debug/vibe.exe` (`vibe 1.0.0`)
Isolation: `VIBE_SETTINGS` / `VIBE_REGISTRY_CACHE` / `VIBEVM_SEARCH_CACHE_DIR` inside the
sandbox (native `C:\…` spelling) plus `NO_COLOR=1`; `VIBE_OFFLINE`, `VIBE_UNATTENDED`,
`VIBE_INVOKED_BY`, `VIBE_NO_DEFAULT_REGISTRY`, `VIBETERM`, `VIBEFRAME` unset.

Standing decision for every run: `--invoked-by` was **not** passed. The skill recommends
it, but `--agent-mode` defaults to `auto`, which resolves to `agent` as soon as an
invoked-by value is present — that would change what the lifecycle commands do. The
sandbox rule "the isolation environment does not change behaviour" wins.

## Not run — waits for phase 2

The prompt needs `[i18n] canonical`, `[translates]` and `vibe doc check --translations`.
`vibe doc` does not exist; `[i18n]` is an accepted top-level table, `[translates]` is not
in the accepted set (`project, package, workspace, origin, requires, requires_any,
provides, obsoletes, conflicts, recommends, suggests, embedded_source, skill, binary,
mcp_server, hooks, extension, extensions, mechanism, mechanisms, artifacts, deploy,
compatibility, boot_snippet, features, target, active, llm, registry, mirror, override,
visibility, i18n, boot, compile`).

Asserts executed in an empty sandbox, for the record:

| # | assert | exit |
|---|---|---|
| 1 | `grep -q "canonical = \"ru\"" vibevm/vibepacks/org.acme/notes-flow-docs-ru/v0.1.0/vibe.toml` | 2 (no such file) |
| 2 | `vibe doc check --translations --path vibevm/vibepacks/org.acme/notes-flow-docs-ru/v0.1.0` | 2 (`unrecognized subcommand 'doc'`) |

(The page moved this prompt to the in-tree `vibevm/vibepacks/…` layout while the packet was
being run; the asserts above are the current ones.)
