# VIBEVM-BACKLOG — pilot feedback from fractality

fractality is the pilot use of vibevm (workspace `CLAUDE.md`,
§"vibevm pilot posture"; owner directive 2026-07-09). This file is the
ledger of **deferrable** improvement wishes for vibevm and the packages
under `packages/` that fractality development surfaces: features,
ergonomics, non-blocking bugs. Urgent large bugs are not parked here —
they are fixed in the host immediately, in the session that hit them.

Entry format — newest on top, one dated block per item:

```
## YYYY-MM-DD — <one-line title>
- **What:** the defect / gap / wish, precisely.
- **Why:** what it would buy the pilot (or any consumer).
- **Where it bit us:** the fractality task and file/command that surfaced it.
- **Severity:** wish | paper-cut | bug (non-blocking).
```

Triage happens on the host side, on the owner's word; entries removed
from here should land in a host plan/WAL, not vanish.

## What must land for this workspace to run vibevm "normally"

The definition of "normal": a cold clone runs `vibe install` in
`fractality/v0.1.0/` with **no flags, no vendored copies, no
working-tree gymnastics**, and gets the same 26 packages. Concretely,
in dependency order:

1. **Multi-source resolution** — one resolve walk that consults the
   repo's own `packages/` tree *and* the `[[registry]]` blocks
   (fall-through for published-only members). Shapes that would do:
   `--registry` joining the multi-walk instead of shadowing it, or a
   first-class `[[registry]] url = "file:///…"` / mirror-overlay for
   authoring repos. Until then: the vendored copies + exclusive
   `--registry` recipe in the workspace CLAUDE.md.
2. **Publish the missing versions to vibespecs** (owner-gated): `wal`
   0.2.0 (registry has only v0.1.0; redbook 0.2.0 pins `=0.2.0` — the
   published registry cannot serve edition 0.2.0 at all today),
   `redbook` 0.2.0 itself, `rust-ai-native` family 0.7.0,
   `core-ai-native` 0.7.0, `two-process-model` 0.1.0, and the rest of
   the edition set. Either this **or** item 1 unblocks a cold clone;
   both make it robust.
3. **`vibe registry test` probe fix** — use a group-qualified probe
   pkgref so the diagnostic stops reporting healthy registries as
   `unknown` (0/2 reachable).
4. **Boot-slot collision report** at install/reinstall time (today:
   10 = core-ai-native + wal, 20 = rust-ai-native-lang +
   sync-from-code — silent).
5. **Installed-binary rot story** — the machine `vibe` shim should
   either track the tree (`vibe self install` cadence) or loudly warn
   when the project tree it operates on is newer than itself; the
   stale-parser failure this session was silent misdirection.
6. **`vibe bin build` for this workspace's slot** — build
   `rust-ai-native` in `vibedeps/stack-rust-ai-native-lang/0.7.0/` so
   the discipline CLI runs in its canonical consumer form instead of
   borrowing the host package's `target/`.

## Verification plan — how we prove each item fixed

**Ground rules.** Nothing here deletes or recreates
`org.vibevm.fractality`. Disposable *scratch consumer projects* live
under the session scratchpad (`vibe init` there is free); in-workspace
checks only regenerate what vibe itself owns (`vibe.lock`, `vibedeps/`,
`spec/boot/INDEX.md`, the `<vibevm>` block) via `install`/`reinstall`.
Hand-authored files (`vibe.toml` outside managed edits, specs, crates,
WAL/CONTINUE) are never touched by a verification run. Run every check
with the working-tree binary (`<host>/target/debug/vibe.exe`) unless
the item is *about* the installed shim (item 5).

1. **Multi-source resolution.**
   - Build a *pruned registry copy* in the scratchpad: copy
     `<host>/packages/` minus `org.vibevm/atomic-commits` and
     `org.vibevm/sync-from-code` (recreates today's true upstream state
     without touching the real tree).
   - `vibe init` a scratch project; `vibe.toml`: require
     `flow:org.vibevm/redbook = "^0.2.0"`, the two `[[registry]]`
     blocks, and the new local-source mechanism under test (joined
     `--registry`, `[[registry]] url="file:///…"`, or mirror overlay —
     whatever shape ships).
   - Expected: one `vibe install` resolves all 26 — 24 from the pruned
     local copy, `atomic-commits`/`sync-from-code` from vibespecs;
     `vibe.lock` shows mixed `source_url`s; exit 0, no manual steps.
   - Then in-workspace: after the owner de-vendors the two copies from
     `<host>/packages/` (host-side commit), `vibe install` in
     `fractality/v0.1.0/` re-resolves cleanly with the same 26-package
     lock (content hashes unchanged for the 24 local ones).
