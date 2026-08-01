# WAL — Project Continuation State {#root}

_Updated: 2026-08-01, третий чекпойнт дня (**PHASE D AT ITS EXIT RAMP — волна
11: the owner drained his queue in one sitting; corpus 97.6 %, registry 105
obligations / 228 drifts, 17 owner-ruled deferrals, owed 71 — every one on the
sync route; the whole build-or-demote tail is closed, партія 1d applied, the
carve-out recorded, three new research entries B-022…B-024**)_

##WAL-NUMBERS-COME-FROM-COMMANDS **Every number below is reproduced by two
commands; run them rather than quoting this file** (why:
`spec://vibevm/terraforms/packages-actualization#quick-start` — every line of
the quick-start prints a number; a figure quoted from a checkpoint decays,
proved twice by the registry snapshot reading as open work). @impl/done

```bash
python campaigns/packages-2026-09/tasks/drift-registry.py   # obligations, routes, convergence
python campaigns/packages-2026-09/tasks/summary.py          # what the verdicts say now
```

## Current phase {#current-phase}

##WAL-PHASE **Progress Control (PROP-043) — wave 2, `packages-2026-09`, Phase D
(Stitching), near its exit gate.** Live zone `campaigns/packages-2026-09/`;
`campaigns/progress-2026-08/` is archival and its state files are never pointed
at (`BACKLOG.md` B-010). @impl/done

##WAL-WAVE9 **Волна 9 (2026-07-31) was THE PUBLICATION**, executed through
[`PHASE-D-PUBLICATION-RUNBOOK.md`](../campaigns/packages-2026-09/PHASE-D-PUBLICATION-RUNBOOK.md)
under the owner's «Публикуй»: the event was **local** (lockfile all-local since
2026-07-26, no version bumps — `vibe reinstall --force` re-fetched every pinned
version from `packages/`), the address family took `@spec://` (62 constructs,
25 files, `../flows/` in the compiled lane 69 → 0), the release batch landed
with its lane twins and the three redrawn topology diagrams, and all seventeen
fenced re-derive prompts now name the install slot. Marker fork ruled **(а)** —
the lane carries authoring markup until the `#use spec://… as SOMETHING`
aliasing design (`BACKLOG.md` B-011) makes stripping safe. Verification:
`self-check.sh` EXIT=0, `sync-engines --check` green (51 pairs),
`address-repair.py --verify` 0 remaining. @impl/done

##WAL-STATE **State at the last regeneration** (2026-08-01, after волна 11;
the commands supersede): corpus **11 147 / 228 / 44 — 97.6 %** over 260 files;
registry **105 obligations / 228 drifts — 17 `deferred` by owner ruling, 88
open**; **owed 71 of 228, every one on `sync-from-code`, the owner's route**;
F-207 / F-263 / F-351 resolved to history this wave. The boss-owed remainder
is **zero** on every route; the whole `build-or-demote` tail is owner-ruled
(the rulings ledger:
`campaigns/packages-2026-09/PHASE-D-HOST-OBLIGATIONS.md#rulings-2026-08-01`).
`progress check` clean at the wave-10 checkpoint, `self-check.sh` all green,
cargo-audit 0.22.2 / cargo-outdated 0.19.0 installed. @impl/done

##WAL-RULINGS-IN-FORCE **Owner rulings taken 2026-07-31/08-01 and in force:**
sync-queue group A = answer (2) — «specified, not built» annotate-in-place is
the sanctioned form, the four closing rules amended, group B's 23 corrections
unblocked; B-007 = **B + A′** — **executed 2026-08-01**: the three-question
criterion lives at `spec/design/README.md#owed-a-record`, twelve four-field
records sit inside their owning `spec/common` sections (PROP-000 ×5, PROP-018
×3, PROP-024 ×4), `spec/decisions/` closed in the genre table; the 51 new
record anchors judged confirmed and their four files sealed (batch D14,
2026-08-01); **health-audit adopted whole** («Проведи всё это») — the 16
routed anchors closed by adoption, `AUDIT.md` carries the five clauses, the
skill installed; **партия 1a applied** (PROP-014's nine annotations +
RUNTIME-TRANSPORT, the ten unbuilt mechanisms filed as B-012 with «провести
исследование, можно ли реализовать»); F-220's WAL-entry half = reading **(b)**
(sound-but-unexercised prescription; the package does not move); B-004 =
**(i)** (all seventeen fence first lines repaired in the publication); B-009
closed (the wind-down's step 4 names `cargo xtask mirror` in all three
instruction files); the campaign-plans practice adopted — both live plans
carry the six flow forms and 21 of 29 routed anchors re-judged `confirmed`. @impl/done

