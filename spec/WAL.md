# WAL — Project Continuation State {#root}

_Updated: 2026-08-02, session end №2 (**PHASE D: THE APPROVAL QUEUE IS
DRAINED WHOLE — the presentation sitting ran ~ten owner exchanges and closed
every sync-route presentation; on the table: ONE already-given ruling to
execute (F-279 — вариант (а) + the new `org.vibevm.ai-native/jtd-codegen`
package), then THE EXIT GATE. Corpus 97.9 % (`ai-native` 98.3 %), registry
91 obligations / 191 drifts — 32 owner-ruled deferrals, 59 open, owed 18 =
17 on owner-ruled builds + F-279's 1; resolved 138; boss-owed zero; the
backlog stands at B-047; B-027's sweep done; the map lives at
`TOOLING-MAP.md`**)_

##WAL-NUMBERS-COME-FROM-COMMANDS **Every number below is reproduced by two
commands; run them rather than quoting this file** (why:
`spec://vibevm/terraforms/packages-actualization#quick-start` — every line of
the quick-start prints a number; a figure quoted from a checkpoint decays,
proved again this sitting by the stale «F-132's nine» queue line). @impl/done

```bash
python campaigns/packages-2026-09/tasks/drift-registry.py   # obligations, routes, convergence
python campaigns/packages-2026-09/tasks/summary.py          # what the verdicts say now
```

## Current phase {#current-phase}

##WAL-PHASE **Progress Control (PROP-043) — wave 2, `packages-2026-09`, Phase D
(Stitching), at the exit gate's doorstep.** Live zone
`campaigns/packages-2026-09/`; `campaigns/progress-2026-08/` is archival and
its state files are never pointed at (`BACKLOG.md` B-010). @impl/done

##WAL-STATE **State at the last regeneration** (2026-08-02, session end №2;
the commands supersede): corpus **11 187 / 191 / 44 — 97.9 %** over 260
files and 11 422 units (`ai-native` **98.3 %**, `host` 99.9 %, `world`
95.5 %); registry **91 obligations / 191 drifts — 32 `deferred` by owner
ruling, 59 open**; **owed 18 = 17 on owner-ruled build deferrals
(B-022/023/025/026/029/031/033/034) + 1 on the open F-279, whose ruling IS
ALREADY GIVEN and awaits execution**; resolved to history **138**. The
boss-owed remainder is **zero** on every route. The rulings chronicle:
`campaigns/packages-2026-09/PHASE-D-HOST-OBLIGATIONS.md#rulings-2026-08-01`
and `#rulings-2026-08-02-2` (~ten exchanges). Panel all green многократно
(последний прогон — со стройкой пина); `progress check --campaign` clean
(260 files); cargo-audit 0.22.2 / cargo-outdated 0.19.0 installed. @impl/done

## Next — the two steps, in order {#next}

1. ##WAL-NEXT-F279 **Execute the already-given F-279 ruling** («Вариант a +
   … jtd генератор в tools/jtd-codegen … в отдельный пакет
   org.vibevm.ai-native/jtd-codegen»): the full verified recipe is
   `CONTINUE.md` §«ПЕРВОЕ ДЕЛО» — schema `schemas/specmap.jtd.json` moves
   into `core-ai-native/v0.8.0/schemas/` (+ canonical example), `xtask
   codegen` re-routes off the dead v0.5.0 slot (closes **B-013** whole),
   the rust-stack README's `##SHIPS-SPECMAP-WIRE-SCHEMA` tells the
   post-move truth (D42 → merge → seal → F-279 resolves → CONVERGENCE
   satisfied), and `tools/jtd-codegen/` (README.md + jtd-codegen.exe)
   becomes the `tool`-kind package `org.vibevm.ai-native/jtd-codegen` —
   **read its README first; the checked-in .exe raises the binary-payload
   question (Rule 4 borderline): one short options question to the owner
   if the form is not obvious.** @spec/done
2. ##WAL-NEXT-EXIT **Run the exit gate** (the standing instruction fires
   once F-279 lands): acceptance §11 steps 0–4 (panel; exhaustive check
   `--campaign`; per-namespace summary; CONVERGENCE owed-0-or-ruled +
   `routing.json`; **write `baseline.json`**, amendment A6), plus the
   **A–D health-audit inventory** (adoption clause of 2026-08-01), the
   phase-close LOG entry and the commit-map trio (§7.1 `#cm-planned`).
   **Phases E/T/F/G are designed and do not start without the owner's
   word.** Phase E's mandate drains the recorded builds per the map's
   frame. @spec/done

## Constraints — do not violate {#constraints}

- ##WAL-C-VERDICT-STANDARD **The verdict standard.** PRESCRIBES → confirmed
  when coherent and every referent resolves (declared-future included);
  DESCRIBES → checked against the tree; unexercisable subject →
  unverifiable. `world` adds source 2 (host conformance); **§3.8 bounds
  source 2**: Go/TS are checked only by their own artefacts and tests, Rust
  is the exception (why: batch plan `#audience`). @impl/done
