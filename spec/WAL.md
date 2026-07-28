# WAL — Project Continuation State

_Updated: 2026-07-28 (**Phase B is CLOSED — B14, B15, B16 landed and the corpus
reached zero**)_

## Current phase

**Progress Control (PROP-043) — wave 2, `packages-2026-09`. Phase B complete.**
Live zone `campaigns/packages-2026-09/`; `campaigns/progress-2026-08/` is
**archival**.

**`progress check --exhaustive` exits 0 over all 259 files — 0 unmarked, 0
warnings.** Every paragraph, list item and non-empty table body cell in the
living corpus carries a stage, a state and an address. Sixteen batches; 4 276
unmarked at the start of 2026-07-27, none left. The corpus is **259 files
(58 host + 201 packages)** — the plan's headline said 260/202 until it was
corrected at the close. `baseline.json` written at the boundary per amendment
A6: 921 units, 917 confirmed, 4 unverifiable, 4 files flagged as moved after
being judged.

**61 rulings are locked** in `MARKUP-B1.md`, **three struck (18, 19, 45)**.
Ruling 45's converse failed twice, so it was struck rather than outranked.

**Findings F-102…F-116 this session.** The two that matter most:

- **F-102** — a fence matched by *prefix* instead of by *run*, in **three**
  parsers. Fixed in `progress-core`, `batch_review` and `vibe-spec`; the first
  two agreed with each other *because* they shared the defect, which is the case
  the "keep the tool independent" argument does not cover. A third
  implementation found it.
- **F-114** — `redbook`'s README promises two projects on the same edition run
  byte-identical practice text; its manifest, three lines above the pins, says
  the cultural-extraction wave is "accumulated here in place" with the edition
  bump deferred. **A normative claim falsified by the manifest that implements
  it**, found only because B16's brief was the first to make an executor read
  the manifest (now ruling 61).

**The sizing rule took its fourth correction, and this one is an explanation
rather than a constant.** The coefficient measures **how aggressively a batch
splits colons**, not any property of the corpus: only a manufactured list can
exceed one unit per terminator. Measured on three consecutive batches — 25
manufactured bullets → +23 units, 10 → +6, 4 → +2. B15 broke the band; B16 was
*predicted in advance* to be insensitive to it and was.

**Both queued wave-level DRIFTs landed 2026-07-28** — F-097, F-103 and F-110 are
closed. Five packages moved their boot snippet to `spec/boot/`, so all 27
`world` packages now share one layout and every in-package link resolves.

**Next: nothing is running.** See §Next — every remaining item needs the owner
or is explicitly off-limits. **Phase C is the plan's next phase and its entry
gate is met**; it is ~11 900 markers without verdicts and was not started.

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
- **`grep -v '\.vibe'` deletes this repository's own packages.** The org
  namespace is literally `org.vibevm`, so that filter drops every
  `packages/org.vibevm.*` path — and reports the absence as an answer. It hid a
  file that was on disk and unchanged, on 2026-07-27, while checking whether an
  exclusion had eaten it. Anchor such filters on a path segment (`/vibedeps/`,
  `/.vibe/`) rather than a substring. Related: **PowerShell `-match` is
  case-INSENSITIVE**, so `-match "DISCOVERY-PROMPT"` matches the lowercase
  directory `discovery-prompt` and inflates a count that `grep` reports as zero.
- **Never `git add -A` (or `git add .`) while an executor is running.** A
  reviewer's own bookkeeping commit swept 13 files of an in-flight batch's
  partial markup into itself on 2026-07-26, which is exactly what the task
  briefs' "do not stage, do not commit" rule exists to prevent — the rule binds
  the reviewer's *command*, not only the worker's intent. Stage explicit paths;
  read `git status --short` before every commit while a batch is out. The
  recovery is `git reset --mixed HEAD~1` (unpushed only; the working tree is
  untouched, so the running worker never notices).
- **A Python `str.replace` with `\n` in the pattern silently no-ops on this
  tree's files.** They are CRLF in the working copy (LF in the blob), so a
  multi-line pattern never matches and the script reports success. It struck
  twice on 2026-07-28 while editing campaign documents. Use an editor tool that
  errors on a missed match, or anchor on a single line.
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

**Phase B is done. Every item below needs the owner or is off-limits — no
batch remains to run autonomously.**

