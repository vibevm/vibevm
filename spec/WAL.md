# WAL — Project Continuation State

_Updated: 2026-07-28 (**Phase C — `flow:wal` is CLOSED at 260 of 260; W2 is half
judged, W2c and W2d remain**)_

## Current phase

**Progress Control (PROP-043) — wave 2, `packages-2026-09`. Phase C, mid-flight.**
Live zone `campaigns/packages-2026-09/`; `campaigns/progress-2026-08/` is
**archival**.

**3 364 verdicts written, sealed and committed — 49.1 % of the phase.** The
`ai-native` cluster measures **2 470 / 207 / 20 — 91.6 %**; `world` now has **667
verdicts — 593 / 59 / 15, 88.9 %, with 30 self-referential (4.5 %)**. One command
prints all of it: `python campaigns/packages-2026-09/tasks/summary.py`.

**`flow:wal` is CLOSED at 260 of 260 — 225 / 27 / 8, 86.5 %** (W2a's four files
81.1 %, W2b's three 90.6 %), and it is the lowest-scoring package in `world` so
far for a structural reason: **this flow's facts describe `spec/WAL.md`, and
`spec/WAL.md` is on disk and measurable line by line.** Six of W2a's sixteen
drifts are the required-sections contract, each measured over the **fourteen most
recent revisions** of the host's WAL — `_Updated:` a bare date in 14 of 14 where
ISO 8601 UTC is required «always and without exception»; Current phase 25-50 lines
against «one or two»; Next 4-5 items against «the single next action», no default
marked; Constraints citing a spec anchor in **4 of 26** entries, with `spec://`
occurring **0 times** in the file in every revision; Session context 41-65 lines of
retrospective against «one-paragraph orientation».

**W1 is CLOSED and the phase's falsifiable prediction is in deeper trouble.** §5-C
predicted `world` would measure HIGHER than `ai-native`; the cluster now reads
**88.9 % against 91.6 %** and is still falling. The reason inverts the prediction's
logic — these flows make claims about the consuming project, and this consumer is
measurable. Two families carry it: thirteen of W1's drifts are attribution's
single-place law broken by its own consumer (88 lines across 50 files), and **69
relative `../flows/…` pointers in the host's compiled boot lane, all 69 dangling**,
because the host has no `spec/flows/` at all.

**F-129 is the batch's own finding, and it is internal to the package.**
`session-end-hook.md` orders «the full hook, steps 1-6»; `cold-resume.md`
§wind-down orders **five** in a different sequence, with no stopping-state step and
no collapse step; and a third fact asserts the two are the same procedure. The host
implements `cold-resume.md`'s five exactly, in order.

**`world` is 23 of 121 files: 3 483 anchors remain — W2c, W2d, then W3…W7.** W1's
and W2's nine delegated tables live in `tasks/evidence/ev-W{1,2}*.json` with their
verdict batches beside them; the three §3.1 sources per batch are captured in
`harvest/world-w{1,2}-*.md`.

**Exit-gate clause (iii) is satisfied** — 39 captured runs under `harvest/`, each
`command → real output`, plus W1's three-source capture. **Clauses (ii) and (iv)
now have a producer**: `tasks/summary.py` prints the X/Y/Z rollup per cluster and
counts `src == [1]` as self-referential, which no shipped command does.

**The phase's own answer to the mandate, so far: the discipline gates everything
except itself.** No package under `packages/org.vibevm.ai-native/` carries a
`conform.toml` or a `discipline/` directory, so every discipline-specific floor step
fails or is skipped in all six slots, while the three portable steps pass wherever
the toolchain is present.

Nothing is blocked. The resume prompt is `CONTINUE.md` §prompt.

## Constraints — do not violate

- **The verdict standard, written down 2026-07-28 after it was applied to 690 rows
  without being stated.** A fact that **PRESCRIBES** what the discipline requires —
  an intent, a participants list, a detector seed, a goal, a tradeoff, an
  alternative, a risk, a routine step — is **confirmed** when it is coherent and
  every referent it names resolves, *including a referent the package itself
  declares as future work* (a card registry's `specified` column, a brief's «vision,
  NOT an implementation plan» line). A fact that **DESCRIBES** what this repository
  already ships or does is **checked against the tree**, and a description that does
  not match is **drift**. A fact whose subject cannot be exercised here is
  **unverifiable, in its own words** — never by a blanket rule over a filename.
  **For `world` there is a fourth clause:** a flow's fact is also checked against
  the host's observed conformance (§3.1 source 2). The host is a living consumer,
  and a law the host breaks is measured, not excused.
- **Never classify a group of rows by one rule to save time.** That is how 138 rows
  became a debt. Every `partial` row is read individually; a `partial` is *related
  code that does not settle the claim*, which is exactly the class that carries
  drift.
- **A wrong REASON in the record is worse than a wrong verdict** — the next reader
  acts on it, and the verdict at least pointed at a real defect. When restating,
  restate the reason too, and say which way the correction runs.
- **The perimeter law (and it cost five misses).** A mechanism's SPEC lives in
  `core-ai-native`, its ENGINE in that package's library crates, its DRIVER in each
  language stack's CLI, and its DEPLOYMENT in a consuming project —
  `research/{rust,ts,go}-demo/`, which carry `conform.toml`, `specmap.toml`,
  `specmap.json` and `discipline/registry/`. A fact can be true at one layer and
  invisible at the other three. **A `not-found` is a fact about the search perimeter
  until the perimeter has been checked.** Full path list: batch plan §4.5.
- **A count that includes `node_modules/`, `.vibe/cache/` or `vibedeps/` is a count
  of somebody else's code.** Ten TypeScript verdicts were confirmed on such a count
  and had to be restated to drift.
- **An absence you assert is not an absence you checked.** Fifteen Go verdicts were
  recorded `unverifiable` on «there is no `research/go-demo`». There is, and it is a
  complete consumer.
- **Delegation goes to the harness's built-in `opus5` subagents, not fractality**
  (owner ruling 2026-07-28). The verdict is never delegated; neither is the review of
  delegated output.
- **Cache campaign maps are load-bearing.** `run/cache.json` carries every C-phase
  verdict; mutate it by **load-and-merge only** — a from-scratch rewrite erases the
  maps and there is no second copy.
- **Never hand-write a timestamp into campaign state.** `verified_at` and
  `processed_hash` are written by `vibe progress seal` and by nothing else; a
  hand-written stamp fails UNSAFE, because `moved_crate` calls a crate moved when its
  commits are *newer* than the verdict.
- **EVERY parsing `vibe progress` subcommand writes the cache — `check` included**,
  and `check` looks read-only. Always pass `--campaign`. **Never point one at
  `campaigns/progress-2026-08`.**
- **With two campaign zones, a bare `vibe progress` writes no state** —
  `resolve_campaign` returns a zone only when exactly one exists. Always pass
  `--campaign`.
- **Do not run a real `vibe` command while `tools/self-check.sh` is running.** The
  floor snapshots the real `~/.vibe` and a concurrent write turns it red.
- **Never `git add -A` (or `git add .`) while a worker is running.** Stage explicit
  paths; read `git status --short` before every commit while a batch is out.
- **A wind-down invalidates any sealed evidence table that cites `CONTINUE.md` or
  `spec/WAL.md`.** Proved 2026-07-28: W2's four tables verified at 3 unresolvable
  when sealed and at **65** one session later, because the wind-down overwrote both
  files. None was a fiction. **Always re-run `verify-evidence.py` before reading a
  table, even one a previous session already verified**, and run
  `repair-refs.py --apply` — it re-points a moved coordinate and refuses to guess
  when the quote occurs twice.
- **`grep -v '\.vibe'` deletes this repository's own packages** — the org namespace is
  literally `org.vibevm`. Anchor such filters on a path segment. Related: PowerShell
  `-match` is case-INSENSITIVE.
- **A Python `str.replace` with `\n` in the pattern silently no-ops on this tree** —
  the working copy is CRLF, the blobs are LF. Use an editor tool that errors on a
  missed match, or anchor on a single line.
- **Never trust a substring match about a data file.** Walk the structure or anchor
  on bytes.
- **Boot pair marking.** `spec/boot/00-core.md` / `90-user.md` carry the owner's own
  machine facts and preferences: mark ADDITIVELY and prefer not to re-form their
  prose. `refs/book/` is the one remaining NOTOUCH entry.
- **`spec/boot/90-user.md` mixes project and machine scope, deliberately for now** —
  owner parked it 2026-07-26: «оставь пока». Do not tidy it unasked.
- **legacy-spec/ is an archive.** Nothing in the living corpus or crates may cite
  into it as a normative source.
- **The parse payload lives outside the repository** since 2026-07-26:
  `~/.vibe/progress-cache/<repo-id>/<branch-slug>/<campaign>/`. Pure acceleration;
  never put a verdict there.
- **Commit delegated work on the completion notification**, never on a filled-in task
  journal.
- **mtime unit in the vvm manifest.** TS port stores `mtime_ms`; the Rust twin stores
  `mtime_nanos` (PROP-019 §2.15).
- **electron-packager temp cache.** Concurrent `<product> self install` runs race on
  the shared tmpdir template rename — run sequentially.
- **CI-off gate split.** `CI` / `VIBE_NO_DEFAULT_REGISTRY` suppresses vibe-embedded
  but NOT project-local (PROP-030 §5 + §3.3).
- **conform R-001 gate.** `crates/vibe-cli/src/registry.rs` is the only sanctioned
  constructor site for embedded/local-composite providers.
- **Outstanding manual runs (owner sign-off pending):** MT-02 (`vibe tree` TUI) and
  MT-03 (`vibe prefs ui`). An agent may pre-run; only a person signs off.

## Done (collapsed — see `git log`)

- **Phase C, the reviewing debt — closed 2026-07-28, 5 commits.** 138 rows read
  individually; the cluster restated 92.4 % → 91.6 %; four cross-language
  inconsistencies resolved to one verdict each; two verdicts restated upward and one
  reason corrected. Five findings opened (F-124…F-128). Two tools added:
  `show-rows.py` (the reviewer's join) and `summary.py` (the exit gate's two
  counts, its `src` arithmetic exercised against a fixture before use).
- **Phase C, the `ai-native` cluster — closed 2026-07-28, 31 commits.** Six batches,
  2 697 verdicts, six campaign tools built with their refusals tested before use, 39
  captured runs, 15 delegated evidence tables persisted into the zone. Seven findings
  opened (F-117…F-123).
- **Phase B — closed 2026-07-27/28 at zero.** `progress check --exhaustive` exits 0
  over all 259 files; 4 276 unmarked facts on the morning of 07-27, none left.
  Sixteen batches, 61 rulings locked, three struck.
- **Wave 1 (`progress-2026-08`) — Phases B, L, C, D, E and close-out**, 93.0 %
  measured at its C gate, 99.8 % after stitching. Its durable artefact is
  `baseline.json` (921 units).
- Earlier: M1.17 / M1.18 / M1.19.

## In progress

**W2 is half judged.** `ev-W2a` (111) and `ev-W2b` (149) are read row by row,
merged, sealed and committed. **`ev-W2c` (179, `two-process-model`, all five files)
and `ev-W2d` (253, `sync-from-code` 5 files + `wal-specspaces` 3) are on disk,
verified and UNJUDGED** — 432 anchors. Nothing needs re-commissioning; start by
reading. Their three §3.1 sources are in `harvest/world-w2-wal-family.md`.

**Two findings from the harvest capture alone are still open on those two files:**
the host's `two-process-model` boot snippet is missing three `{#…}` heading anchors
the package added on 2026-07-27, so three of its four sections cannot be cited (a
stale install, not a changed rule); and both `CLAUDE.md:141` and `SPECSPACES.md:8`
place the specspaces snippet at «slot 11 of `spec/boot/INDEX.md`» where `grep -c`
on that file returns **0** — the same shape as F-128.

## Next

1. **Judge `ev-W2c` (179), then `ev-W2d` (253).** Both already pass
   `verify-evidence.py`; re-run it first anyway, because this session proved a
   wind-down can invalidate a sealed table. Then `show-rows.py --brief` row by row.
   `CONTINUE.md` §recipe is the loop, §standard the verdict standard, and
   §w2-judged what W2a/W2b already settled — including the non-adoption line.
   **Re-measure the per-anchor cost when W2 closes** — W5 is provisional at ~697 and
   splits if the cost is higher than C1's.
2. **Every `world` verdict must carry `src`** (a non-empty subset of `[1,2,3]`,
   amendment A2); `merge-verdicts.py` refuses a batch without it, and `src == [1]`
   counts separately as self-referential — `summary.py --batch <id>` prints both.
   `world`'s share so far is 30 of 667, 4.5 %.
3. **Then W3…W7.** 3 483 anchors remain in `world`.
3. **At the phase close:** the X/Y/Z summary in the LOG and the self-referential
   count (both from `summary.py`), and `baseline.json` (amendment A6).
4. **Phases T and G are designed and unrun. Neither starts without an explicit
   instruction.**

## Known issues

- **Opened 2026-07-28 by W2b, on the owner, not blocking: F-129** — the `wal`
  package ships **two contradictory wind-downs**. `session-end-hook.md` orders «the
  full hook, steps 1-6» (stopping state, rewrite WAL, collapse, overwrite CONTINUE,
  propose commit, report); `cold-resume.md` §wind-down orders **five** in a
  different sequence (overwrite CONTINUE **first**, rewrite WAL, commit, push, chat
  TL;DR) with no stopping-state and no collapse step; and
  `WIND-DOWN-IS-THE-EXPLICIT-FORM-OF-THE-HOOK` asserts they are the same procedure.
  All three `@impl/done`, all three in one package. The host implements
  `cold-resume.md`'s five exactly. Closing it edits a published slot → F-122's
  territory.
- **Opened 2026-07-28 by the reviewing pass, all on the owner, none blocking:**
  **F-124** (three evidence ids — `H4`, `DR1-014`, `DL1-015` — cited across all
  three stacks and resolving in no register; the `H`-series is in daily use inside
  `core-ai-native`'s appendices with no roster anywhere); **F-125** (`core-ai-native`
  v0.8.0 publishes one PLDI'25 measurement twice — ATLAS 75.3 % / 70.2 %,
  CONTRADICTION-MAP 74.8 % — and four documents quote whichever they read; this
  corrected a C3a verdict's reason); **F-126** (`rust-ai-native-tcg` names both a
  shipped consultation oracle and an unbuilt token-level masker; three names in one
  family point elsewhere); **F-127** (the Go stack prescribes `go test -race` 15
  times across 5 documents and passes it 0 times); **F-128** (`spec/boot/INLINE.md`
  does not exist, and line 5 of `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` says the four
  non-negotiable commit rules load first and verbatim from it and are not restated
  elsewhere — `link = "inline"` occurs zero times in every `vibe.toml` in the tree).
- **Open on the owner, and none unblocks Phase C:** **F-120** (the kind-line
  notation — 102 uses, 8 ranks, defined by one example, cited to a
  `GUIDE-SPEC-AUTHORING` that is not in this repository); **F-121** (four mechanism
  documents each end with «any unexercised mechanism is removed», mark it
  `@impl/done`, and are contradicted by their own contents — nothing enforces any of
  them); **F-122** (one `name@version`, two contents, **173 files across 33
  packages** — Phase B marked inside published slots; closing it is a release event
  under §5-D); **F-123** (82 of the last 400 commit subjects exceed the 72-character
  hard limit, 20.5 %); plus the older **F-114**, **F-087 / F-088**, **F-078** and
  PROP-043 §2.
- **F-087 is now measured:** 4 model mentions in 400 commit bodies, **none an
  authorship claim** — two are a colour-theme name, two describe model tiers as
  configuration data.
- **F-117** — the Phase C kick-off documents a `summary` cache field that DRIFT-033
  deleted; a session following it literally would reinstate the defect the ruling
  removed.
- **F-118** — wave 2 ran sixteen batches with no `run/journal.jsonl`; opened at C,
  not back-filled.
- **F-119** — the book's chapter 1 cites `safeharbor.md`, which exists nowhere, in
  both `redbook` slots. Invisible to the gate because the campaign's `exclude` globs
  drop `redbook/*/spec/book/ru/`.
- **F-092** — `SKILL.md` YAML frontmatter cannot carry a fact anchor; 9 files.
- **F-069** — aggregator grammar.
- **`specmap` ratchet** — 37 gated orphans host-side, unmoved.
- **vibespecs 401 on this machine** — resolution goes through project-local
  `packages/` since `vibe update` repointed it.

## Session context

**Open `campaigns/packages-2026-09/tasks/evidence/ev-W2c.json` first** (179 rows,
`two-process-model`), then `ev-W2d.json` (253). Run
`python campaigns/packages-2026-09/tasks/verify-evidence.py <table>` before reading
a word of either, then `repair-refs.py --apply`, then `show-rows.py --brief` row by
row. Avoid touching `CONTINUE.md` or `spec/WAL.md` while judging — those two files
are what the tables cite.

**The line this batch drew, and the rest of `world` will be judged on it.** A flow's
prescription that the host simply never adopted is **not** drift: a human's morning
read leaves no repository artefact, and the flow never claims the host performs one.
Drift is where the host's own **written contract contradicts the flow** — the
cold-start read order, the `_Updated:` format, the Next section's shape. Judged the
other way, one unadopted document would have produced forty-two drifts and buried
the two that matter.

**Two rule pairs were judged differently on purpose, because their own words
differ.** `NEVER-APPEND-TO-THE-WAL` prohibits appending only, and the host never
appends — confirmed. `REWRITE-THE-FILE-DO-NOT-PATCH-OR-APPEND` names patching too,
and `CLAUDE.md`'s step 2 says «Update … bump … refresh» — drift. Each fact is judged
on its own sentence, never on the family it belongs to.

**The instruments failed in a new way and it is now a constraint.** W2's tables were
verified clean and sealed; a wind-down then overwrote `CONTINUE.md` and the WAL's
`_Updated:` line, and the same tables re-verified at 65 unresolvable. Every one was
a real quote whose coordinate the boss moved. `repair-refs.py` recovered 51; the
other 14 are named in the batch commit. Its first real `--apply` also reformatted
every table — fixed to measure the file's own indent, because 51 changes buried in
4 481 is not a diff anyone reads.

**And the phase measured itself again.** The host's WAL breaks six clauses of the
protocol it ships, `spec://` appears zero times in a file whose flow requires spec
anchors, and the instruction «read the WAL before doing anything else» is compiled
at line 1382 of a 1585-line lane the host reads first. None of it was fixed — Phase
C files findings; it does not repair the subject it is measuring.
