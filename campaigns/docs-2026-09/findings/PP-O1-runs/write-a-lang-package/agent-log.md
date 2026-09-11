# agent-log — write-a-lang-package

Fixture: `workspace-root` · cwd: `work/` · binary: host `target/debug/vibe.exe` (`vibe 1.0.0`)
Isolation: `VIBE_SETTINGS` / `VIBE_REGISTRY_CACHE` / `VIBEVM_SEARCH_CACHE_DIR` inside the
sandbox (native `C:\…` spelling) plus `NO_COLOR=1`; `VIBE_OFFLINE`, `VIBE_UNATTENDED`,
`VIBE_INVOKED_BY`, `VIBE_NO_DEFAULT_REGISTRY`, `VIBETERM`, `VIBEFRAME` unset.

Standing decision for every run: `--invoked-by` was **not** passed. The skill recommends
it, but `--agent-mode` defaults to `auto`, which resolves to `agent` as soon as an
invoked-by value is present — that would change what the lifecycle commands do. The
sandbox rule "the isolation environment does not change behaviour" wins.

**This prompt was edited on the page while the packet was being run.** It was executed
twice: once against the text extracted at the start (`packages/sql-style/`, run 1) and once
against the text now on the page (`vibevm/vibepacks/org.acme/sql-style/v0.1.0/`, run 2).
Both passed. `prompt.txt` and `asserts.md` here carry run 2, the current text.

## Run 2 — current text (in-tree package)

| # | command | exit |
|---|---|---|
| 1 | `vibe init package org.acme/sql-style` | 0 |
| 2 | rewrite `…/sql-style/v0.1.0/vibe.toml` → `kind = "lang"`, snippet source, `category = "foundation"` | 0 |
| 3 | delete the generated `10-tool-sql-style.md`; author `10-lang-sql-style.xml` (the three rules) | 0 |
| 4 | author `…/vibevm/vibespecs/langs/sql-style/SQL-STYLE.xml` (idioms, examples, reasons) | 0 |
| 5 | `vibe check --path vibevm/vibepacks/org.acme/sql-style/v0.1.0` | 0 (`clean — every check passed`) |

`vibe init package org.acme/sql-style` with no path creates
`vibevm/vibepacks/org.acme/sql-style/v0.1.0/{vibe.toml,README.md,vibevm/vibespecs/boot/10-tool-sql-style.md}`
— exactly the layout the current prompt asserts, but with `kind = "tool"` and a Markdown
snippet, so both had to be rewritten.

## Run 1 — text as extracted (`packages/sql-style`)

Same content authored by hand at `packages/sql-style`;
`vibe check --path packages/sql-style --quiet` exited 0 and
`grep -q 'kind = "lang"' packages/sql-style/vibe.toml` exited 0.

## Decisions

- `boot_snippet.category = "foundation"`: the vocabulary is
  `foundation, flow, stack, tool, app, user-override` — it has no `lang` variant either.
- The snippet names exactly three rules — name every column; say which join and join on
  keys; keep the query readable top to bottom — and cites the guide by address
  (`@spec://org.acme/sql-style/langs/sql-style/SQL-STYLE#root`). The guide carries the
  three anchored sections the snippet points at (`#columns`, `#joins`, `#shape`), each
  with an example and the reason behind it.

## Unclear from the prompt alone

- The prompt does not say what markup a boot snippet or a guide uses, and `vibe init
  package` produces Markdown while the sibling pages' asserts expect `.xml`. The asserts
  here pin neither, so any layout `vibe check` accepts would pass.
- `kind = "lang"` has no `boot_snippet.category` of its own, so the agent must pick a
  category the vocabulary does allow without any guidance on which one is right.