2. **Published editions.**
   - `git ls-remote --tags https://github.com/vibespecs/org.vibevm.<name>`
     shows the pinned tags: `wal` v0.2.0, `redbook` v0.2.0,
     `rust-ai-native{,-lang,-mcp}` v0.7.0, `core-ai-native` v0.7.0,
     `two-process-model` v0.1.0 (plus the rest of the edition set).
   - Scratch project with **no** local registry at all (network only):
     `vibe install` of `flow:org.vibevm/redbook@^0.2.0` +
     `stack:org.vibevm/rust-ai-native@^0.7.0` completes; lock's
     `source_url`s are all `https://github.com/vibespecs/…`.
   - Content-hash cross-check: each package's `content_hash` in the
     scratch lock equals the hash in this workspace's `vibe.lock` (same
     bytes from either source — the PROP-002 identity law).
3. **`vibe registry test` probe.**
   - From `fractality/v0.1.0/`: `vibe registry test` reports
     `vibespecs → reachable`; the string `is not group-qualified` never
     appears. GitVerse may report `unreachable`/`auth-required` (that is
     this box's network, item is not about it) — but not the probe bug.
4. **Boot-slot collision report.**
   - `vibe reinstall` in `fractality/v0.1.0/` → the run reports the
     collisions (today: slot 10 core-ai-native+wal, slot 20
     rust-ai-native-lang+sync-from-code) or the packages get re-slotted
     so `grep -oE '/[0-9]+-' spec/boot/INDEX.md | sort | uniq -d` is
     empty. Either way: nothing silent.
5. **Installed-shim rot.**
   - Re-run the original failing case with the *shim*:
     `~/opt/bin/vibe install --registry "<host>/packages" --unattended`
     in a scratch project requiring `stack:org.vibevm/rust-ai-native` —
     the manifest parse error must be gone (shim refreshed), **or** the
     shim refuses/warns loudly that it is older than the tree it
     operates on (whichever story ships).
6. **Slot binary build.**
   - From `fractality/v0.1.0/`: `vibe bin build` (consented) →
     `vibedeps/stack-rust-ai-native-lang/0.7.0/target/release/rust-ai-native.exe`
     exists; `vibe bin exec rust-ai-native -- floor` runs the workspace
     floor end to end with the same verdict as the borrowed host binary;
     then the workspace CLAUDE.md recipe (§"Driving vibevm here") flips
     its step 5 to the canonical form and drops the borrow note.

When an item passes its block, move the entry into the host WAL/plan as
"landed", update the CLAUDE.md recipe accordingly, and delete the entry
here — the backlog holds only open items.

---

## 2026-07-10 — vendored files materialise with CRLF on Windows

- **What:** `vibe install` writes `vibedeps/` (and `.vibe/cache/`)
  file contents with CRLF line endings on Windows even when the source
  registry packages are LF — committing a freshly materialised tree
  produces a wall of git "CRLF will be replaced by LF" warnings and a
  normalize-on-next-touch diff.
- **Why:** byte-stable vendoring (write exactly the source bytes, or
  honor `.gitattributes`) keeps `vibedeps/` diffs meaningful and
  re-installs idempotent for consumers who commit their deps.
- **Where it bit us:** committing
  `packages/org.vibevm.fractality/delegation-rules/v0.1.0/vibedeps/**`
  (IGNITION Phase 5) — sixteen warnings on `git add`.
- **Severity:** paper-cut.

## 2026-07-10 — no re-materialise path for in-place content changes

- **What:** after the RP1 relicense edited published-shape packages in
  place (same versions, new licence text), `vibe install` correctly
  reports "vibe.lock unchanged — nothing to re-resolve" — but there is
  no sanctioned verb to force re-materialising vendored copies from
  the (mutated) sources short of deleting `vibedeps/` slots by hand.
- **Why:** authoring repos DO mutate unpublished coordinates in place;
  a `vibe reinstall --refresh` (or content-hash awareness) would keep
  mirrors honest. Related tension, surfaced to the owner: the
  qualified-naming law says a `name@version` coordinate must never
  mean different content — in-place edits of registry-shaped packages
  sit uneasily with it even locally.
- **Where it bit us:** MT-05's post-merge refresh (IGNITION Phase 6):
  19 stale EULA mirror lines remain in vendored/cache copies until the
  next version bump.
- **Severity:** wish.

## 2026-07-09 — `vibe registry test` probes with an unqualified pkgref

- **What:** `vibe registry test` builds its probe reference as
  `flow:vibe-probe-99zzqq` (no group), which trips the resolver's own
  group-qualification validation — every registry reports `unknown`,
  `0/2 reachable`, even when the registry is fine.
- **Why:** the diagnostic is unusable exactly when you need it (first-time
  registry wiring).
- **Where it bit us:** fractality workspace bring-up, diagnosing the
  redbook resolution failure; `vibe registry test` from
  `fractality/v0.1.0`.
- **Severity:** bug (non-blocking; diagnostic-only).

## 2026-07-09 — no way to combine a local packages dir with `[[registry]]` fall-through

- **What:** `--registry <path>` is an exclusive M0 mode
  (`InstallResolver::Local`, VIBEVM-SPEC §9.1 precedence) — it shadows
  the manifest's `[[registry]]` blocks entirely. An authoring repo that
  hosts most packages locally but consumes a few published-only ones
  cannot resolve both in one walk; `[[mirror]] url = "file:///…"`
  exists but expects the vendor layout, and there is no documented
  "local overlay + network fall-through" recipe.
- **Why:** any monorepo piloting its own packages (this one) hits it on
  the first umbrella package with externally-published members.
- **Where it bit us:** installing `flow:org.vibevm/redbook` for the
  fractality workspace — members `atomic-commits`/`sync-from-code` are
  published on vibespecs but absent from `packages/`. Worked around by
  vendoring both into `packages/org.vibevm/` (they are the owner's own
  published flows, tag-pinned).
- **Severity:** wish (feature gap).

## 2026-07-09 — boot-slot collisions are silent at install time

- **What:** the generated `spec/boot/INDEX.md` carries two slot
  collisions (10: `core-ai-native` + `wal`; 20: `rust-ai-native-lang` +
  `sync-from-code`) and `vibe install` says nothing. Ordering stays
  deterministic, but the slot grid's promise (one snippet per slot) is
  quietly broken.
- **Why:** slot grids are how humans reason about boot order; silent
  collisions rot that map.
- **Where it bit us:** first full redbook + discipline install into the
  fractality workspace (26 packages, 1 node).
- **Severity:** paper-cut.

## 2026-07-09 — machine-installed vibe rots silently (note, no action)

- **What:** the PATH `vibe` (`~/opt/bin/vibe`) failed to parse
  `packages/org.vibevm/rust-ai-native/v0.7.0/vibe.toml` (inline-table
  `[requires] packages = {…}`); the working-tree build parses it fine —
  the bug was already fixed in tree, the installed binary just lags.
- **Why:** recorded as the motivating case for the workspace rule "use
  the working-tree vibe, never the installed one" (CLAUDE.md, pilot
  posture).
- **Where it bit us:** the very first `vibe install` attempt for this
  workspace.
- **Severity:** note (rule already adopted; VVM self-update cadence is
  the owner's call).

## 2026-07-09 — GitVerse over https hangs from this box

- **What:** `git ls-remote https://gitverse.ru/vibespecs/<repo>` hangs
  past 60 s (credential prompt or network); the GitVerse registry as
  resolve fall-through would stall unattended installs on this machine.
- **Why:** affects any resolution walk that reaches the GitVerse block.
- **Where it bit us:** probing for redbook's missing members.
- **Severity:** paper-cut (environmental; GitHub answered fast).
