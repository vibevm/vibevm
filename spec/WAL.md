# WAL — Project Continuation State

_Updated: 2026-07-28 (**Phase C — the reviewing debt is CLOSED and `world` batch W1
is CLOSED at 407 of 407; W2…W7 remain**)_

## Current phase

**Progress Control (PROP-043) — wave 2, `packages-2026-09`. Phase C, mid-flight.**
Live zone `campaigns/packages-2026-09/`; `campaigns/progress-2026-08/` is
**archival**.

**3 104 verdicts written, sealed and committed — 45.3 % of the phase.** The
`ai-native` cluster now measures **2 470 confirmed / 207 drift / 20 unverifiable —
91.6 %**, restated from 92.4 % by the reviewing pass below; `world` has its first
**407 verdicts — 368 / 32 / 7, 90.4 %, with 26 self-referential (6.4 %)**. One
command prints all of it: `python campaigns/packages-2026-09/tasks/summary.py`.

**W1 is CLOSED and the phase's falsifiable prediction is in trouble.** §5-C predicted
`world` would measure HIGHER than `ai-native`; the first world batch reads **90.4 %
against 91.6 %**, and the reason inverts the prediction's logic — these flows make
claims about the consuming project, and this consumer is measurable. Thirteen of
W1's thirty-two drifts are one law broken by its own consumer (attribution's
single-place rule, restated in eight further host locations, 88 lines across 50
files), and a second family runs through every package: **69 relative `../flows/…`
pointers in the host's compiled boot lane, all 69 dangling**, because the host has
no `spec/flows/` at all.

**The 138-row reviewing debt is PAID.** Sixty `partial` rows had been sorted by
filename and seventy-eight by one two-branch rule; read individually they come to
**101 confirmed / 36 drift / 3 unverifiable**. Eleven rows left `unverifiable`
correctly and eight moved out of it because their subject was structural, not
behavioural. **The standard that judges a verdict is now written down** — see
§Constraints and the campaign LOG — after four claims turned out to have been judged
two ways across three languages.

**`world` is 16 of 121 files: 3 743 anchors remain, batches W2…W7.** W1's five
delegated tables live in `tasks/evidence/ev-W1{a,b,c,d,e}.json` and its five verdict
batches beside them; all three §3.1 sources for it are captured in
`harvest/world-w1-git-family.md`. The worker contract that produced them —
`tasks/WORLD-WORKER-BRIEF.md` — now carries the ref grammar in full, after W1
returned 43 unresolvable refs of which **not one was a fiction**.

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

Nothing running. The tree is clean and in sync with `origin/main`.

## Next

1. **W2 — `two-process-model`, `wal`, `wal-specspaces`, `sync-from-code`**: 20 files,
   692 anchors, per `tasks/PHASE-C-BATCHES.json`. The recipe is W1's and it works:
   capture the batch's three §3.1 sources into `harvest/`, commission per-package
   evidence tables from `opus5` workers against `tasks/WORLD-WORKER-BRIEF.md`, run
   `verify-evidence.py` BEFORE reading, then `show-rows.py` row by row and judge.
   **Re-measure the per-anchor cost now that W1 has closed** — W2 and W5 are
   provisional at ~695 anchors and split if the cost is higher than C1's.
2. **Every `world` verdict must carry `src`** (a non-empty subset of `[1,2,3]`,
   amendment A2); `merge-verdicts.py` refuses a batch without it, and `src == [1]`
   counts separately as self-referential — `summary.py --batch <id>` prints both.
   W1's own share is 26 of 407, 6.4 %.
3. **Then W3…W7.** 3 743 anchors remain in `world`.
3. **At the phase close:** the X/Y/Z summary in the LOG and the self-referential
   count (both from `summary.py`), and `baseline.json` (amendment A6).
4. **Phases T and G are designed and unrun. Neither starts without an explicit
   instruction.**

## Known issues

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

**The debt's own result is one sentence: no row moved because a worker's evidence
was wrong.** Every drift the reviewing pass found was already visible in the
`searched` field the worker returned; what was missing was someone reading it. A
delegated table that records what it searched converts a reviewer's disagreement
into one lookup — and converts a reviewer's absence into 138 unexamined rows.

**Comparing the twins is what earned the rest.** Four claims appear in two or three
language projections and had been judged differently in each — `vibe codemod
rename-seam`, the product seam's lockfile dispatch, the `complete` latency target,
and the Layer-1 grammar mask. Each is now judged once, and **two of the corrections
run upward**: a document that declares itself «held at stub depth» and
VERY-FAR-FUTURE is not contradicted by being unbuilt.

**The standard had been in use for 690 rows and had never been stated**, which is
exactly how one claim gets two verdicts. It is now in §Constraints, in `CONTINUE.md`
§standard, and in the campaign LOG.

**And one of my own reasons was wrong in a way worth keeping.** C3a recorded the
TypeScript guide's «~74.8 %» as drift because the figure «appears only in
`.vibe/cache/**`». It does not — the live v0.8.0 CONTRADICTION-MAP publishes it in
C-4's own title, one `grep` away, while the ATLAS records 75.3 % / 70.2 % for the
same measurement. The verdict stood; the reason did not, and the real defect is
larger than the one recorded.

**The `ai-native` cluster's result is unchanged and still one sentence: the
discipline gates everything except itself.** Three portable floor steps pass wherever their toolchain is present; every
discipline-specific step fails or is skipped in all six slots, because no package
carries the policy its own gate demands. `core-ai-native` ships library crates only
and has no floor at all.

**F-121 turned out to be a family.** Four mechanism documents — ENGINE-CONFORM,
LEDGER-INTENT, BROWNFIELD and PROP-014 — each close with a rule that unexercised
mechanisms are removed rather than carried as aspiration, each mark it `@impl/done`,
each are contradicted by their own contents, and none is enforced by anything. *The
rule whose job is to keep a document honest is the rule nobody gave a checker.*

**The perimeter was wrong five times and never in a worker's work.** The delegated
`not-found` was accurate every time; the brief that produced it was not. Registries
in the language stack's CLI, PROP-014's deployment in the host, its adoption through
PROP-031's five citations, and the `research/*-demo` consumers — each was invisible
from where the search was pointed. The law is now written into the batch plan with a
literal path list.

**Two of my own errors were caught by the corpus, through batches looking elsewhere.**
A tooling count that included `node_modules` and a dependency cache was read as the
demo's own practice — ten verdicts restated to drift. And fifteen Go verdicts were
recorded `unverifiable` on an absence asserted without checking; `research/go-demo`
exists, carries `go.mod` and 15 production files in the prescribed cell layout, and
twelve of those fifteen became confirmed on it.

**The instruments kept being the thing that was wrong, and each failure was loud.**
Three wrong comparisons preceded the right boot-lane join — a byte compare that was
really the compiler stripping markup, a path derivation off by one component, and a
report diffing raw text where the decision was taken on stripped text. The evidence
checker narrowed its rule three times after reporting honest quotes as fictions,
ending at one general form: *segments between ellipses must appear in order inside
the block*. Across nine delegated tables that is **3 947 refs, 12 unresolvable —
0.3 %**.

**And the phase measured its own repository twice, uncomfortably.** F-122: 173 files
carry different content under a `name@version` a consumer already resolved. F-123:
82 of the last 400 commit subjects break a hard limit this repository ships as a
package — six of them written by this phase while it wrote the measurement.