## Constraints — do not violate {#constraints}

- ##WAL-C-VERDICT-STANDARD **The verdict standard.** A fact that PRESCRIBES is
  confirmed when coherent and every referent resolves, including
  declared-future ones; a fact that DESCRIBES is checked against the tree; a
  fact whose subject cannot be exercised here is unverifiable in its own
  words. For `world` add source 2 — the host's observed conformance (why:
  `spec://vibevm/terraforms/packages-actualization#world-verdicts`). **§3.8
  bounds source 2:** for `ai-native-lang` packages the audience is external —
  Go/TS are checked only by their own artefacts and tests, Rust is the
  exception (why: batch plan `#audience`, owner ruling 2026-07-31). @impl/done
- ##WAL-C-NON-ADOPTION **Non-adoption is not drift; a marked exception is not
  drift.** Drift is the host's own written contract contradicting the flow, or
  a measurable rule broken over a double-digit share of its window. Each fact
  is judged on its own sentence, never on its family (why: batch plan
  `#which-side` routes (b)/(c); the capability/practice/rule test is §6.1
  `##A-REAL-DEFECT-CONVICTING-THE-WRONG-SENTENCE`). @impl/done
- ##WAL-C-VERDICT-FIRST **A false `confirmed` is repaired verdict-first:**
  re-judge it `drift`, let the registry mint the obligation and assign its
  route, and only then edit. First live test 2026-07-31: the Go GUIDE's
  `gated_packages` clustered to F-166 on the owner's sync route instead of
  landing as an unapproved diff (why:
  `spec://vibevm/terraforms/packages-actualization#log`, the D9-rulings
  entry). @impl/done
- ##WAL-C-STRIKE-PER-ANCHOR **A strike-by-ruling checks each anchor's own
  recorded reason, never the obligation row's** — a row merged by shared
  anchor carries per-anchor reasons, and §B.1's F-189 strike hit two anchors
  the ruling never examined (why: release queue `#stacks-audience`, wave-8
  note). @impl/done
- ##WAL-C-PERIMETER **The perimeter law.** SPEC in `core-ai-native`, ENGINE in
  its five crates (vendored into six siblings), DRIVER in each stack's CLI,
  DEPLOYMENT in a consuming project — and the tree holds at least two adopters
  (the host and the `fractality` specspace). A `not-found` is a fact about the
  search perimeter until the perimeter has been checked; `legacy-spec/**` is
  excluded and is not evidence of practice in either direction (why: batch
  plan `#compliance-blindness`, owner ruling 2026-07-31). @impl/done
- ##WAL-C-READ-FURTHER **Read the document further before searching wider** —
  the cheapest disproof is usually twelve lines down; a document that
  contradicts itself under your reading is telling you the reading is wrong
  (why: batch plan §6.1 `##READ-FURTHER-BEFORE-SEARCHING-WIDER`). @impl/done
- ##WAL-C-OWN-CORPUS **The campaign is inside its own corpus.** Exclude
  `campaigns/*/run/**` from evidence; open `campaigns/`/`spec/terraforms/`
  hits and confirm they are instances, not prose about the finding; any git
  figure names the HEAD it was taken at (why: batch plan §6.1
  `##THE-CAMPAIGN-IS-INSIDE-ITS-OWN-CORPUS`; the string-vs-thing fourth cause
  is the sync queue `#reverify-first`). @impl/done
