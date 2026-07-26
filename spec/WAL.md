# WAL — Project Continuation State

_Updated: 2026-07-26 (**wave 1 closed out**: the task queue drained, the
baseline artifact exists for the first time, F and G deferred by owner
ruling, and wave 2's plan reviewed for ratification)_

## Current phase

**Progress Control (PROP-043) — wave 1 CLOSED OUT, and the ledger is at
ZERO DRIFT.** The spec tree measures **4 491 confirmed / 0 drift / 3
unverifiable of 4 494 = 99.93 %**. Findings **61 of 69**.
`FACT-GRAIN-EVIDENCE` — the row wave 1 closed out believing no work in this
repository could touch — **closed on 2026-07-26**, and not the way the plan
expected. It needed no re-mint and no publication: all three `-lang` stacks
required `core-ai-native ^0.7` while the current version is 0.8.0, and on a
0.x version that caret excludes 0.8.0. The long-lead item was a caret.
*(An earlier line here read 4 488 / 4 492 — stale by the two anchors sealed
in `812bfecc`. Recounted from the cache.)*

**The task queue is empty.** DRIFT-020, 022 and 023 all landed and were
reviewed diff-by-diff. `bash tools/self-check.sh` exits 0 against the real
`~/.vibe/` — now carrying the user-home tripwire — and `progress check`,
`conform check` and the specmap ratchet are all green at HEAD.

**Close-out is complete.** `campaigns/progress-2026-08/baseline.json`
exists (920 units; 914 confirmed / 2 drift / 4 unverifiable) and
round-trips clean: `rescan` against it reports 919 carried-forward, 0 new,
and 1 changed that is the named-crate invalidation rule firing correctly.
`deferrals.md` is written. The REPORT is filled against all six §8
predictions — four confirmed, one confirmed only because close-out went
and measured it, one falsified in the favourable direction.

**Phases F and G were deferred to wave 2 by owner ruling**, and not for
time: close-out measured their inputs and found them absent. F's three
views are empty (`freeze/plan` 0, `action="rework"` 0, `stage="idea"` 0)
because Phase B recorded what facts *are* and was never asked what should
*happen* to them. G's `harvest/` is empty because Phase C skipped its own
harvest step and its exit gate did not check for it.

**Wave 2 is RATIFIED and Phase A is open** —
`spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md`. All six §4.5
amendments adopted and applied to the phase bodies (Phase C's exit gate now
enumerates five conditions; every §6 prediction names the step that tests it;
a sixth prediction added — *this campaign's stitching introduces zero new
false claims*, which wave 1 would have failed 1 and 1). Two baseline numbers
corrected at ratification: the ai-native join target is **703** specmark
sites, not 247 (that figure was the rust family alone), and **7** of 10
packages carry `crates/`, not 8. Phase A step 1 closed — scope widened, zone
`campaigns/packages-2026-09/` seeded, **344 files / 13 916 facts / 8 997
unmarked**, `progress check` 0 across both corpora, and the expected file
count corrected from 294 to **286** (eight are extractor fixtures the
always-on `fixtures` exclusion drops). **Phase A step 2 — the v0.8.0 re-mint —
was deferred by owner ruling and then turned out not to be needed:** the
blocker was a caret, fixed in place. Phase C is therefore no longer blocked
on an engine. Phase A step 3's pilot found F-068 and F-069 in one 19-line
file.

Key laws unchanged: no fractality (Fable = judgment and ALL review,
Opus = DRIFT execution), engine pin `claude-opus-5`, `reality-mismatch`
closes only through sync-from-code with owner approval.

## Constraints — do not violate


- **mtime unit in the vvm manifest.** TS port stores `mtime_ms`; Rust
  twin stores `mtime_nanos` (PROP-019 §2.15) — account for the unit
  difference when reading both.
- **electron-packager temp cache.** Concurrent `<product> self install`
  runs race on the shared tmpdir template rename — run sequentially.
- **CI-off gate split.** `CI` / `VIBE_NO_DEFAULT_REGISTRY` suppresses
  vibe-embedded but NOT project-local (PROP-030 §5 + §3.3).
- **conform R-001 gate.** `crates/vibe-cli/src/registry.rs` is the only
  sanctioned constructor site for embedded/local-composite providers.
- **Boot pair marking.** `spec/boot/00-core.md` / `90-user.md` carry the
  owner's own machine facts and preferences: mark ADDITIVELY, and prefer not
  to re-form their prose. They left the `NOTOUCH` list on 2026-07-26 (owner
  instruction), together with `VIBEVM-SPEC.md`; `refs/book/` is the one entry
  that remains. A session may edit all three.
- **`spec/boot/90-user.md` mixes project and machine scope, deliberately for
  now.** It is tracked in a public repository and carries this developer's
  workstation facts (`##ssh-auth-lead`, `##GITVERSE-SSH` naming a host,
  `##proven-commands-lead`) alongside genuinely project-scoped ones (the
  multi-homed repo URLs, the split-host posture, the token-path convention).
  Raised 2026-07-26; **owner parked it — "оставь пока, будем переосмысливать
  это когда-нибудь потом".** Do not tidy it unasked.
- **legacy-spec/ is an archive.** Nothing in the living corpus or
  crates may cite into it as a normative source — archive-provenance
  pointers only; the campaign plan in `spec/terraforms/` is the one
  live file still inside a legacy-named path (owner carve-out).
- **Cache campaign maps are load-bearing.** `run/cache.json` carries
  the C-phase verdicts; mutate it by load-and-merge only (scan
  preserves the maps; a from-scratch rewrite would erase them).
- **The parse payload lives outside the repository** since 2026-07-26:
  `~/.vibe/progress-cache/<repo-id>/<branch-slug>/<campaign>/`. It is
  pure acceleration — deleting it is silent and harmless. Never put a
  verdict there.
- **Never trust a substring match about a data file.** `"parsed"`,
  `"verdicts"` and `updated_at` each read as present when they were not,
  in one day. Walk the structure or anchor on bytes. **It struck a third
  time in code**, and inside the campaign's own correction: PROP-043 §7.3
  was made to claim `Baseline::load / store` because a `store` existed —
  on `Cache`, a different type in the same crate (F-065).
- **EVERY parsing `vibe progress` subcommand writes the cache — `check`
  included, and `check` looks read-only.** The `--campaign` flag chooses only
  where state is *written*; the observed scope is always global. Since wave 2
  widened `progress.toml`, ANY such command aimed at wave 1's closed-out zone
  drags all 286 package files into its cache. This happened twice on
  2026-07-26 — once by `scan` (caught, restored) and once by `check`, which
  was NOT caught and got committed in `07a38e1a`; the zone was restored from
  `d3482dd7` and both seals re-applied by hand.
  **RESOLVED 2026-07-26 (owner picked F-073 option (a)):** the wave-2 zone
  `campaigns/packages-2026-09/` now owns the host corpus's verdicts too — all
  58 host verdict maps were migrated into it, so there is **one live zone**
  and a host anchor minted by sync-from-code has a campaign that judges it.
  Wave 1's `run/` is archival from here (its durable artefact is
  `baseline.json`, 920 units, intact; `ZONE-LIFETIMES` already calls `run/`
  disposable after close-out). **Never point a progress subcommand at
  `campaigns/progress-2026-08`.**
