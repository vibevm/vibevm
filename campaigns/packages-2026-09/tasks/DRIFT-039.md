# DRIFT-039 — a boot snippet resolves from its own package {#root}

**Finding:** F-103, widened by F-110 (campaign §7 LOG, 2026-07-28).
**Executor:** the reviewer, directly. **Gate:** floor + `progress check
--exhaustive` + `vibe check`. **Landed:** 2026-07-28.

## What was wrong {#what}

Five `world` packages kept their boot snippet at `boot/NN-….md` instead of
`spec/boot/NN-….md`. Every relative link inside those five files pointed at
`../flows/…`, which from `boot/` resolves to `<pkg>/flows/…` — a directory that
does not exist, because the flow documents live at `<pkg>/spec/flows/…`.

**8 of 8 links broken**, resolved one at a time rather than sampled:
`sync-from-code` 3, `git-atomic-commits` 2, `git-autonomy` 1,
`git-conventional-commits` 1, `dev-runtime-docs` 1. The 22 packages using
`vibevm/vibespecs/boot/` had **zero** broken links, so the trait and the defect coincided
exactly.

Separately (F-110), all five READMEs named the snippet as `spec/boot/NN-…`
while their manifests declared `source = "boot/NN-…"`.

## Why the fix is a move and not a link edit {#why}

**The links were already correct for the installed form.** Once `vibe install`
places the snippet at a consuming project's `vibevm/vibespecs/boot/`, `../flows/…` resolves
to `<project>/spec/flows/…` — right. Rewriting them to `../spec/flows/…` would
have fixed the package and broken every consumer.

What was wrong was the **layout**: the in-package path did not mirror the
installed one. Moving the snippet fixes both readings at once, and it makes
F-110 vanish rather than needing its own edit — **the READMEs were right all
along; the layout had never caught up to them.**

Three facts settled the direction:

- **`vibe init` scaffolds `source = "spec/boot/10-tool-{name}.md"`.** The
  tool's own convention is `vibevm/vibespecs/boot/`.
- **22 packages use `vibevm/vibespecs/boot/`, 5 used `boot/`.**
- **`git-attribution-policy` — a member of the same family, renamed by the same
  commit as three of the five — already used `vibevm/vibespecs/boot/`.** The family was
  split against itself.

*Checked and found false before it was written down:* the bare-`boot/` layout is
**not** collateral from the `git-*` rename. `520e7478` carried it forward;
`atomic-commits`, `autonomy` and `conventional-commits` all had it before.

## What changed {#change}

- Five snippets moved `boot/NN-….md` → `spec/boot/NN-….md` (`git mv`, content
  untouched).
- Five `vibe.toml` `[boot_snippet] source` lines updated in the same commit.
- **Nothing else references the path**: only those five manifests, and no Rust
  code anywhere in `crates/` or `xtask/`.

## Acceptance {#acceptance}

- ✅ **8 of 8 relative links resolve in-package**, re-checked one at a time.
- ✅ All five READMEs now agree with their manifests, with **zero README edits**.
- ✅ `vibe check` 0 errors, 0 warnings, 0 info.
- ✅ `progress check --exhaustive` clean over 259 files, 0 unmarked — the moved
  files carry their markup with them.
- ✅ `tools/self-check.sh` exit 0.
