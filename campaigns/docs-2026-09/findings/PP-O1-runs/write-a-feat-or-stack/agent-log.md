# agent-log — write-a-feat-or-stack

Fixture: `workspace-root` · cwd: `work/` · binary: host `target/debug/vibe.exe` (`vibe 1.0.0`)
Isolation: `VIBE_SETTINGS` / `VIBE_REGISTRY_CACHE` / `VIBEVM_SEARCH_CACHE_DIR` inside the
sandbox (native `C:\…` spelling) plus `NO_COLOR=1`; `VIBE_OFFLINE`, `VIBE_UNATTENDED`,
`VIBE_INVOKED_BY`, `VIBE_NO_DEFAULT_REGISTRY`, `VIBETERM`, `VIBEFRAME` unset.

Standing decision for every run: `--invoked-by` was **not** passed. The skill recommends
it, but `--agent-mode` defaults to `auto`, which resolves to `agent` as soon as an
invoked-by value is present — that would change what the lifecycle commands do. The
sandbox rule "the isolation environment does not change behaviour" wins.

**This prompt was edited on the page while the packet was being run.** It was executed
twice: once against the text extracted at the start (`packages/<name>/`, run 1) and once
against the text now on the page (`vibevm/vibepacks/org.acme/<name>/v0.1.0/`, run 2).
Both passed. `prompt.txt` and `asserts.md` here carry run 2, the current text.

## Run 2 — current text (in-tree packages)

| # | command | exit |
|---|---|---|
| 1 | `vibe init package org.acme/welcome-page` | 0 |
| 2 | `vibe init package org.acme/static-site` | 0 |
| 3 | rewrite `…/welcome-page/v0.1.0/vibe.toml` → `kind = "feat"`, `[requires] capabilities = ["ui:page-host"]` | 0 |
| 4 | author `…/welcome-page/v0.1.0/vibevm/vibespecs/boot/10-feat-welcome-page.xml` | 0 |
| 5 | author `…/welcome-page/v0.1.0/vibevm/vibespecs/feats/welcome-page/WELCOME-PAGE.xml` (`#acceptance`) | 0 |
| 6 | rewrite `…/static-site/v0.1.0/vibe.toml` → `kind = "stack"`, `[provides] capabilities = ["ui:page-host"]` | 0 |
| 7 | author `…/static-site/v0.1.0/vibevm/vibespecs/boot/10-stack-static-site.xml` | 0 |
| 8 | author `…/static-site/v0.1.0/vibevm/vibespecs/stacks/static-site/STATIC-SITE.xml` (`#build`) | 0 |
| 9 | `vibe check --path vibevm/vibepacks/org.acme/welcome-page/v0.1.0` | 0 |
| 10 | `vibe check --path vibevm/vibepacks/org.acme/static-site/v0.1.0` | 0 |

`vibe init package <group>/<name>` with no path lands exactly where the current prompt
wants it, so run 2 needed no hand-made directories — only the generated manifest rewritten
and the two specification documents authored.

## Run 1 — text as extracted (member folders under `packages/`)

Same content authored by hand at `packages/welcome-page` and `packages/static-site`;
`vibe check --path packages/welcome-page --quiet` and
`vibe check --path packages/static-site --quiet` both exited 0.

## What the schema probes taught (both runs)

Three probe manifests were fed to `vibe check` to read the accepted key sets out of its
parse errors:

- top-level tables: `project, package, workspace, origin, requires, requires_any,
  provides, obsoletes, conflicts, recommends, suggests, embedded_source, skill, binary,
  mcp_server, hooks, extension(s), mechanism(s), artifacts, deploy, compatibility,
  boot_snippet, features, target, active, llm, registry, mirror, override, visibility,
  i18n, boot, compile`
- `[provides]` accepts exactly `capabilities`
- `boot_snippet.category` accepts `foundation, flow, stack, tool, app, user-override` —
  there is **no `feat`** variant, although `feat` is a package kind

## Decisions

- `category = "app"` for the feat, because the category vocabulary has no `feat`.
  Recorded rather than silently chosen.
- The feat's document carries five acceptance criteria under an anchored `#acceptance`
  section; the stack's carries the build rules under `#build`.
- The generated `10-tool-<name>.md` snippets were deleted and replaced with XML snippets
  whose names state the kind.

## Unclear from the prompt alone

- "requires the capability `ui:page-host`" / "provides `ui:page-host`" give no manifest
  key. `[requires] capabilities = […]` / `[provides] capabilities = […]` were found only
  by reading `vibe check` parse errors — neither `vibe --help` nor any subcommand help
  names them.
- The prompt does not say what markup a specification document uses. XML was chosen from
  an installed package's own boot snippet; the asserts do not pin it either way.