- **With two campaign zones, a bare `vibe progress` writes no state.**
  `resolve_campaign` returns a zone only when exactly one exists, and
  otherwise drops to ad-hoc mode — reports still work, state silently does
  not. Always pass `--campaign`.
- **Do not run a real `vibe` command while `tools/self-check.sh` is
  running.** The floor now snapshots the real `~/.vibe` before it builds
  and compares after the test steps (DRIFT-020's tripwire). `vibe progress
  scan` writes into `~/.vibe/progress-cache/`, so a concurrent scan turns
  the floor red — correctly, by the gate's own definition, but confusingly.
  Sequence them: scan first, then the floor.
- **Never hand-write a timestamp into campaign state.** Sealing verdicts by
  hand on 2026-07-26 put `verified_at` 2 and 8.5 hours in the FUTURE on two
  files — plausible-looking values, invented. Direction is what matters:
  `moved_crate` calls a crate moved when its commits are *newer* than the
  verdict, so a future stamp means nothing is ever newer and invalidation
  rule 2 never fires. It fails UNSAFE. Let the tool write it
  (`vibe progress seal`), or it is wrong in a direction nothing checks (F-076).
- **Commit delegated work on the completion notification**, never on a
  filled-in task journal — executors write §9 as they go.
- **Outstanding manual runs (owner sign-off pending):** MT-02
  (`vibe tree` TUI) and MT-03 (`vibe prefs ui`). An agent may pre-run;
  only a person signs off.

## Done (collapsed — see `git log`)

- **Phase D — stitching, complete (2026-07-25/26).** Waves d1 and
  d2a–d2h closed **304 of 311 drift rows** across 36 files. *(This line
  read "310 of 311" until close-out reconstructed the §9 LOG from the
  verdict map: Phase D ended at 7 open rows, and the last 6 were closed by
  Phase E — by building the behaviour the spec already promised, not by
  stitching prose. The two phases are separated precisely so that
  distinction survives.)* d1 took the
  shipped-under-proposed families in one sweep (F-053 PROP-030's 63 rows;
  F-018's bridge four, 137; F-043's PROP-000 twelve) — 191 of those were
  scripted straight off the C-phase verdict map, since a deterministic
  transform beats a re-reading. d2 took the stale headers (22 rows,
  13 files), the design-doc tense family, PROP-003's solver tail
  corrected clause-by-clause, the module index completed to the live tree
  (26 rows added), the MT keymap re-authoring, and the archive's status
  lines. Every row ran through sync-from-code with owner approval.
- **Phase E — coding, queue drained (2026-07-25/26).** DRIFT-006…021
  executed by Opus, each reviewed diff-by-diff; DRIFT-015 superseded
  before it ran. Landed: the specmap evidence join with its report
  column, the lossless-fold check (warning severity — `EXPLICIT-BEATS`
  blesses the divergence a document cannot distinguish from a lying
  fold), the gate panel in `campaign.json`, baseline invalidation's two
  missing rules, blockquote fact anchors, the incremental parse path, the
  `--plain` and resolver-doc corrections, two `deviates` that turned out
  never to have been deviations, the cache split, the no-op-write skip,
  and the removal of the legacy `~/.vibevm` read leg.
- **The test suite stopped reading the developer's home.** F-055, F-056
  and F-057 were one forgotten discipline caught three times by accident.
  Six e2e files now route through a `UserScratch` helper that isolates
  settings, registry cache and search cache together. DRIFT-021 then
  removed the leg no isolation could reach — and found a third read path
  nobody had measured, carrying the vibeterm control-server token.
- **Wave 1 close-out (2026-07-26).** DRIFT-020 made test isolation a
  guarantee rather than a convention — a pre-`main` constructor in the new
  `vibe-test-support` crate, so linkage alone isolates a test process, plus
  a floor tripwire that hashes the real `~/.vibe` and fails if anything
  moved. Both mechanisms were made to fire before either was trusted.
  DRIFT-022 bounded the `[env]` promotion to `VIBE_*`/`VIBEVM_*` on the
  owner's letter (a). DRIFT-023 built `Baseline::store` and
  `vibe progress baseline` — the writer PROP-043 §7.3 had claimed for
  months and nobody had ever built (F-065), which meant §6's monthly
  recurrence had never run end to end. F-063's PROP-002 half landed under
  sync-from-code; PROP-043 gained the two commands it was short (`gate`,
  undocumented since DRIFT-008, and `baseline`). Three new findings filed
  (F-065, F-066, F-067). The plan's §9 LOG regained the entries Phase D
  and Phase E never wrote, and the §11 REPORT was scored against all six
  predictions.
- Earlier: Phase C (2026-07-25, 93.0 % measured), Phase L, Phase B,
  M1.17/M1.18/M1.19.

## In progress

Nothing running; the task queue is empty and the tree is clean.

## Next

1. **Push.** 13 commits from this session are unpushed —
   `github/main` still sits at `e7851a0e`. GitVerse (`origin`) is 125
   behind and its SSH link has been down since 2026-07-25. Recovery is a
   plain `cargo xtask mirror`; **never** `--force`.
2. **Ratify wave 2, or strike its amendments.** The plan carries §4.5's
   six proposals; each is the owner's to take or drop. A1 is the one that
   matters most — the plan today repeats wave 1's exact defect, gating
   Phase C on verdicts while its own §3.2 step says the checker runs are
   captured as doc fixtures.
3. **F-063's owner half.** `spec/boot/90-user.md`
   `##TOKEN-FILE-CONVENTION` still names `VIBEVM_PUBLISH_TOKEN` as the
   highest-precedence token source; `VIBEVM_PUBLISH_TOKEN_<HOST>` outranks
   it. The PROP-002 half landed. That file is user-owned — no session may
   edit it, and the corrected line is in `deferrals.md` §1.
4. **Re-verify two files before trusting anything downstream** —
   `MT-02-vibe-tree-tui.md` and `PROP-026-tcg-tool-family.md` carry
   verdicts formed against text Phase D changed afterwards.
   `vibe progress baseline` names both on every run. Minutes of work.
5. **Open tails now in `campaigns/progress-2026-08/deferrals.md`**, which
   is the authoritative list: F-064 (the second config home), F-065's
   spec half is closed but F-066 (the spec still names the old config
   home) and F-067 (the staleness signal inverts) are open, MT-02/MT-03
   await a human sign-off, and Phases F and G are wave 2's to carry.

## Known issues

- **GitVerse SSH link DOWN since 2026-07-25.** Banner-exchange timeout —
  network-level, not divergence. Recovery: plain `cargo xtask mirror`;
  NEVER `--force`. **GitHub is also behind**: 13 commits unpushed.
- **F-064 — a second config home** (`legacy_xdg_config_path()`,
  `user_config.rs:285`) that `$VIBE_SETTINGS` does not relocate. Same
  shape as the leg DRIFT-021 removed, one severity lower.
- **F-066 — `VIBEVM-SPEC.md` §9.5 still names `~/.config/vibe/config.toml`**
  as the user-level config. That path is only a migration fallback now.
  Close it with F-064, or after: once F-064 deletes the leg, the spec
  line names a path that does not exist.
- **F-067 — the staleness signal inverts.** `processed_hash` is only
  written by a real verify batch, and this campaign sealed verdicts by
  hand throughout, so `progress baseline`'s warning fires on the files
  with the *freshest* verdicts. Cleared by hand for PROP-002/PROP-043;
  standing for MT-02/PROP-026, which genuinely need re-verifying.
- **vibespecs 401 on this machine** — redbook + rust-ai-native resolve
  via vibe-embedded; consuming lockfiles carry `source_kind = "embedded"`.
- **specmap ratchet** — 37 gated orphans host-side, unmoved.

## Session context

The session that closed wave 1 out. Three DRIFT tasks dispatched and
reviewed diff-by-diff, and the review caught what mattered each time: the
tripwire's home resolution had to mirror `home_dir()` exactly or it would
guard the wrong directory, and the baseline's marker snapshot had to be
resolved by the *same* code path `rescan` compares against or every row
would report `marker_diverged` forever.

The lasting lesson is in the REPORT's gap list rather than in the ledger.
**The campaign's own corrections introduced drift, and its own verification
confirmed it.** Phase D authored a `Shipped:` line claiming a
`Baseline::store` that had never existed — the `store` in view belonged to
`Cache`, a different type in the same crate — and Phase C had earlier
sealed five token-precedence anchors `confirmed` on evidence that compared
one spec document against another carrying the identical error. Both were
found by going to the code, and both are now predictions wave 2 should make
about itself rather than surprises it rediscovers.