1. ~~Two wave-level DRIFTs.~~ **BOTH LANDED 2026-07-28, surfaced under
   sync-from-code and applied on the owner's approval.** DRIFT-038 (`7b0ec6aa`)
   retired the four dead package names — 50 edits over 29 files, zero live dead
   references left, nine deliberately kept because **a path is not a name**.
   DRIFT-039 (`521bb6cd`) moved five boot snippets `boot/` → `spec/boot/`, which
   made 8 of 8 broken links resolve **and dissolved F-110 with zero README
   edits** — the READMEs had been right all along.
2. **Open findings needing the owner**: F-087, F-088, F-078 (DRIFT-035 written,
   deliberately not dispatched), and PROP-043 §2 — the spec names what a unit
   **is** and never what structure **is**, a boundary two DRIFTs have now moved
   in code. **New this session: F-114** (the redbook edition contract) is a
   release-shaped decision, not an edit.
3. **Phases T and G are designed and unrun. Do not start either without an
   explicit instruction.**
4. **If Phase B ever runs again, size with the band and read a miss as a report
   about colon-splitting.** `1.07–1.15 × terminators + items + cells`, eight
   points spanning 1.021–1.153. **The quantity is terminators under a recorded
   regex, not sentences**; the coefficient is fitted to a 17 % undercount plus
   two artefacts (a quoted question and `(e.g. …)` both fire it), and repairing
   the counter breaks every prediction unless every coefficient is re-derived in
   the same commit.

## Known issues

- **F-113 / F-114 / F-115 / F-116** — `redbook`'s roster counts 21/22/23 in three
  documents; its edition contract is falsified by its own manifest; the
  TypeScript stack's front door points at a README that does not exist; and the
  three `discipline-mcp-*.md` briefs carry twelve divergences, three normative.
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

**Phase B's last three batches, and the phase closed at zero.**

**The instruments were the story again, and the new part is how one was caught.**
F-102 was a fence matched by prefix in *three* parsers. Two of them — the gate
and the review tool — **agreed with each other**, and agreement read as
confirmation. The campaign's standing argument for keeping the tool independent
of `progress-core` rests on their disagreeing; it does not cover the case where
they share a defect. What surfaced it was a third implementation with no stake
in either, reading 130 paragraphs where the gate read 141. *A related lesson, on
a smaller instrument: the first test written to demonstrate the `vibe-spec` half
of that bug **passed against the broken code**, because it asked whether a
section had children and the stray heading was level 1 and therefore the root's.
A probe that passes is evidence about the probe until a control says otherwise.*

**The sizing rule stopped being a curve fit.** It was falsified a fourth time at
B15 and the replacement is not another constant: the coefficient measures how
aggressively a batch splits colons, because only a manufactured list can exceed
one unit per terminator. That is arithmetic, not correlation — and it was then
*predictive* twice, most sharply at B16, whose brief said in advance that this
would be the batch the coefficient could not move.

**Fifteen of twenty-four briefs have carried a factual error found by the batch
running them, and the last three were all counting-unit slips rather than
arithmetic** — names against sites, headline against itemisation. B16's executor
named the pattern; the rule that follows from it is *state the unit with every
count*.

**One ruling was found whose obedience would have been a semantic edit.** Ruling
57 splits a colon introducing cases; ruling 59 now stops it when the cases are
joined by **or**, because a bulleted list is a conjunction and bulleting
`A OR B` asserts both where the source asserts either. In sixty-one rulings it
is the only place following the list produced the thing the list forbids.

**The through-line is that the instruments kept being the thing that was
wrong.** The review tool shipped four bugs and every one was the same mistake —
a rule approximated instead of read: a bullet stripper that ate a `+` from
prose, a shorthand pattern matching `@ts-ignore`, a heading test that also
matched `##ANCHOR`, and a fence check that could not fire. Two were caught only
because the tool and the gate **disagreed**, which is the entire argument for
keeping the tool independent of `progress-core`.

**The sizing rule was locked wrong twice and both times on too few points.** B9
called two measurements «stable to 0.7 %», the plan promoted that to a rule, and
B10 falsified it. B11 replaced it with a mechanism proven by a controlled pair —
two documents identical in every respect but sentence count, whose multipliers
differ by exactly that ratio. B13 then found the quantity is misnamed: it counts
terminators, misses 17 % of real sentences, and the coefficient is fitted to the
undercount. **Calling a measured quantity by the name of the thing it
approximates is an invitation to improve it into wrongness.**

**Fifteen of twenty-one briefs contained a factual error found by the batch that
ran them** — including two of mine in one brief, and one that proved the plan's
own arithmetic wrong. The habit that makes this work is not care; it is that
every number comes from a command and every brief is still checked.