- ##WAL-C-JOIN-BY-INSTRUMENT **A bulk re-judge goes through the instrument's
  own join, never a substring filter.** Волна 9's naive marker filter matched
  104 anchors — more than double the family — and was discarded; the honest
  join (repaired diff lines → governing anchor via the mirror's fact spans)
  landed at 46, exactly the measured family (why: §7 LOG, волна-9 entry;
  blanket confirmation is the softening §3.6 forbids, by script). @impl/done
- ##WAL-C-WRONG-REASON **A wrong REASON is worse than a wrong verdict** — when
  restating, restate the reason and say which way the correction runs (why:
  batch plan §6.1; the queue's own «716» was a line count read as a commit
  count and self-refuting). @impl/done
- ##WAL-C-CACHE-MERGE-ONLY **`run/cache.json` is mutated by load-and-merge
  only**; never chain `merge-verdicts.py` with `progress seal`; never
  hand-write `verified_at`/`processed_hash` (why: batch plan `#closure` and
  §6.1 `##NEVER-CHAIN-MERGE-AND-SEAL`; seal's UNSAFE note in
  `tasks/merge-verdicts.py`'s own docstring). @impl/done
- ##WAL-C-PROGRESS-WRITES **Every parsing `vibe progress` subcommand writes
  zone state — `check --exhaustive` included — and `--campaign` selects the
  state zone, not the read perimeter.** Always pass `--campaign`; never point
  any of them at `campaigns/progress-2026-08` (why: `BACKLOG.md` B-010, paid
  2026-07-31 — six state files of the closed zone rewritten by a delegated
  «read»). @impl/done
- ##WAL-C-SELF-CHECK-EXCLUSION **No real `vibe` command while
  `tools/self-check.sh` runs** — the floor snapshots the real `~/.vibe` (why:
  the floor's own capture step; turned a green panel red once). @impl/done
- ##WAL-C-STAGE-EXPLICIT **Never `git add -A` while a worker is out**; stage
  explicit paths; commit delegated work on the completion notification, never
  on a filled-in journal (why: `spec://vibevm/terraforms/packages-actualization#log`
  Phase C delegation lessons; a worker's untracked harvest rode along once in
  rehearsal). @impl/done
- ##WAL-C-DURABLE-CITATIONS **A wind-down invalidates any evidence table
  citing `CONTINUE.md` or `spec/WAL.md`** — briefs cite durable files only;
  re-run `verify-evidence.py` before reading any table (why: batch plan §6
  delegation rules; the controlled experiment is 116 dead refs in the one
  pre-rule batch against zero after). @impl/done
- ##WAL-C-SHELL-TRAPS **Shell traps that already fired:** `grep -v '\.vibe'`
  deletes this repo's own `org.vibevm` packages — anchor filters on a path
  segment; PowerShell `-match` is case-insensitive; Python `str.replace` with
  `\n` silently no-ops on this CRLF working copy — use an editor tool that
  errors on a missed match; never trust a substring match about a data file
  (why: each recorded in the §7 LOG the day it bit). @impl/done
- ##WAL-C-BOOT-PAIR **Boot pair marking:** `spec/boot/00-core.md` /
  `90-user.md` carry the owner's machine facts — mark additively, do not
  re-form; `90-user.md` mixes project and machine scope deliberately («оставь
  пока», owner 2026-07-26); `refs/book/` stays NOTOUCH (why: owner rulings on
  record in the boot files themselves). @impl/done
- ##WAL-C-DELEGATION **Delegation goes to the harness's built-in `opus5`
  subagents, not fractality; the verdict and the routing of an anchor are
  never delegated, and neither is review** (why:
  batch plan `#delegation`, owner ruling carried from Phase C §6). @impl/done
- ##WAL-C-MISC **Small standing facts:** the parse payload lives at
  `~/.vibe/progress-cache/…` and never carries a verdict; vvm manifest mtime
  units differ across ports (`mtime_ms` TS / `mtime_nanos` Rust, PROP-019
  §2.15); electron-packager temp cache races on concurrent self-installs —
  run sequentially; `CI`/`VIBE_NO_DEFAULT_REGISTRY` suppresses vibe-embedded
  but not project-local (PROP-030); `crates/vibe-cli/src/registry.rs` is the
  only sanctioned constructor site for embedded/local-composite providers;
  MT-02 and MT-03 await the owner's manual sign-off. @impl/done

## Done (collapsed — see `git log` and the §7 LOG) {#done}

##WAL-DONE **Phase D, waves 1–10 — the whole arc from 601 drift verdicts to
232, 94.3 % → 97.6 %.** Волна 10 (2026-08-01): the three rulings executed
(twelve decision records + criterion, партия 1a, health-audit adoption), the
D13 seal tail closed as 51 judged record anchors (D14), the 260-vs-259
acceptance flag closed by judging the D9 README (D15). Waves 5–8 re-verified every route (18/76, 31/59, 47/171 and
16/40 verdicts did not survive as stated — the four named causes are batch plan
§6.1); волна 9 published. The boss lane closed F-241 and F-148 by building the
malformed-report precision, F-287/F-175/F-303 by correction, the campaign-plans
practice by adoption (21 anchors), and the D9/D10 rulings passes executed the
owner's six decisions of 2026-07-31/08-01. Phase C closed 2026-07-28 at 6 847 /
6 847; Phase B at zero unmarked; wave 1 (`progress-2026-08`) is archival with
`baseline.json` (921 units). @impl/done

## In progress {#in-progress}

##WAL-INFLIGHT **Nothing is in flight.** The two B-012 evidence passes
returned the same day (2026-08-01) and their material landed boss-reviewed:
`harvest/d14-b012-part-A.md` + `part-B.md` (evidence) and
`harvest/d14-b012-prop014-feasibility.md` (the boss synthesis — verdict table
over all ten mechanisms, the four-item Phase-E-ready shortlist, the three
gating keys, four owner decision points). The pass also surfaced and filed
`BACKLOG.md` **B-013** (broken specmap schema-bump route) and **B-014** (host
`specmap.json` drifted ungated, 599/5266 stale spec-side lines), and caught
one false clause in the партія-1a annotation at PROP-014 :56 — repaired
verdict-first as **F-133** (minted D16 → §3.6(a) one-clause fix → confirmed
D17 → sealed; registry back at 108 / 232 with F-133 resolved to history). @impl/done

## Next {#next}

1. ##WAL-NEXT-GROUP-B **Group B state after волна 11 (both sittings):** партия
   1d — applied (F-207, F-263 closed); 1c → research B-022 (F-159 deferred);
   1b — items 1–2 → research B-023, item 3 applied (crash hard-error), item 5
   → build B-025 (mark-not-suppress, owner's visualisation ground), item 6 →
   build B-026 (SARIF ingest, high priority; F-206 deferred naming it), item
   7 applied (the five-sibling closing-rule form) + B-027 filed (the
   marker-semantics audit). **One item remains unruled: the trait-signature
   correction (партия 1b item 4)** — re-presented alongside the owner's
   specmap questions. Then group C. @spec/done
2. ##WAL-NEXT-ADR **B+A′ is fully executed** — the §3.13 carve-out landed as
   (ii): census 35→36, the four-field record at PROP-000 §7
   (`##split-host-decision…`) carrying the owner's updated split-host posture
   of 2026-08-01 (GitHub leads; GitVerse supplementary — source mirror +
   deliberately-published registry storage). Nothing of B-007 remains open. @spec/done
3. ##WAL-NEXT-B012 **B-012 is ruled and drained (2026-08-01, same day):** the
   owner ruled on the whole set — everything builds (`BACKLOG.md`
   B-016…B-021, disposition `planned`; B-018 wide form, high priority),
   security is protocolled and parked (B-015 — nothing until the owner's
   explicit notice, no code triggers), B-012 itself is `done`. The study's
   recommendation column is superseded by its `#rulings` section. Scheduling
   the builds is a later owner call; nothing starts from this entry. @spec/done
4. ##WAL-NEXT-EXIT **Run the exit gate when the queues drain**: registry
   CONVERGENCE (owed → 0 or every survivor owner-ruled — today all 73 owed sit
   on the owner's sync route), `progress check` green over both corpora
   (clean at this checkpoint), `summary.py` arithmetic shown, `baseline.json`
   written (amendment A6), the A–D health-audit inventory (adoption clause).
   The 260-vs-259 file-count discrepancy is **reconciled and closed** —
   judged, not footnoted (D15). @spec/done
5. ##WAL-NEXT-PHASES **Phases T and G are designed and unrun; neither starts
   without an explicit instruction.** Phase E inherits the recorded builds
   (F-211 route-(a) option, B-010's check fix, B-011's aliasing design,
   B-005's ancestry probe, B-012's feasibility verdicts once ruled). @spec/done

## Known issues {#known-issues}

- ##WAL-KI-OPEN **Open on the owner, none blocking:** **F-129** (the wal
  package's two contradictory wind-downs, `@impl/done` ×3 — a release-event
  edit); **F-122** (one `name@version`, two contents, 173 files — Phase B
  marked inside published slots; the публикация of волна 9 re-vendored the
  marked content, which REALISES the install-side half: slots now match their
  packages again); **F-125** (one PLDI'25 measurement, three numbers);
  **F-126** (tcg names a shipped oracle and an unbuilt masker in one family);
  **F-127** (Go prescribes `-race` 15×, runs it 0×); **F-128**
  (`spec/boot/INLINE.md` named by line 5 of the instruction files, `link =
  "inline"` zero occurrences); **F-120** (the kind-line notation's absent
  guide); the **H-роster** (H1–H6 cited ~49× corpus-wide, defined nowhere —
  owner: `core-ai-native/appendix/`, found волной 8 — supersedes F-124's
  framing); **F-069** (aggregator grammar); the `specmap` ratchet's 37 gated
  orphans. @impl/done
- ##WAL-KI-CLOSED-THIS-ARC **Closed since the last checkpoint and worth not
  re-deriving:** F-121 (group A answer (2)), F-136/F-145 (the публикация),
  F-240/F-187/F-213 (волна 8), F-241/F-148 (built), F-303/F-175 («mechanical»
  removed), B-009 (wind-down step 4). F-123's commit-length figure and F-087's
  measurement are wave-6-era numbers — re-measure at HEAD before citing. @impl/done
- ##WAL-KI-BACKLOG **The backlog carries the designed-and-deferred work:**
  B-001 (link tables), B-002 (budget row), B-003 (Go floor fixtures), B-004's
  general fenced-content shape (the 17 first lines are fixed; fences stay
  unaddressable by construction), B-005 (ancestry probe), B-006 (double
  git-family emission), B-007 (ruled B+A′, executing), B-008 (vibe-index
  licence line), B-010 (check-that-writes), B-011 (marker aliasing design),
  B-013 (broken specmap schema-bump route), B-014 (host index drifts
  ungated) — plus the B-012 drain of 2026-08-01: B-015 (security programme,
  parked until the owner's notice), B-016…B-021 (the PROP-014 builds, all
  `planned` by owner ruling; B-018 wide form, high priority) — plus волна
  11's research trio: B-022 (LEDGER-INTENT mechanisms, F-159 waits on it),
  B-023 (JS/TS syntactic tier + Python frontend, two F-146 anchors wait),
  B-024 (do @stage/state markers subsume specmap lifecycle statuses — the
  owner's two-tombstones question), B-025 (mark-not-suppress acknowledged
  deviations, planned), B-026 (SARIF ingest, planned, high priority), B-027
  (the Specified-not-built marker-semantics audit; proposed rule awaits the
  owner). @impl/done

## Session context {#session-context}

##WAL-CTX-BOOT **A cold session starts at the campaign quick-start**
(`spec://vibevm/terraforms/packages-actualization#quick-start`), reads the
batch plan for the phase in flight and §7's LOG from the end, and takes every
number from the two commands at the top of this file. `CONTINUE.md` is the
cold-resume snapshot; this file supersedes it wherever they diverge. @impl/done

##WAL-CTX-ADDRESSABLE **This file's own addressability debt is paid:** every
heading carries an anchor, every constraint entry carries an id and a why
citing a spec anchor or issue, and the in-progress and next entries cite
`spec://` — the three gaps the campaign measured against its own WAL
(`spec/WAL.md:335-337` of the previous revision) and the F-220(b) host
obligation recorded in `run/state/routing.json`. @impl/done
