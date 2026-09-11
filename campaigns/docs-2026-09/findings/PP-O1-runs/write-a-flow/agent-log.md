# agent-log — write-a-flow

Fixture: `workspace-root` · cwd: `work/` · binary: host `target/debug/vibe.exe` (`vibe 1.0.0`)
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
| 1 | author `packages/review-notes/vibe.toml` (`[package] kind = "flow"`, `[boot_snippet]`) | 0 |
| 2 | author `packages/review-notes/vibevm/vibespecs/boot/review-notes.xml` | 0 |
| 3 | author `…/vibevm/vibespecs/flows/review-notes/REVIEW-NOTES-PROTOCOL.xml` | 0 |
| 4 | `vibe check --path packages/review-notes` | 0 (`clean — every check passed`) |

## Decisions

- `vibe init package org.acme/review-notes packages/review-notes` was **not** used: it
  scaffolds a nested project and puts the package under `vibevm/vibepacks/…`, which is not
  the layout the page's second assert demands
  (`packages/review-notes/vibevm/vibespecs/boot/review-notes.xml`). The prompt says "write
  the boot snippet, the protocol document and the manifest", so all three were authored.
- The manifest shape was taken from what `vibe init package` generates; the XML dialect
  (`<spec xmlns="https://vibevm.org/spec/1">`, `<title id="root">`, `<status>`, `<facts>`
  with named fact elements) from an installed package's own boot snippet.
- The snippet stays under 200 words and names the protocol by address
  (`@spec://org.acme/review-notes/flows/review-notes/REVIEW-NOTES-PROTOCOL#root`);
  the protocol has anchored sections (`#file`, `#entry`).

## Unclear from the prompt alone

- Nothing blocking, but two things are guesses: **where** the package root goes
  (member folder, or under `vibevm/vibepacks/`) and **what markup** a boot snippet uses.
  The assert's filename `review-notes.xml` is the only hint that it is XML, and a reader
  who follows `vibe init package` will produce `10-tool-review-notes.md` instead.
