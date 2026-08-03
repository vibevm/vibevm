# WAL — Project Continuation State {#root}

_Updated: 2026-08-04, wind-down №4 (**PHASE E — FIRST SLICE LANDED WHOLE:
B-011 designed → owner-approved → spec'd → built by five claudez slices →
the host lane regenerated anchor-qualified (0 duplicate anchors vs the 59
warnings the entry was filed on; M-LOAD taken on both measurements);
B-022's enum slice landed (+6 vendors); the B-022/B-023 studies closed
with owner rulings. Panel green — tail read; mirrors synced. The
campaign registry is untouched this session — the anchor re-judgements
are their own passes, next. PHASES T/F/G STILL WAIT FOR THEIR OWN WORD;
package PUBLICATION waits for the refactor's end by the owner's word.**)_

##WAL-NUMBERS-COME-FROM-COMMANDS **Every number below is reproduced by two
commands; run them rather than quoting this file** (why:
`spec://org.vibevm.core/vibevm/terraforms/packages-actualization#quick-start`). @impl/done

```bash
python campaigns/packages-2026-09/tasks/drift-registry.py
python campaigns/packages-2026-09/tasks/summary.py
```

## Current phase {#current-phase}

##WAL-PHASE **Progress Control (PROP-043) — wave 2, `packages-2026-09`:
Phase E RUNNING under the 2026-08-03 mandate; the first slice (волна А
opener B-011 + the research pair + B-022's build slice) landed whole
2026-08-04.** Live zone `campaigns/packages-2026-09/`;
`campaigns/progress-2026-08/` is archival (B-010). @impl/done

##WAL-STATE **State at wind-down** (2026-08-04; the commands supersede):
registry **90 / 190 — owed 17 on the six deferred rows, resolved 139 —
unchanged this session by design** (re-judgements are queued passes, not
byproducts). The host boot lane is anchor-qualified: 137/137 heading
anchors and 518/518 fact ids unique, resolution preamble + tombstone
under the header. Panel green incl. the new `lane-citation lint (B-011)`
step; mirrors at `5fc99cf7`+. @impl/done

## Next — continuing the Phase E mandate {#next}

1. ##WAL-NEXT-B006 **B-006** (the git-family double emission) — next in
   волна А; the duplicate is now mechanically visible as same-origin
   repeats in the lane's tombstone. Direction: the soft-hoist/`use_ref`
   machinery (PROP-038 §2.5, already in `render_static`) or lane-set
   dedup; a contract-changing design goes to the owner first. Carries
   W3's named follow-up: per-node qualify for cross-origin normal
   closures. Then **B-031, B-028**. @spec/done
2. ##WAL-NEXT-REJUDGE **The re-judgement passes — rulings in hand,
   routine:** *(a)* F-159/B-022 — interim annotations into
   LEDGER-INTENT per the synthesis verdict table (M-A layer 2 → B-020;
   M-B → B-020+M-D; M-C's two absent measures → B-020; M-D → B-015
   notice), then the five anchors re-judge; *(b)* F-146/B-023 — the §2
   frontend-table re-annotation (honest ts-tsc depth + the deferral
   verbatim from B-023's disposition), then the two anchors. Both via
   `vibe progress mirror --campaign` → merge-verdicts → seal, never
   chained. *(c)* the host prose fix `terraform/REPORT.md:41`. @spec/done
3. ##WAL-NEXT-OWNER **On the owner, none blocking:** audit's open rows;
   DBT-0023; MT-02/MT-03; map forks one at a time; the pre-publication
   boundary call (versions mint + publication as ONE operation, his
   2026-08-04 ruling). @spec/done

## Constraints — do not violate {#constraints}

- ##WAL-C-VERDICT-STANDARD **The verdict standard.** PRESCRIBES →
  confirmed when coherent and every referent resolves; DESCRIBES →
  checked against the tree; unexercisable → unverifiable; `world` adds
  source 2, §3.8 bounds it (Go/TS by their own artefacts; Rust the
  exception). @impl/done
- ##WAL-C-BUILD-FIRST **BUILD-FIRST (owner, 2026-08-02).** A discipline
  rule is never weakened for being unused; an annotation is legitimate
  only as an interim naming a recorded build. @impl/done
- ##WAL-C-CAMPAIGN-FRAME **The campaign frame.** The map's waves execute
  through the campaign's phases; what a mandate does not cover waits.
  Phase E covers: волна А (B-011 done → B-006 → B-031 → B-028), the
  research pair (done), B-022's agreed slice (done). T/F/G wait. @impl/done
- ##WAL-C-RELEASE **Release (owner, 2026-08-04).** Publication into the
  GitHub registry is a SEPARATE operation after the refactor completes;
  versions are NOT bumped until the pre-publication boundary, where the
  new-slot mint and publication are one operation. In-slot edits keep
  accumulating meanwhile (the registry's release-route rows track them). @impl/done
- ##WAL-C-PRESENTATION-FORMAT **Presentation format (binding).** Суть
  по-человечески БЕЗ чтения спек → дерево для развилок → точные имена;
  жаргон приложением; спеки не цитировать. @impl/done
- ##WAL-C-DELEGATION **The E/T worker transport** (mechanics
  `campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md`; switch
  `SUBAGENT-MODE.toml` re-read before EVERY fan-out, now `claudez`):
  launchers `C:\Users\olegc\opt\bin\{claudez,claudez2}`, per-launcher
  state dirs, `-c` continues its own thread, effort max built in;
  disjoint perimeters → parallel (≤5/launcher, ≤10 total; cargo-heavy
  2–3), conflict-prone many-place → ONE thread; stream-json logs
  DIRECTLY into `C:\Users\olegc\git\v\cache\agents\sorted\<task-id>\`;
  packet-mandated heartbeats; ~30 s polls (log growth primary; stall ≈
  mtime ≳5 min); WORKER-REPORT with «Decisions taken (each with why)»
  mandatory — **its EXISTENCE is part of the mechanical set-compare**;
  boss scaffolds carry named refinement points; rejection right:
  ПРИНЯТО · НЕ ПРИНЯТО→`-c` (a rework landing in a specific report
  section DICTATES that section's text verbatim) · re-commission
  (ceiling 2) · discard; every cycle in meta.md; acceptance by
  artifacts; workers get no git verbs; briefs cite durable files only.
  **Code-slice self-verify INCLUDES
  `cargo clippy -p <crate> --all-targets -- -D warnings`** (paid
  2026-08-04). Verdicts, anchor routing, review, commits never
  delegated. @impl/done
- ##WAL-C-PANEL-TAIL **The panel's exit is read from the TAIL, never a
  task notification; background form is bare `bash tools/self-check.sh`
  (an appended echo swallows the exit — paid 2026-08-04 by fanning out
  on a red panel); the mirror fan-out waits for the read tail.** @impl/done
- ##WAL-C-SYNC-ENGINES **A fix inside a package's crates is vendored
  forward the same pass** — `cargo xtask sync-engines` (panel-gated).
  Run it from the HOST root (xtask lives there). @impl/done
- ##WAL-C-LANE-LAW **The compiled lane's law (B-011, live):** labels are
  origin-qualified; the lane opens with the resolution preamble +
  tombstone; a `spec://…/boot/STATIC#…` directive/`@spec` target is
  rejected by the scanner AND linted by the panel; HTML comments are
  masked machinery for the directive scanner; regeneration =
  `cargo run -q -p vibe-cli --bin vibe -- install --assume-yes`. @impl/done
- ##WAL-C-NO-MEASUREMENTS-ANSWER **«Замеров нет и нескоро будет»** — the
  standing answer; never re-raised. @impl/done
- ##WAL-C-DEFERRED-IS-OWNER-RULED **`deferred` in the registry = an
  owner-ruled row.** The gate reads owed + rulings. @impl/done
- ##WAL-C-REAL-MIRROR **The real mirror is `vibe progress mirror
  --campaign <zone>`**; any anchor-set change requires it before
  `merge-verdicts.py`; never chain merge and seal. @impl/done
- ##WAL-C-VERDICT-FIRST **A false `confirmed` is repaired verdict-first.** @impl/done
- ##WAL-C-STRIKE-PER-ANCHOR **A strike-by-ruling checks each anchor's own
  recorded reason.** @impl/done
- ##WAL-C-QUEUE-FROM-REGISTRY **The owner's queue derives from the
  registry, never a harvest snapshot.** @impl/done
- ##WAL-C-PERIMETER **The perimeter law.** SPEC in `core-ai-native`,
  ENGINE in its five crates (vendored ×6), DRIVER per stack CLI,
  DEPLOYMENT in the consumer; `legacy-spec/**` excluded; a `not-found`
  is a fact about the perimeter until checked. @impl/done
- ##WAL-C-READ-FURTHER **Read the document further before searching
  wider.** @impl/done
- ##WAL-C-OWN-CORPUS **The campaign is inside its own corpus:** exclude
  `campaigns/*/run/**` from evidence; git figures name their HEAD. @impl/done
- ##WAL-C-CACHE-MERGE-ONLY **`run/cache.json` is load-and-merge only;
  never hand-write `verified_at`/`processed_hash`;** WinError 5 → retry.
  Print via `PYTHONIOENCODING=utf-8`. @impl/done
- ##WAL-C-PROGRESS-WRITES **Every parsing `vibe progress` subcommand
  writes zone state; always `--campaign`; never point at
  `campaigns/progress-2026-08`.** @impl/done
- ##WAL-C-SELF-CHECK-EXCLUSION **No real `vibe` command while
  `tools/self-check.sh` runs.** Steps 0c and 6b (local jtd-codegen) are
  part of the panel; new step: `lane-citation lint (B-011)`. @impl/done
- ##WAL-C-STAGE-EXPLICIT **Never `git add -A` while a worker is out;**
  stage explicit paths. `git commit -m … <pathspec>` does NOT pick up
  untracked files. @impl/done
- ##WAL-C-DURABLE-CITATIONS **Briefs cite durable files only; a wind-down
  invalidates evidence citing `CONTINUE.md`/`spec/WAL.md`.** @impl/done
- ##WAL-C-SHELL-TRAPS **Shell traps that already fired:** Bash-tool cwd
  PERSISTS between calls — a `cd` in one command breaks the next call's
  relative paths (paid twice 2026-08-04; use absolute paths); `grep -v
  '\.vibe'` deletes our own packages; PowerShell `-match`
  case-insensitive; CRLF vs `str.replace`; Git Bash heredocs eat `\\`;
  `git commit -q` глотает вывод; `json.dump` indent must match (debt=2);
  Windows holds worktree file handles after a worker dies — `worktree
  remove --force` → Permission denied → `prune` + `rm -rf`. @impl/done
- ##WAL-C-BOOT-PAIR **Boot pair marking:** `00-core.md`/`90-user.md`
  additively; `refs/book/` NOTOUCH. @impl/done
- ##WAL-C-MISC **Small standing facts:** `vibe.lock` +
  `[[mcp_server]]`/`[[binary]]` are B-046's rails; MT-02/MT-03 await
  manual sign-off; `vibe progress baseline --campaign` writes the
  boundary baseline; `cargo outdated` unrunnable here; W3's named
  follow-ups — per-node qualify (rides B-006), directive-errors-fail-
  the-compile wiring, `#[verifies]` on the dynamic_lane exhibits. @impl/done

## Done (collapsed — see `git log` and the §7 LOG) {#done}

##WAL-DONE **2026-08-03/04, the first Phase E slice, whole:** research
pair B-022/B-023 (worker evidence + boss syntheses + owner rulings — the
B-023 depth verdict counter-probed by the owner and re-judged, then ruled
deferred-with-a-named-trigger); B-011 design (approved with every
recommended fork + the priority-placement addition) → PROP-035/PROP-009
contract edits → W1 qualify cell, W2 alias grammar, W3 splice
integration (+preamble, tombstone, candidates, lane-citation rejection),
W4 dynamic-lane exhibit (M-LOAD executable), W5 seam split — six worker
cycles, all accepted, one `-c` rework total (the missed report); the
host lane regenerated 59→0; B-022's enum slice (+6 vendors); the panel
gained the lane-citation lint; three panel-caught tails fixed (host
clippy, package clippy, file-length ×3) and their root cause written
into the transport law; the comment-mask integration fix caught on the
first live regeneration. Earlier: Phase D closed 2026-08-03 at a green
gate (§7 LOG). @impl/done

## In progress {#in-progress}

##WAL-INFLIGHT **Nothing is in flight.** No workers out, no unsealed
merges, tree clean, mirrors synced. The next session continues the
mandate (B-006 + the re-judgement passes) without re-asking. @impl/done

## Known issues {#known-issues}

- ##WAL-KI-OPEN **Open on the owner, none blocking:** F-129; F-122;
  F-126; F-127; F-128; F-120; the H-roster; F-069; the specmap
  ratchet's 37 gated orphans; F-125 verify-before-citing; the
  2026-06-12-01 rider. @impl/done
- ##WAL-KI-AUDIT **Audit's active subset (`AUDIT.md` §2026-08-03):**
  cargo-outdated (-03), dead_code shadow (-04); DBT-0023 filed. @impl/done
- ##WAL-KI-BACKLOG **Backlog B-001…B-047** — B-011 build LANDED (entry
  stays for the strip follow-up lineage), B-013 done, B-022 slice built
  (annotations pending), B-023 done+ruled (deferral trigger named),
  B-015 parked. @impl/done

## Session context {#session-context}

##WAL-CTX-BOOT **A cold session starts at the campaign quick-start**,
reads `CONTINUE.md` (the continue recipe), **the transport law
`campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md` WHOLE (three new §8
facts)**, the design doc `spec/design/deterministic-loading-aliasing.md`
(approved status, §6.1, §10), plan §5E/§6.1 + §7 LOG tail, `BACKLOG.md`
B-006/B-031/B-028 — and takes every number from the two commands at the
top. `CONTINUE.md` is the cold-resume snapshot; this file supersedes it. @impl/done
