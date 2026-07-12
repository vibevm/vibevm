# MT-05 — the dogfood: relicensing the host through the swarm (Phase 6)

_Proves: the boss integration end to end on REAL work — a live boss
session, armed only with the Phase-6 boot snippet and the
`fractality-delegate` skill, routes a real host-repo chore through the
delegation fabric: matrix verdict, packets, a two-worker swarm in git
worktrees of the host repository, boss review, RP1's minimal
acceptance, merge. Also the P6 baseline measurement. Manual-test #5._

**Owner authorization:** RP1 (resolved 2026-07-09) names exactly this
task: «переколбасить все наши EULA на UPL-1.0 … если что-то меняется в
самом корневом VibeVM, верифицируй это … Должна быть хотя бы
минимальная приемка.» This test deliberately crosses into the host
repository — worktrees, branches, and a merge to `main`.

**Paid:** two GLM (`model = "small"`) worker turns. **Isolated:**
scratch `--home` for the fabric; the DELIVERABLE intentionally lands in
the host repo via reviewed merges.

## The target set

Canonical package manifests still carrying the EULA placeholder
(vendored `vibedeps/` and `.vibe/cache/` copies are regenerated
artifacts, refreshed after the merge, not hand-edited):

- `packages/org.vibevm/core-ai-native/v0.7.0` — batch A
- `packages/org.vibevm/rust-ai-native/v0.7.0` — batch A
- `packages/org.vibevm/rust-ai-native-lang/v0.7.0` — batch A
- `packages/org.vibevm/rust-ai-native-mcp/v0.7.0` — batch A
- `packages/org.vibevm/typescript-ai-native/v0.6.0` — batch B
- `packages/org.vibevm/typescript-ai-native-lang/v0.6.0` — batch B
- `packages/org.vibevm/typescript-ai-native-mcp/v0.6.0` — batch B

Per package, two edits: `license = "EULA"` → `"UPL-1.0"` in
`vibe.toml`, and `LICENSE.md` replaced with the UPL-1.0 text (the
fractality package's `LICENSE.md` is the canonical copy; the
`Copyright (c) 2026 Oleg Chirukhin` line stays).

## Steps

1. **Matrix verdict (the skill's step 1).** Error cost: reversible
   (branches + review before merge; a licence FIELD swap the owner
   ordered — the owner decision already happened, RP1). Context:
   compilable (exact file list + exact replacement text). Verify:
   mechanical (grep + diff + host self-check). Size: S–M. → DELEGATE,
   `model = "small"`, scenario 1, two disjoint batches.
2. **Packets.** Two worktree-mode packets against the host repo
   (`repo = <host root>`, `base = main`), each listing its batch's
   exact files, the exact field edit, the full UPL-1.0 replacement
   text, non-goals ("touch nothing else; do not commit"), and
   acceptance `findstr /C:"UPL-1.0" <each vibe.toml>`.
3. **Fire the swarm**: `A=$(fractality spawn …)`, `B=$(…)`,
   `fractality wait $A $B` — both `completed exit=0`, acceptance green.
4. **Boss review + RP1 acceptance, BEFORE merge:**
   - `git -C <wt> diff --stat` per worktree: only the intended files.
   - Diff read: field + licence text, nothing else.
   - `grep -rl 'license = "EULA"' packages --include=vibe.toml`
     inside the worktree, target set → zero (vendored copies excluded).
   - Host floor: `bash tools/self-check.sh` green on the merged tree.
5. **Merge** both `fractality/<id>` branches into `main` (boss commits
   the worktree changes first — workers cannot run git); then refresh
   the regenerated copies: `vibe install` (working-tree vibe, local
   registry) in the three consumers (host root, fractality,
   delegation-rules) so `vibedeps/` mirrors pick up the new licence.
6. **P6 baseline:** eligible grunt tasks in this exercise vs delegated
   through the fabric. Record honestly.
7. Teardown: `mc stop`, remove the scratch home, `git worktree prune`
   in the host repo.

## Recorded run

_(Agent pre-run output is appended below on each execution; the pass is
signed by a human — the pre-run only flags divergence.)_

### Pre-run 2026-07-10 — GREEN on firing #2; findings folded back

**PASS — signed off by the owner, 2026-07-10** (firing #2 + the
phase-2 merge block accepted; F19 disposition and the review findings
acknowledged).

Firing #1 failed before any worker ran: `git worktree add` of the host
repository into a run dir overflowed Windows MAX_PATH (`Filename too
long` on deep `vibedeps/` paths) — **F19**, fixed in provisioning
(`-c core.longpaths=true`); only a real deep repo could have caught
it.

Firing #2 (runs `01KX4RXEGKM23F0WGM76SYT32X` batch A,
`01KX4RXFNF7DSC3CXYJDFWKJW0` batch B):

