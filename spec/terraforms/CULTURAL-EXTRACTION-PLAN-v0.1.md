# Cultural-Pattern Extraction — Autonomous Refactoring Plan v0.1

**status: EXECUTABLE · launch under `/goal` and walk away · pauses cleanly at the 80% context frame and resumes via `восстанови сессию` · the *bootstrap* refactoring of `REFACTORING-ENGINE-META-PLAN` — it precedes the engine and generates its requirements from lived experience**

> **How to run this.** You (the executing model) are launched under `/goal` with this file as your operating manual. Boot the normal way first (`CLAUDE.md` → `spec/boot/INDEX.md` → its files → `spec/WAL.md` → `CONTINUE.md`), then read this whole file, then execute §2's lifecycle in order. You run **autonomously**: no human is watching mid-run, so there are no "confirm you understood" checkpoints — instead you do the one-time comprehension check in §1.4. The only interruptions are session restarts at the 80% context frame (§5). **The boot prompt must ALWAYS resolve** — between restarts much can change, and a session that cannot boot cannot resume.

---

## 0. Prime directives {#directives}

1. **Analyze every in-scope file. Skip nothing in-scope.** "In-scope" is defined precisely in §4.1 — it is *not* literally every file (journals, generated blocks, regenerated vendor trees, and the read-only zone are excluded, with reasons). Within the in-scope set, missing one file is a defect.
2. **Ideal quality, no shortcuts.** We spend whatever time and tokens it takes. The work is observed and steered from outside; *doing* the work is cheaper than inventing ways to avoid it. There is no "MVP" version of this.
3. **Go to the end — and a clean pause IS reaching the end.** We run until done *or* until the 80% context frame (§5). Pausing at 80% to checkpoint and hand off to the next session is **completion-by-checkpoint, not failure or a shortcut.** Never compact; checkpoint and stop.
4. **The boot prompt must always resolve** (§4.9 gate). Every change that touches boot, packages, dependencies, or the manifest is followed by a boot-resolves check, so any restart boots cleanly.
5. **Do NOT delegate.** No agent swarm, no fractality. This is architecture, merge judgment, and ambiguity-that-is-design — the never-delegate set. One boss-side session, at maximum reasoning, restarting on overflow.
6. **Safety first, always on branch `refactor`** (§1.1). Nothing lands on `main`.

## 1. Safety envelope — do this once, first {#envelope}

### 1.1 Branch + rollback anchor
Create and switch to branch `refactor` **now**; tag the current head `pre-cultural-refactor` as the rollback anchor. All work happens on `refactor`; `main` is never touched by this run.

### 1.2 Durable state layout (`neworder2/`)
All process state lives under `neworder2/` (git-tracked, on `refactor`), so a restart is lossless:
- `neworder2/scope.md` — the in/out manifest (§4.1).
- `neworder2/oldpacks.md` — the existing-packages index (§4.2).
- `neworder2/concepts.md` — the **concept registry**: concept → target package → source files → merge notes (§3, the dedup axis).
- `neworder2/allspecs.md` — the source-file control table (§4.4).
- `neworder2/trace-baseline/` — the "before" snapshot of the spec↔code graph (§4.3).
- `neworder2/notes/` — one notesfile per source spec, **mirroring the `spec/` tree** to avoid name collisions (§4.4).
- `neworder2/memory/` — any plan, found bug, or insight you want to record during the freeze goes here (never into a spec).
- `neworder2/report.md` — the final analytical report (§4.7).

### 1.3 The 80% frame + restart protocol
Baked into every per-file loop (§5). Do **not** rely only on your own context estimate — **commit after every capsule** so progress is durable even if compaction or overflow strikes mid-file.

### 1.4 One-time comprehension check (anti-drift)
Before the mass run, **restate this plan's lifecycle and the capsule protocol in your own words** into `neworder2/memory/00-understanding.md`, then **execute ONE trial capsule end-to-end** (pick the single clearest cultural pattern — e.g. Conventional Commits) through §4.5–4.6 and confirm it passes the full gate ladder (§4.9). Only then start the mass run. This catches a misunderstanding on one capsule instead of seventy.

## 2. The lifecycle {#lifecycle}

