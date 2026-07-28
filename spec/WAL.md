# WAL — Project Continuation State

_Updated: 2026-07-28 (**Phase C — the `ai-native` cluster is CLOSED, 80 of 80 files;
`world` untouched**)_

## Current phase

**Progress Control (PROP-043) — wave 2, `packages-2026-09`. Phase C, mid-flight.**
Live zone `campaigns/packages-2026-09/`; `campaigns/progress-2026-08/` is
**archival**.

**2 697 verdicts written, sealed and committed — 39.4 % of the phase.** The
`ai-native` cluster measured **2 491 confirmed / 175 drift / 31 unverifiable —
92.4 %** over its 80 files, in six batches: C1 mechanisms 77.1 %, C2 guiding layer
92.4 %, C3 the three language GUIDEs 89.7 %, C4+C5 scaffolds and tcg 97.5 %, C6
skills/boot/READMEs 92.7 %, C7 the `discipline-mcp` trio 100 %.

**`world` is untouched: 121 files, 4 150 anchors, batches W1…W7.** Two of §3.1's
three sources are already mechanised and captured — the source-1 link join reads
**185 citations, 0 broken** over the observed corpus, and the source-2/3 boot-lane
join reads **17 of 31 contributions carrying the package's exact word stream**.

**Exit-gate clause (iii) is satisfied** — the clause wave 1 skipped and paid a
deferred documentation phase for. **39 captured runs** live as files under
`campaigns/packages-2026-09/harvest/`, each `command → real output`.

**The phase's own answer to the mandate, so far: the discipline gates everything
except itself.** No package under `packages/org.vibevm.ai-native/` carries a
`conform.toml` or a `discipline/` directory, so every discipline-specific floor step
fails or is skipped in all six slots, while the three portable steps pass wherever
the toolchain is present.

**Next: the reviewing debt, then `world`.** See §Next. Nothing is running, nothing is
blocked, and the resume prompt is
[`campaigns/packages-2026-09/PHASE-C-RESUME.md`](../campaigns/packages-2026-09/PHASE-C-RESUME.md).

## Constraints — do not violate

- **The perimeter law (new, and it cost five misses).** A mechanism's SPEC lives in
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

Nothing running; the tree is clean and in sync with `origin/main`.

## Next

1. **The reviewing debt — do this before opening `world`.** 138 rows were classified
   in bulk instead of read: 60 `partial` rows in `tasks/evidence/ev-C45-go.json`
   sorted by filename, 78 in `ev-C45-rust.json` sorted by one two-branch rule. A
   `partial` is *related code that does not settle the claim* — the class that carries
   drift. Read each, judge it on its own evidence, restate what moves with
   `merge-verdicts.py … --force`.
2. **The `world` cluster — W1…W7**, 121 files, 4 150 anchors, per
   `tasks/PHASE-C-BATCHES.json`. **Every `world` verdict must carry `src`** (a
   non-empty subset of `[1,2,3]`, amendment A2); `merge-verdicts.py` refuses a batch
   without it, and `src == [1]` counts separately as self-referential. W2 and W5 are
   provisional at ~695 anchors — re-measure the per-anchor cost after the first world
   batch.
3. **At the phase close:** the X/Y/Z summary in the LOG, the self-referential count
   (no shipped command computes it — write the script), and `baseline.json`
   (amendment A6).
4. **Phases T and G are designed and unrun. Neither starts without an explicit
   instruction.**

## Known issues

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

**The cluster's own result is one sentence: the discipline gates everything except
itself.** Three portable floor steps pass wherever their toolchain is present; every
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