- ##WAL-C-BUILD-FIRST **BUILD-FIRST (owner, 2026-08-02).** For discipline
  mechanisms the annotate-absence default is dead: a rule is never weakened
  for being unused; an annotation is legitimate only as an interim naming a
  recorded build («Specified, not built (→ B-nnn)», marker `@impl/plan` per
  the executed B-027 rule). «Система не заморожена, она должна развиваться»
  (why: ledger, four rulings of the fourth exchange). @impl/done
- ##WAL-C-CAMPAIGN-FRAME **The campaign frame (owner, 2026-08-02).** The
  map's waves (`TOOLING-MAP.md`) execute through the campaign's phases (E
  after D's gate); nothing starts from the map; what a mandate does not
  cover waits (why: the map's `##frame-line`, his words verbatim). @impl/done
- ##WAL-C-PRESENTATION-FORMAT **Presentation format** (refined 2026-08-02,
  binding): суть по-человечески first, then the EXACT technical names —
  settings, files, behaviour — precision never lost; spec jargon only as
  appendix. «Две настройки» без имён — недопустимо (why: the owner's
  rebuke, second exchange). @impl/done
- ##WAL-C-NO-MEASUREMENTS-ANSWER **The no-measurements standing answer:**
  «замеров нет и нескоро будет» — recorded in B-042, the map, and all three
  stacks' complete-targets. **The question is never raised to the owner
  again** (why: his exact words, sixth exchange). @impl/done
- ##WAL-C-DEFERRED-IS-OWNER-RULED **`deferred` in the registry = an
  owner-ruled row**, never a boss-side routing record: the 58-row bulk flip
  was made and REVERTED within the hour; the gate reads owed + rulings, not
  status counts (why: the ninth exchange's recorded near-miss). @impl/done
- ##WAL-C-REAL-MIRROR **The real mirror is `vibe progress mirror --campaign
  <zone>`** (per-file views under `run/mirror/`) — `progress check` is NOT
  it; any anchor-set change requires the mirror before `merge-verdicts.py`
  (why: F-309's two new roster anchors, merge-verdicts' fifth useful
  refusal). @impl/done
- ##WAL-C-VERDICT-FIRST **A false `confirmed` is repaired verdict-first:**
  re-judge to drift, let the registry mint, then edit — executed five times
  this sitting (D30 ×2, D39 ×4, incl. a fourth family member found
  mid-pass) (why: `#log`, the sitting's entries). @impl/done
- ##WAL-C-STRIKE-PER-ANCHOR **A strike-by-ruling checks each anchor's own
  recorded reason** — the ts-oracle zombie claim lived in
  `RUST-SIDE-OWNS-TERMINATION`, not the harvest table's SHUTDOWN anchor
  (why: the sixth exchange's per-anchor catch; F-230 precedent). @impl/done
- ##WAL-C-QUEUE-FROM-REGISTRY **The owner's queue derives from the
  registry, never from a harvest snapshot** (why: the stale «F-132's nine»
  line — партия 1a had drained it a day earlier). @impl/done
- ##WAL-C-PERIMETER **The perimeter law.** SPEC in `core-ai-native`, ENGINE
  in its five crates (vendored ×6), DRIVER per stack CLI, DEPLOYMENT in the
  consumer; ≥2 adopters in-tree (host + fractality); `legacy-spec/**`
  excluded; a `not-found` is a fact about the perimeter until checked
  (why: batch plan `#compliance-blindness`). @impl/done
- ##WAL-C-READ-FURTHER **Read the document further before searching wider**
  (why: batch plan §6.1). @impl/done
- ##WAL-C-OWN-CORPUS **The campaign is inside its own corpus:** exclude
  `campaigns/*/run/**` from evidence; git figures name their HEAD (why:
  batch plan §6.1). @impl/done
- ##WAL-C-CACHE-MERGE-ONLY **`run/cache.json` is load-and-merge only; never
  chain merge and seal; never hand-write `verified_at`/`processed_hash`.**
  Merge may hit a transient WinError 5 on the cache swap — retry, it is
  idempotent. Verdict shape: `files.<path>.campaign.verdicts.<A>.v`; print
  via `PYTHONIOENCODING=utf-8` (why: §6.1 + this sitting's lock races). @impl/done
- ##WAL-C-PROGRESS-WRITES **Every parsing `vibe progress` subcommand writes
  zone state; always pass `--campaign`; never point at
  `campaigns/progress-2026-08`** (why: B-010). @impl/done
- ##WAL-C-SELF-CHECK-EXCLUSION **No real `vibe` command while
  `tools/self-check.sh` runs** (why: the floor's capture step). Step 0c
  (CLAUDE/AGENTS/GEMINI byte-compare) is part of the panel since
  2026-08-02. @impl/done
- ##WAL-C-STAGE-EXPLICIT **Never `git add -A` while a worker is out**; stage
  explicit paths (why: Phase C lessons). @impl/done
- ##WAL-C-DURABLE-CITATIONS **Briefs cite durable files only; a wind-down
  invalidates evidence tables citing `CONTINUE.md`/`spec/WAL.md`** (why:
  batch plan §6; 116 dead refs in the one pre-rule batch). @impl/done
- ##WAL-C-SHELL-TRAPS **Shell traps that already fired:** `grep -v '\.vibe'`
  deletes our own `org.vibevm` packages; PowerShell `-match` is
  case-insensitive; Python `str.replace` with `\n` no-ops on CRLF — use
  editor tools; Git Bash heredocs eat `\\` in inline python — use script
  files; `git commit -q` глотает вывод — контроль `echo $?` (why: each
  recorded the day it bit). @impl/done
- ##WAL-C-BOOT-PAIR **Boot pair marking:** `spec/boot/00-core.md` /
  `90-user.md` carry owner machine facts — mark additively; `refs/book/`
  NOTOUCH (why: owner rulings in the boot files). @impl/done
- ##WAL-C-DELEGATION **Delegation goes to the harness's built-in `opus5`
  subagents, not fractality; verdicts, anchor routing and review are never
  delegated** (why: batch plan `#delegation`). This sitting ran all
  boss-side (judgement work). @impl/done
- ##WAL-C-MISC **Small standing facts:** parse payload at
  `~/.vibe/progress-cache/…` carries no verdicts; `vibe.lock` +
  `[[mcp_server]]`/`[[binary]]` tables are the autodiscovery rails B-046
  rides; MT-02/MT-03 await the owner's manual sign-off; the redbook
  manifest carries the standing edition rule (next roster change bumps the
  version). @impl/done

## Done (collapsed — see `git log` and the §7 LOG) {#done}

##WAL-DONE **Phase D, waves 1–12: 601 drift verdicts → 191, 94.3 % →
97.9 %.** The 2026-08-02 presentation sitting (~ten exchanges, batches
D29–D41, twenty-plus commits) drained the whole approval queue: builds
B-033…B-047 filed under the BUILD-FIRST pivot; B-011 raised to Самый
Высокий Приоритет; B-027's marker sweep executed (19 flips to
`@impl/plan`, B-027 done); the tooling map approved and integrated as
`TOOLING-MAP.md` (+ `#map` in BACKLOG); self-check step 0c built; the pin
build landed (`crates/vibe-cli/build.rs` + `RUST_PIN` from workspace
`rust-version`); PROP-000 gained the boot-surface attribution exception;
the 74.8 % canon fell to ATLAS's 75.3/70.2 pair; nine documents of the
final portion repaired incl. four floor-gloss copies and the three-stack
replay split; F-285 снят; F-210 closed with the OracleRegistry history
(the owner's own MCP-SOVEREIGNTY resolution) and B-046/B-047 filed from
his architecture direction. Earlier waves: см. §7 LOG. Phase C closed
2026-07-28 at 6 847/6 847; Phase B at zero unmarked. @impl/done

## In progress {#in-progress}

##WAL-INFLIGHT **Nothing is in flight.** No delegated passes, no unsealed
merges, no seal refusals, no uncommitted state; both mirrors synced at the
session-end fan-out. The one pending EXECUTION is the already-ruled F-279
recipe (`#next` step 1); the owner owes nothing until the jtd-codegen
package-form question, if it arises. @impl/done

## Known issues {#known-issues}

- ##WAL-KI-OPEN **Open on the owner, none blocking:** F-129 (wal package's
  two wind-downs); F-122 (realised by волна 9's re-vendor); F-126 (tcg
  naming family — partially superseded by the F-216 repair); F-127 (Go
  `-race` 15×/0×); F-128 (`spec/boot/INLINE.md` naming); F-120 (kind-line
  guide); the H-roster (owner: `core-ai-native/appendix/`); F-069
  (aggregator grammar); the specmap ratchet's 37 gated orphans. F-125's
  three-numbers question is **likely closed** by the 75.3/70.2 canon —
  verify before citing. @impl/done
- ##WAL-KI-BACKLOG **The backlog carries the designed-and-deferred work
  B-001…B-047** (see `BACKLOG.md` and its `#map` section; the drainage
  shape is `TOOLING-MAP.md`'s waves А–Г under the campaign frame). B-015
  security stays parked until the owner's explicit notice. @impl/done

## Session context {#session-context}

##WAL-CTX-BOOT **A cold session starts at the campaign quick-start**
(`spec://vibevm/terraforms/packages-actualization#quick-start`), reads
`CONTINUE.md` (the F-279 recipe lives there), the batch plan §§3.6/3.7/3.8
+ §6.1 twice, §7's LOG from the end (the ~ten 2026-08-02 entries), and
takes every number from the two commands at the top of this file.
`CONTINUE.md` is the cold-resume snapshot; this file supersedes it wherever
they diverge. @impl/done