Execute in order. Phases 1–7 are the **autonomous `/goal` run**. Phases 8–9 are **manual-trigger** follow-ups (run in their own sessions on the owner's word), not part of the walk-away run.

1. **Scope manifest** (§4.1) — *added: define the in/out set before touching anything.*
2. **Collect existing packages** (§4.2) — index what already exists (dedup input).
3. **Trace baseline** (§4.3) — *added: snapshot the spec↔code graph, so we can prove edges intact after.*
4. **Collect source files** (§4.4) — build `allspecs.md`.
5. **Restricted freeze + analysis** (§4.5) — markers + notesfiles; no semantic spec edits.
6. **Distill + refactor** (§4.6) — unfreeze packages; capsule moves, gated.
7. **Report** (§4.7).
8. **Conflict resolution** (§4.8) — *manual trigger; philosophical; may span sessions.*
9. **Marker cleanup** (§4.9-cleanup) — *manual trigger.*

## 3. The verified sequential traversal — the engine of every phase {#traversal}

The generalized pattern used throughout:
1. Obtain the list.
2. Create a control file with an `object` column and one or more status columns.
3. Walk the list; process each object per the phase's instructions.
4. Mark each object's status **only when that status's work is FULLY done** (atomicity — see below).
5. When the list is exhausted, verify every object carries the status; **loop over any that do not** until all are marked.

A status is set **atomically**: e.g. a file's `analyzed` is set **only when BOTH its in-spec markers AND its notesfile are complete**. Re-entry after a restart re-reads the file, finishes what is missing, and does **not** duplicate work. Multiple status columns mean multiple sub-phases; the process is complete only when every column is filled for every object.

## 4. Phase procedures {#procedures}

### 4.1 Scope manifest — `neworder2/scope.md` {#scope}
Write the in/out set with a reason for every exclusion. Three zones:

- **In-scope, editable** — the authored spec corpus: `spec/common/**`, `spec/modules/**`, the **authored** parts of `spec/boot/**` (the prose outside the generated `<vibevm>` blocks), and the root markdown (`CLAUDE.md`, `AGENTS.md`, `GEMINI.md`, `README.md`, `MEMORY.md`, `SPECSPACES.md`, `CONTRIBUTING`-class files).
- **In-scope, READ-ONLY** — analyze for pattern awareness, **never edit or extract from**: `spec/boot/00-core.md` and `spec/boot/90-user.md` (owner-owned), `VIBEVM-SPEC.md` (owner-frozen), and `refs/book/**` if referenced (the owner's book). Reading them informs classification; touching them is forbidden.
- **Out-of-scope** — do not analyze, do not touch: `refs/**` (third-party + the book — forbidden zone), `vibedeps/**` and `**/.vibe/cache/**` (regenerated — any edit is wiped by `vibe install`), `spec/WAL.md` (a 456 KB session journal, *state* not a pattern source — the freeze still forbids semantic edits, but it is not analyzed), the generated blocks (`<vibevm>…</vibevm>`, `spec/boot/INDEX.md` "generated by vibe"), `spec/terraforms/**` and `spec/research/**` (process/plan artifacts, including THIS plan), and `neworder2/**` (our own workspace). Fractality log files under `packages/**` are out (large machine logs, not specs).

"Skip nothing" (§0.1) means: skip nothing in the **in-scope** set. Acceptance: `scope.md` lists every zone with reasons.

### 4.2 Collect existing packages — `neworder2/oldpacks.md` {#packages}
Trigger: the keyword *"начни сбор существующих пакетов"* (or equivalent). Verified traversal (§3) over the packages in groups `org.vibevm.ai-native`, `org.vibevm.world`, `org.vibevm.fractality`. **Markdown specs only — NOT the fractality log files** (large machine logs; skip them). Reset the file at the start. For each package: read its key specs and **distil into the index** — `object | loaded | covers (1-line) | exported spec:// namespace | has a checker?` — and **seed the concept registry** (`concepts.md`) with the patterns it already owns. You are building a *durable index*, not holding every package's raw text in context forever: the distillation in `oldpacks.md` + `concepts.md` is what survives restarts and drives dedup. Acceptance: every package row is `loaded`.

### 4.3 Trace baseline — `neworder2/trace-baseline/` {#baseline}
Snapshot the "before" so a moved unit's edges can be proven intact:
- `specmap.snapshot` — run the specmap index build and record the current **dangling-edge set** (there is a known pre-existing orphan `EmbeddedPrecedence`; capture it here so it does not count as *new* later). Command: `cargo xtask specmap` (writes/reads the index) — record the dangling list.
- `prose-links.tsv` — a grep index of every inline `spec://…#anchor` citation across the in-scope specs + boot + the trio (prose links are **not** graph edges, so they must be tracked separately).
- `anchors.tsv` — every `{#anchor}` in the corpus.
Acceptance: the three files exist and are committed. These are the deltas §4.9 gates against.

### 4.4 Collect source files — `neworder2/allspecs.md` {#sources}
Verified traversal (§3) over the in-scope-editable + in-scope-read-only set. Table columns: `file | analyzed | distilled | refactored | removed/relocated? | notesfile`. All status columns start empty. The `notesfile` path **mirrors the `spec/` tree** under `neworder2/notes/` (e.g. `spec/common/PROP-000.md` → `neworder2/notes/spec/common/PROP-000.md`), which is the collision-free naming. Create each notesfile empty with its **first line = the repo-relative path of the source spec it mirrors**, and record that path in the `notesfile` column. Acceptance: every in-scope file has a row and a notesfile.

### 4.5 Restricted freeze + analysis {#analysis}
**The freeze:** during analysis, no in-scope spec or root markdown may be changed **in meaning** — the only permitted edit is **adding markers**. Session commands (`сохрани/восстанови сессию`) may touch `WAL.md`/`CONTINUE.md`, never specs. Any plan/bug/insight goes to `neworder2/memory/`.

Verified traversal (§3) over the source files, **ordered by pattern density — highest first (`CLAUDE.md`, `PROP-000`, the boot snippets)** so target packages and concept entries are established early and later files merge into them. For each FILE:
- **80% check first** (§5). If over, pause+checkpoint; do not start this FILE.
- **Paragraph-by-paragraph classify** (verified traversal over paragraphs). Each paragraph is one of: **(a) a VibeVM feature** (stays — e.g. "`vibe` CLI options", the resolver's fixed-point), **(b) a reusable pattern** — a package candidate not specific to vibevm (e.g. a `windows-computer-use`-style GitBash procedure), or **(c) a universal AI-dev pattern** — a `redbook`-collection candidate (e.g. Conventional Commits).
- **Mark every non-feature candidate** in place: `<!-- MARKER:NAME -->` … `<!-- /MARKER:NAME -->`, with a considered, meaningful name (e.g. `REFACTORING:COMMITS:CONVENTIONAL:001`). Mark **mixed** feature+pattern spans too (split happens at §4.6). Contiguous candidate spans get **one** marker.
- **Whole-file second pass**: look for file-level patterns not visible paragraph-by-paragraph.
- **Save every candidate to the notesfile**, each wrapped in a matching marker.
- **CONFLICT RESOLUTION / MERGE** (the key judgment): if a pattern is already in `concepts.md` (seen earlier, or owned by an existing package), do **not** duplicate — record the new occurrence in `concepts.md` and ask: *can we merge the requirements into the single best, strongest version of this pattern?* We want the best version, not first-wins. Record the merge plan.
- **Acceptance (atomic):** set `analyzed` **only when BOTH the in-spec markers AND the notesfile are complete**. Re-entry re-reads and finishes; never duplicate.

Phase acceptance: every source file's `analyzed` is checked.

### 4.6 Distill + refactor — the capsule move {#refactor}
Begins after **all** files are analyzed. **Unfreeze `packages/`.** Verified traversal (§3) over each notesfile's marked knowledge chunks. For each chunk:
- **Target** (from `concepts.md` — dedup): an existing package (prefer reusing `redbook` / an existing package — do **not** reinvent) or a new one. Create the package if none fits.
- **Clean vs mixed:** clean text → move as-is; mixed → **split** (the feature part stays; the pattern part moves).
- **The capsule** (move the connection *with* the meaning, one transaction):
  1. Move the pattern text into the target package's spec, **keeping its `{#anchor}`**; its address becomes `spec://<pkg>/…#<anchor>`.
  2. In the source, rewrite so the **feature stays** and the pattern is **replaced by a citation** — e.g. "for this, see `spec://org.vibevm.world/<pkg>#<anchor>`."
  3. Add the target package as a **vibevm dependency** and **activate its instruction/boot-snippet** where the text was cut.
  4. Add the package to the host `specmap.toml` `[[external_specs]]` so edges into its new address resolve.
  5. **Retarget inbound edges**: code `#[spec]`/`scope!` citing the old address (`grep -rn 'old#anchor' crates/`) → the new address; prose `spec://…#anchor` citations (`grep -rn` over specs + boot + trio) → the new address.
- **Byte-identical trio:** if extracting from `CLAUDE.md`/`AGENTS.md`/`GEMINI.md`, keep all three **byte-identical** (the `sync-engines` gate enforces it). **NEVER extract Rule 1's attribution paragraph** — by its own text it is the single place in the project where that topic lives.
- **Mark** `distilled`, then `refactored`. Set `removed/relocated?` to one of `processed` (stayed in place), `removed` (cut into packages, source gone), or `relocated: <path>`.
- **80% rule** applies before each chunk.

### 4.7 Report — `neworder2/report.md` {#report}
The big analytical report: what was done, the concept→package map, insights, contradictions found, and the **engine requirements** — every manual `move-unit`/`rename-address` you performed by hand is a specification for the operation the future refactoring engine must automate. This is the bootstrap's payload to the engine build. Open-ended; use your judgment, you ran the whole process.

### 4.8 Conflict resolution — MANUAL trigger {#conflicts}
The last, long, philosophical phase, run in its own session(s) on the owner's word. Packages or statements-within-packages may conflict *irreconcilably* and be non-mergeable; these must be split across different packages. Separate procedure, discussed when triggered. Not part of the autonomous run.

### 4.9 The gate ladder + boot-resolves + marker cleanup {#gates}

**Per-capsule gate ladder (all must pass before the capsule's commit):**
1. **No new dangling edges** — rebuild the specmap index; the dangling set must not grow vs `trace-baseline/specmap.snapshot` (this catches a severed spec↔code edge; the pre-existing orphan is in the baseline, so it does not count).
2. **No new broken prose links** — re-grep `spec://` citations; every one still resolves vs `trace-baseline/prose-links.tsv`.
3. **`bash tools/self-check.sh` exit 0** — fmt, test, clippy, `vibe check`, conform, **sync-engines** (the byte-identical trio), package gates. Check the REAL exit code (`; echo "EXIT=$?"`), never a `| tail`'d pipe.
4. **Boot resolves** — `vibe install` re-materialises + regenerates `spec/boot/INDEX.md`; `vibe check` passes; every path `INDEX.md` names exists. A restart must boot.
5. **Topic commit** (Rule 3) — one commit per capsule/concept, Conventional Commits, no AI attribution (Rule 1). Progress is durable after every capsule.

**Marker cleanup — MANUAL trigger** (`§4.9-cleanup`): most markers leave with their moved text; this final manual sweep (verified traversal over `spec/`) removes any residual `<!-- MARKER:… -->` from chunks that stayed in place. Run on the owner's word after review.

## 5. Context-budget + resume protocol {#resume}
- **Before each FILE / chunk**, estimate context usage. If **> 80%**, STOP: do **not** compact, do **not** start the next unit. Checkpoint (save session → update `WAL.md`/`CONTINUE.md` with the resume pointer), print a status summary to the user, and end `/goal` as **paused-complete**.
- **Do not trust the estimate alone.** Commit after every capsule (§4.9.5) so a surprise overflow/compaction never loses more than the in-flight unit.
- **Resume** (`восстанови сессию`): boot (must resolve) → read `neworder2/` durable state → find the last unprocessed unit via `allspecs.md`'s columns → continue exactly there.

## 6. Whole-process acceptance {#acceptance}
- `allspecs.md`: every file has `analyzed`, `distilled`, `refactored` checked; `removed/relocated?` filled with `processed` | `removed` | `relocated: <path>` for every row.
- `report.md` written.
- The gate ladder (§4.9) is green on the final state: `self-check` exit 0, no new dangling vs baseline, no new broken prose links, boot resolves, on branch `refactor`.
- Markers cleaned (manual phase) and conflicts resolved-or-dispositioned (manual phase).

## 7. What changed from the v0 plan, and why (for the human) {#improvements}
Injected from the design discussion: **§4.1 scope manifest** (defines "all" precisely + a read-only danger zone: `00-core`/`90-user`/`VIBEVM-SPEC` never edited); **§4.2 packages-as-index** (distil, don't hold all raw text — context-budget) + logs excluded; **§3 concept registry** (dedup across files — the target axis the per-file notesfiles miss); **§4.3 trace baseline** + **§4.9 gate ladder** (specmap dangling-delta, prose-link-delta, self-check, boot-resolves — real safety the markers alone cannot give); **§4.6 capsule protocol** (move the edge *with* the meaning; `external_specs` upkeep; retarget code + prose); **byte-identical-trio + Rule-1 attribution** guards; **§3 atomicity** (`analyzed` only when markers AND notesfile done); **§1.4 restate-and-trial** (real comprehension, not compliance theater) replacing the per-step confirmations; **commit-after-every-capsule** (durable through context surprises); **branch + tag + boot-resolves** safety. The marker/notesfile/freeze/verified-traversal method is yours, kept intact.

---

*This is the bootstrap refactoring of `spec/terraforms/REFACTORING-ENGINE-META-PLAN-v0.1.md`: it runs with minimal tooling (the existing specmap gate, no engine), cleans and layers vibevm's own specs, and its `report.md` (§4.7) hands the engine build a concrete requirements list drawn from doing every move by hand. `spec/WAL.md` is the living state and supersedes this plan where they diverge.*