```text
spawn x2 -> both completed exit=0 (wait exit 0)
EULA left in either worktree (canonical set): 0
manifests carrying UPL-1.0: 4/4 (A), 3/3 (B)
"proprietary" placeholder text remaining in the 7 LICENSE.md: 0
diff surface: only packages/org.vibevm/** licence files (no foreign edits)
```

Boss-review findings (the review loop doing its job):

- The packet's `findstr Universal <LICENSE.md>` acceptance checks were
  **weak**: the OLD placeholder text itself mentions the "Universal
  Permissive License" in its relicensing-intent paragraph, so those
  commands pass either way. The manifest checks (`findstr UPL-1.0
  <vibe.toml>`) were the real gate. Lesson for packet authors:
  acceptance must assert what CHANGED, not what is merely present.
- Two packages (`rust-ai-native` v0.7.0, `typescript-ai-native`
  v0.6.0) already carried UPL LICENSE.md files against EULA manifests
  — a pre-existing manifest/licence-text mismatch (the licensing
  flow's forbidden state); the workers correctly left the
  already-correct files untouched, and the field-only edit closed the
  mismatch.
- One acceptance snapshot (batch A's `rust-ai-native/vibe.toml`)
  reported exit 1 while the artifact was verifiably correct at review
  minutes later — a single unexplained miss, possibly a write-flush
  race between the exiting worker and the pod's immediate acceptance
  run; recorded for observation, not yet an F-number (no second
  occurrence, no diagnosis).

P6 baseline: eligible grunt batches 2, delegated 2 → **100%** (caveat
in the Phase 6 report: the measuring session built the fabric).

**Phase 2 (review → acceptance → merge), same day:**

```text
foreign edits per worktree: 0 and 0   (diff surface = licence files only)
host self-check on the merged-content worktree: EXIT=0 (all green)
merges: 893e314 (rust/core family), 79938ab (typescript family)
grep 'license = "EULA"' over canonical packages/ after merge: 0
```

Post-merge note on the regenerated copies: `vibe install` correctly
refuses to re-materialise unchanged versions (the lockfile did not
move), so `vibedeps/` and `.vibe/cache/` mirrors still carry the old
licence lines until the packages' next version bump — and the host
root's own slot was additionally locked by this session's live MCP
server. Deliberately left as-is: the mirrors are regenerable
artifacts. Pilot observation for vibevm: RP1's in-place relicense of
published-shape coordinates sits in tension with the qualified-naming
law («never reuse a name@version coordinate for different content»);
surfaced to the owner rather than resolved here.

### 2026-07-12 — host root `LICENSE.md` → UPL-1.0 (the last straggler)

The prior firings relicensed the discipline packages; the one EULA left in our
shipped surface was the host root `LICENSE.md` (the "EULA placeholder"). Owner
made the UPL decision final and commissioned this run. One worktree-mode packet
(run `01KXBEHEYJCQ1RNJ5657Q31HVA`, glm / `small`, exit 0, $0.39, ~42 s): the
worker copied the canonical UPL-1.0 text from
`packages/org.vibevm/wal-workspaces/v0.1.0/LICENSE.md` (via Read/Write, no
shell) and appended the third-party/refs note. Boss-verified byte-exact (`diff`
vs canon differed only by the note; no EULA/proprietary text left; only
`LICENSE.md` changed) and merged to `main` as
`chore(license): relicense vibevm to UPL-1.0`. Host crates inherit via
`license-file.workspace`. Evidence:
`reports/trial-results/2026-12-07-18-18-mt05-host-relicense-upl/`.

**Finding — E-BUG-001:** the run reported `acceptance: 0/2 ok`, a false
negative — `findstr /C:"multi word phrase"` lost its quoting in the pod's
acceptance runner (each word parsed as a filename). Filed at
`plans/external/E-BUG-001.md`; the boss-side `diff` + `grep` was the real gate,
confirming this test's own lesson that acceptance is advisory until the diff is
read.
