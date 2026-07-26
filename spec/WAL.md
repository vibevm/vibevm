# WAL — Project Continuation State

_Updated: 2026-07-26 (session end — **wave 1 closed out to zero drift; wave 2
ratified, opened and through Phase A**; seven DRIFT tasks executed)_

## Current phase

**Progress Control (PROP-043) — wave 2, `packages-2026-09`, Phase A closed and
Phase B not started.** The live zone is `campaigns/packages-2026-09/`;
`campaigns/progress-2026-08/` is **archival** (baseline.json, 920 units, intact).

**Ledger: 4 495 confirmed / 0 drift / 3 unverifiable over 275 files.** Floor
green with the user-home tripwire, `progress check` 0, `conform check` 0,
specmap ratchet 37, tree clean, `github/main` = `3dbedba0`, fully pushed.

**Wave 1's last drift row closed, and not the way the plan expected.**
`FACT-GRAIN-EVIDENCE` was believed to need a package re-mint; the blocker was a
**caret**. All three `-lang` stacks required `core-ai-native ^0.7` while the
current version is 0.8.0, and on a 0.x version that caret excludes 0.8.0. Fixed
in place — no new version slot, no publication. specmap went 1 041 → 5 267 spec
units, fact-targeting edges 0 → 65, unresolved edges 77 → 12.

**Wave 2 is ratified with all six §4.5 amendments**, applied to the phase
bodies rather than left in a list. Phase A: scope widened, zone seeded, pilot
run, and three of its own baseline numbers corrected (the ai-native join target
is 703 specmark sites, not 247; 7 of 10 packages carry crates, not 8; 286
observable package files, not 294).

**Next is Phase B** — 16 batches, 217 files, 6 555 facts, plan in
`campaigns/packages-2026-09/BATCH-PLAN.md`. Markup is **hybrid** by owner
ruling: opus marks per package, Fable reviews every diff and keeps splits,
anchor naming and audience. Run `vibe update` once before B1 — it is the
owner's stated re-materialisation path and has never been exercised here.

Key laws unchanged: no fractality, engine pin `claude-opus-5`,
`reality-mismatch` closes only through sync-from-code with owner approval, and
the executor never edits `spec/**`.

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

1. **`vibe update` once**, to retire the one unverified assumption Phase B
   would otherwise stand on.
2. **Phase B, batch B1** — `core-ai-native` live slot, 27 files / 1 487 facts.
   `BATCH-PLAN.md` first; it carries the two rules easiest to lose mid-batch
   (superseded slots are out of scope, and F-069 belongs to Phase C).
3. **Three open findings, all wave 2**: F-069 (aggregator grammar — Phase C's,
   does not block B), F-075 (`seal` refuses on coverage, not recency — owner's
   call), F-077 (two counts of the same thing can disagree).
4. **Phases F and G**, inherited from wave 1 and needing a judgment-marking
   pass and a harvest pass respectively before they can start at all.

## Known issues

- **GitVerse SSH down since 2026-07-25** — banner-exchange timeout, network
  level, not divergence. GitHub carries everything. Recovery: plain
  `cargo xtask mirror`; NEVER `--force`.
- **F-069 — the aggregator grammar gap.** An umbrella's facts are about *other*
  packages, so the document cannot be their source of truth. Phase C's, not
  Phase B's — a marker records stage/state; source-of-truth is a verdict
  question.
- **F-075 — `seal` refuses on coverage, not recency.** Verdict entries carry
  only `v` and `ev` with one date per file, so "every marker has a verdict" is
  checkable and "every verdict is fresh" is not. A session sealing after
  re-deriving one anchor of three hundred will be believed.
- **F-077 — two counts of the same thing can disagree.** `campaign.summary` and
  the verdict maps; recomputed once, nothing keeps them in sync.
- **`-lang` slots carry 0.8.0 engines under 0.7.0 numbers.** Green and working;
  owner ruled obsolete versions are not worked on and versions are not bumped
  per change.
- **vibespecs 401 on this machine** — redbook + rust-ai-native resolve via
  vibe-embedded.
- **specmap ratchet** — 37 gated orphans host-side, unmoved.

## Session context

One long session that closed wave 1 out and opened wave 2. Seven DRIFT tasks
dispatched and reviewed diff by diff; every one of them landed, and two of them
disproved a premise in the task I had written rather than agreeing with it —
that PROP-043 could not be sealed (it can), and that fact ids are unique across
the corpus (they are per file; 316 names live in more than one file, `root` in
168). Both stop rules fired correctly, which is the mechanism working, and the
frequency is the lesson: I wrote "measured" about things I had inferred.

The most expensive discoveries were all the same shape — **a derived thing that
nothing keeps honest.** A caret excluded the version the whole family needed. A
hand-written timestamp landed in the future and silently disabled an
invalidation rule. Three separate stale projections — `tasks.json`,
`campaign.summary`, both findings ledgers — each stated a number that another
part of the same file contradicted. None was caught by a gate; all were caught
by someone recomputing from the source.

The corpus itself shrank by four rounds of subtraction, none of which came from
estimating its size: machine copies, licence boilerplate, derived indexes and
superseded slots came out only because the owner kept asking what the corpus
was made of. Phase B is 6 555 facts rather than the 8 992 first reported, and
the difference is entirely text that nothing resolves to.
