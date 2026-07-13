# 00 — The CORRECTED model (redo v2, owner-directed 2026-07-13/14)

The first pass (branch `cultural-backup` @ `0eb3202`) was **wrong** and was reset away.
This doc locks the corrected model so no future/compacted context repeats the mistake.

## What was wrong in v1 (do NOT do again)

- I **cited existing packages and left the content in the host** ("keep the binding + a
  citation"). The host files (CLAUDE.md, PROP-000, PROP-006, …) stayed fat.
- I **created zero new packages.** Extraction must MOVE content INTO packages — creating
  the new ones the corpus needs.
- I wrote **manual loading prose** in the host — "its boot snippet (slot 30) delivers that
  format to every session". That is exactly wrong: a dependency loads automatically; the
  host must say **nothing** about how boot snippets load.
- I was **too timid on scope** — marked things "feature-stays" (even MFBT) that are in fact
  reusable and deserve their own package.

## The corrected model (owner, across 4 messages)

1. **Every reusable idea → its own package.** Not "vibevm-specific stays". Even
   `move fast and break things` works beyond vibevm → its own package. The named packages
   (git-practices, human-authored-packages, dev-runtime-docs, delegation-first, mfbt, …) are
   **only EXAMPLES — there will be MANY packages.** Enumerate them all via the checklist.
2. **Content lives in the PACKAGE** (single source of truth). The host **section is DELETED**
   or **reduced to a thin stub** (e.g. PROP-016 → "we have two mirrors, github + gitverse" +
   a link; PROP-006 the FILE → deleted).
3. **The package is a real DEPENDENCY** in the host `vibe.toml [requires.packages]`, and it is
   **loaded statically** (`link = "static"` — the default; the boot snippet lands in
   `spec/boot/INDEX.md` and is read every session, forced). Core coding practices load
   statically. `inline` (verbatim into `INLINE.md`) is the sparingly-used priority lane —
   consider only for the very top-priority boot content.
4. **The host says NOTHING about loading.** No "boot snippet delivers…" prose. Declaring the
   dependency IS the delivery mechanism (PROP-009). If a session needs the content, the dep
   provides it — automatically.
5. **Hierarchical families** where a topic has sub-topics (PROP-028 model): e.g.
   `org.vibevm.world/git-practices` is an aggregator whose **members are separate sub-packages
   in its dependencies** — attribution, conventional-commits, group-by-meaning, autonomy, …
6. **The C++ `#include` rule** (PROP-009 §2.1): installing a dep never edits a node's authored
   spec. Host authored text is edited **by us** (delete the extracted section); the dep's
   content arrives via `vibedeps/` + `INDEX.md`, never pasted into the authored file.

## The method — checklist-driven, miss NOTHING (why neworder2 exists)

Use the verified sequential traversal (§3 of the plan) over the WHOLE corpus. For **every**
unit, decide: reusable idea → which package (existing or new-to-create). Build the complete
`concepts.md` = the exhaustive package plan (existing packages to reuse + NEW packages to
author, incl. families). Nothing skipped. Then execute package-by-package.

## The per-package capsule (v2)

For each target package P (existing or new):
1. **Author/extend P**: create `packages/<group>/<name>/v<ver>/` (vibe.toml, README, LICENSE,
   boot/NN-flow-<name>.md, spec/flows/<name>/*.md) with the content **moved out of the host**.
   Template: `packages/org.vibevm.world/atomic-commits/v0.1.0/`. For families, P depends on its
   members (sub-packages).
2. **Delete/thin the host source**: remove the extracted section entirely, or reduce to a
   stub + a plain link (NO loading prose). Delete the whole file where the owner says so
   (PROP-006).
3. **Declare the dependency**: add `"<kind>:<group>/<name>" = { version = "^x", link = "static" }`
   to host `vibe.toml` (or the family aggregator's).
4. **Retarget inbound edges**: code `#[spec]`/`scope!` + prose `spec://…#anchor` citers of the
   removed anchors → the package address. Load-bearing code-cited anchors (e.g.
   `PROP-000#token-secrecy`, `PROP-008#*`, `PROP-012#markers`) must be handled — the content
   moves WITH its anchor to the package, and the code edge is repointed.
5. **Install + gate**: `./target/debug/vibe.exe install --registry packages --assume-yes`
   (**MCP servers must be OFF**), then the gate ladder — specmap 0 suspects/0 warnings,
   `vibe check`, `self-check` exit 0, boot resolves. Commit per package/family (Conventional
   Commits, no AI attribution — Rule 1).

## Git state

- `cultural-backup` @ `0eb3202` — the wrong v1 (preserved, do not build on).
- `cultural-refactor` @ `8831a14` — clean baseline; the v2 redo lands here.
- Rollback tag `pre-cultural-refactor` @ `8831a14`.
- Restored from backup (model-independent): `scope.md`, `oldpacks.md`, `trace-baseline/`.
- MCP servers `rust-ai-native-mcp` / `typescript-ai-native-mcp` are OFF (required for install).

## Guards still in force

- Rule 1 attribution content is EXTRACTED now (owner lifted the guard) → to a package
  (`attribution-policy` exists; owner also named `human-authored-packages` — reconcile). But
  **commit messages / branches never attribute to AI** — that operational rule stands.
- Trio CLAUDE/AGENTS/GEMINI stays byte-identical (sync-engines) — edit all three together.
- Owner-frozen / RO zone (`VIBEVM-SPEC.md`, `spec/boot/00-core.md`, `90-user.md`) — analyze,
  never edit. NOTE: deleting a host anchor these RO files cite would dangle their link →
  such extractions need the citer repointed; if the citer is RO, surface it to the owner.
