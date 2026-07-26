# WAL — Project Continuation State

_Updated: 2026-07-26 (session end — **Phase B running, three batches landed;
eleven findings; Phases T and G designed from scratch**)_

## Current phase

**Progress Control (PROP-043) — wave 2, `packages-2026-09`, Phase B in flight.**
Live zone `campaigns/packages-2026-09/`; `campaigns/progress-2026-08/` is
**archival**.

**34 of 202 package files marked; 4 276 unmarked facts remain** (measured, never
decremented). B1+B2 closed `core-ai-native` (943 units / 16 files); B5 closed
`go-ai-native-lang` (665 units / 19 files). Corpus **260 files** after two owner
subtractions. Floor green, `progress check` clean, tree pushed.

**Eleven findings, F-081…F-091, and the through-line matters more than any of
them: three were the measuring instrument being wrong, not the corpus** — the
floor gating a frozen slot, the parser blind to units its own grammar allows,
the sync gate covering four of seven workspaces. Each was green and wrong. **A
green panel reports what was checked and says nothing about what was covered.**

**Next: batch B6** (`typescript-ai-native-lang`, 18 files) — write a thin
`MARKUP-B6.md` beside B5's and dispatch. The 29 locked rulings in
`MARKUP-B1.md` bind it; **rulings 18 and 19 are struck.**

**Two phases were designed and not run:** Phase T (tests by swarm, between E and
F) and Phase G (documentation as a package, after F). Both have full specs;
Phase T also has a worker prompt and an operator runbook.

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

1. **B6** — `typescript-ai-native-lang`, 18 files. Thin brief, dispatch, review,
   gate, commit. Then B7–B16.
2. **Size batches with a ×1.6 multiplier.** `BATCH-PLAN.md`'s `facts` column is
   a *pre-markup scan* count: B5 scanned 411 and finished at 665 units.
3. **Open findings needing a task**: F-092 (`SKILL.md` frontmatter — 9 files,
   one exemption in `blocks.rs`; **renumbered from F-083**, which DRIFT-031 had
   already closed for the task-list gap), F-077's tail (`counters` written from one
   computation, pinned by a test), F-089/090 (PROP-014 against itself).
4. **Open findings needing the owner**: F-087 (17 commit bodies name a model;
   uncleanable without a history rewrite the mirror law forbids), F-088
   (`ATLAS.md` declares a generator that is tracked nowhere), F-078 (DRIFT-035
   is written and deliberately **not dispatched**).
5. **Before Phase G's manifest:** confirm «quick.dev (v2)» means **Qwik**.

## Known issues

- **F-069** — aggregator grammar. Phase C's problem, not Phase B's.
- **F-078** — the boot lane carries four git rules twice. The counter fix is
  necessary and **not sufficient**: `##HOIST-LCA` puts the hoist target at the
  root, which is also the root's own compile site, so the duplicate *migrates*.
  Measured on a fixture, not deduced.
- **F-092** — `SKILL.md` YAML frontmatter cannot carry a fact anchor; 9 files.
  Was filed as F-083, an id already spent; renumbered 2026-07-26.
- **F-087 / F-088** — owner's, see Next.
- **`specmap` ratchet** — 37 gated orphans host-side, unmoved.
- **vibespecs 401 on this machine** — resolution goes through project-local
  `packages/` since `vibe update` repointed it.

## Session context

One long session that ran `vibe update` (the campaign's last unverified
assumption — it held, and repointed the resolve off a stale second working copy
for free), opened Phase B, landed three batches, closed six findings and opened
eleven, and designed two phases that did not exist.

**The pattern worth carrying forward is about this reviewer's own work.** Nine
task files were written; **every executor that ran one found a factual error in
it by measuring rather than trusting it.** Two premises died in a single task.
One correction reached an answer already given to the owner and withdrew it. The
stop rules caught all of them before the tree moved, which is the mechanism
working — but the frequency is the lesson: **a current-state bullet written from
reading is wrong about this codebase far more often than it is right, wherever a
command could have checked it.**

The second pattern is the same shape one level up. **A ledger entry is a claim
with a date, and quoting it is not checking it.** Three stale lines cost real
time, and the worst was in `CLAUDE.md` itself — it said `TASKS.md` did not
exist, three months after it did, and only the editor's read-before-write guard
stopped that claim from destroying the file.
