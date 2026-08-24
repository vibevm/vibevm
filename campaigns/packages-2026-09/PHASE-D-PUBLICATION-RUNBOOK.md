# Phase D — the publication runbook {#root}

_Prepared 2026-07-31 under the owner's in-session approvals: group A answer (2),
«Публикуй», F-220 → (b), the four D9 form rulings. Every mechanic below was
probed on this tree the same day, not assumed. The event is **local**: the
lockfile has pointed every one of its 36 packages at THIS working copy since
2026-07-26 (`source_kind: local`, `deferrals.md#engine` — «publishing is only
needed for external consumers», and the network registries 401 here). No
version bump is needed: `vibe reinstall --force` re-fetches every package's
content from `packages/` **at the version the lockfile pins**._

## The one open precondition — the marker flood {#marker-fork}

**Discovered while probing, and it is the owner's call, so the run stops here
until it is made.** The boot compiler strips nothing: no marker handling
exists anywhere in the compile path (`boot_artifacts.rs`, `normal.rs`,
`vibe-spec`) — today's `STATIC.md` carries **0** authoring tokens only because
the vendored snippets predate Phase B's markup. A `--force` re-vendor carries
the markup in (§3.5 named this consequence at the phase opening), and the
static lane compiles from it. Measured: the 22 static contributions that map
to canonical sources carry **838 `##ANCHOR` / `@stage/state` tokens over
1 446 source lines** — all of it would land in the lane every session reads
«first and in full».

- **(а) Publish as is.** The tokens are house grammar every agent here reads;
  nothing breaks. Cost: permanent authoring noise in the highest-priority
  lane, and its token budget pays for markup, not content.
- **(б) Teach `bootgen` to strip authoring markup first, then publish.** A
  small host build in `vibe-workspace` (filter `##TOKEN` prefixes and
  `@stage/state` suffixes at compile; the spirit already exists — PROP-035 §7
  ignores directives inside fences), plus a line in the artifact contract and
  tests. The lane stays clean forever, independent of what packages carry.
  Cost: one host code change inside the publication event, reviewed like the
  F-241 build was.

**Recommendation: (б).** A compiled artifact should not carry authoring-side
markup for the same reason it does not execute fenced directives; (а) spends
the lane's budget on noise permanently to save one small reviewed build once.

> **RULED — owner, 2026-07-31: (а), publish as is — and the recommendation
> above was wrong in a way the owner caught.** Naive stripping breaks
> cross-lane resolution: a dynamic module can reference an anchor that existed
> in the source markup and vanished after cleaning. Stripping therefore needs
> an aliasing design first (`#use spec://… as SOMETHING`, with the compiler
> re-loading cleaned markup to resolve) — recorded as **BACKLOG B-011**,
> deliberately deferred. The run proceeds under (а); step 2 below is skipped
> and step 4's fork-(б) check does not apply.

## Approvals held, and what each covers {#approvals}

| approval | covers |
|---|---|
| «Публикуй» (2026-07-31) | the address-family transformation + the drafted release-batch corrections (d8b F-189 ×3, F-190 go/ts; d9 items 1–5 with the four form rulings) + the three `##three-processes-lead` diagrams (flagged «repair in the same diff or ship two topologies per document») |
| group A = (2) | the four F-121 amendments — **already applied and re-judged** the same day |
| NOT covered | sync-queue group B (23 corrections — per-batch presentation still owed), group C singles, the 17-fence product decision (B-004 — unanswered; fences are unanchored, so publishing without them changes no verdict) |

## The steps {#steps}

0. **Gate panel first**: `bash tools/self-check.sh` → expect 0. Working tree
   committed; `git status` clean.
1. **Apply the approved texts** (all in `packages/`, canonical side):
   a. d9 finals: F-153 (6 path lines, `spec/` prefix), F-211 (2 OP-INIT
      bullets), F-188 (3 Motivation lines, rust per ruling (ii) — no PROP-031
      citation), F-251 («four» → «three» ×2).
   b. d8b finals: F-189 ×3 (the superseded-topology rows) + F-190 go/ts (the
      two-clause policy-lines repair; rust copy already landed as F-132).
   c. The three fenced diagrams (`##three-processes-lead` in each stack's
      tools doc) — redraw to the PROP-027 per-family-server topology; no
      anchor moves.
   d. `python campaigns/packages-2026-09/tasks/address-repair.py --apply`
      (62 link constructs, 25 files; refuses if any emitted address fails to
      resolve), then `--verify`.
2. **(if fork = б)** land the bootgen strip + tests, `cargo test -p
   vibe-workspace`, before step 3.
3. **Re-materialise**: `cargo run -q -p vibe-cli --bin vibe -- reinstall
   --force --assume-yes` — refreshes all `vibedeps/` from local sources at
   pinned versions and regenerates the boot artifacts.
4. **Verify the lane**: `grep -c "\.\./flows/" vibevm/vibespecs/boot/STATIC.xml` → **0**
   (today: 69); `address-repair.py --verify` green; `cargo xtask sync-engines
   --check` still green (33 pairs; .md-only edits touch no engine);
   `bash tools/self-check.sh` → 0; fork-(б) only: `grep -c "@status:impl/done"
   vibevm/vibespecs/boot/STATIC.xml` → 0.
5. **Refresh the campaign mirror**: `vibe progress mirror --campaign
   campaigns/packages-2026-09` (the new ts-lang README enters the corpus
   here, unjudged by design).
6. **Re-judge**: the address family (regenerate the join — the boss slice is
   built from `address-repair.py --family` output, one confirmed verdict per
   repaired-link anchor, evidence = the resolving `@spec://` form + the healed
   lane) and the 18 release-batch anchors (F-153 6, F-211 2, F-188 3, F-251 2,
   F-189 3, F-190 2). `merge-verdicts.py --force`, then seal — **never
   chained**.
7. **Regenerate**: `drift-registry.py --write`; `summary.py`; update
   PHASE-D-RELEASE-QUEUE.md (the published rows), sync-queue (address rows),
   §7 LOG (волна 9 — the publication wave).
8. **Commit**, topic-grouped: package texts · vibedeps + boot artifacts (one
   large mechanical commit, named as such) · campaign state · docs.
9. **Roll out**: `cargo xtask mirror` (the B-009-consistent standard rollout;
   fast-forward-only). Fallback on fan-out unavailability: `git push origin
   main`, then mirror when available.

## Rollback {#rollback}

Nothing leaves the machine before step 9. Steps 1–8 revert by `git revert` of
the topic commits plus one `vibe reinstall --force` from the reverted sources
(the same mechanism that applied them). B-005 caution at step 9: `mirror
--check`'s probe tests equality, so a behind-target reads as drift — read the
push output, not the check, when judging success.
