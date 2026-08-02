# WAL — Project Continuation State {#root}

_Updated: 2026-08-03, wind-down №3 (**PHASE D CLOSED · PHASE E AUTHORIZED —
the owner's «даю добро» is recorded: first slice = wave А (B-011, самый
высокий приоритет) + the research pair B-022/B-023; executor = claudez
workers (the full transport contract is built and live-verified: `-c`
thread isolation, worktree parallelism, stream-json logs into the archive,
30-second observability, WORKER-REPORT acceptance maps); boss = Fable,
minimised to stitching-acceptance-judgement. Corpus 98.0 % (`ai-native`
98.3 %), registry 90 / 190 — owed 17, every one on an owner-ruled build.
PHASES T/F/G STILL WAIT FOR THEIR OWN WORD.**)_

##WAL-NUMBERS-COME-FROM-COMMANDS **Every number below is reproduced by two
commands; run them rather than quoting this file** (why:
`spec://vibevm/terraforms/packages-actualization#quick-start` — a figure
quoted from a checkpoint decays). @impl/done

```bash
python campaigns/packages-2026-09/tasks/drift-registry.py   # obligations, routes, convergence
python campaigns/packages-2026-09/tasks/summary.py          # what the verdicts say now
```

## Current phase {#current-phase}

##WAL-PHASE **Progress Control (PROP-043) — wave 2, `packages-2026-09`:
Phase D CLOSED 2026-08-03 at a green gate; PHASE E AUTHORIZED the same day
(§7 LOG, пятый обмен) and STARTS IN THE NEXT SESSION.** Live zone
`campaigns/packages-2026-09/`; `campaigns/progress-2026-08/` is archival
and never pointed at (B-010). @impl/done

##WAL-STATE **State at wind-down** (2026-08-03; the commands supersede):
corpus **11 188 / 190 / 44 — 98.0 %** over 261 files (`ai-native` 98.3 %,
`host` 99.9 %, `world` 95.5 %); registry **90 / 190 — 32 owner-ruled
deferrals, 58 open**; CONVERGENCE **173 routed / 17 owed / 0 partial**,
the 17 on six deferred rows each naming its build (ledger
`#close-2026-08-03`); resolved **139**; `baseline.json` written (2 221
units); panel green incl. step 6b. Mirrors synced at every checkpoint. @impl/done

## Next — the Phase E mandate {#next}

1. ##WAL-NEXT-E-MANDATE **Execute the Phase E mandate (authorized
   2026-08-03, «даю добро»):** first slice = **волна А — B-011**
   (deterministic loading: rename-on-splice, `#use spec://… as X` + `@!X`,
   the C++ ADL analogy, the dynamic-STATIC.md case; the owner's design
   directions live verbatim in `BACKLOG.md` B-011) **+ the research pair
   B-022/B-023** (по образцу B-012) in a parallel lane. **B-011's design
   is boss work and returns to the owner BEFORE implementation**;
   implementation, spike prototypes and evidence sweeps go to claudez
   workers. B-006/B-031/B-028 follow B-011 inside wave А; waves Б/В/Г
   wait; release events (the `-lang` re-mint residue among them) go to
   the owner before publication; **phases T/F/G are not covered by this
   добро**. @spec/done
2. ##WAL-NEXT-ECONOMICS **The economics of the split (owner, verbatim
   2026-08-03):** «минимизация нагрузки на Fable и максимизация
   кодинговой работы на claudez субагенты»; «работа по
   сшивке-приемке-суждениям высокоуровневым никто лучше [Fable] не
   сделает». Fable never types code it can hand off — it cuts packets,
   judges designs, reviews over the WORKER-REPORT map, stitches, merges,
   re-judges anchors, commits, talks to the owner. ALL coding goes to
   workers. @impl/done
3. ##WAL-NEXT-OWNER-COURT **On the owner, none blocking:** the audit's
   open rows (cargo-outdated layout; the dead_code shadow triage; the
   2026-06-12-01 rider); DBT-0023 (fractality lock's quinn-proto);
   MT-02/MT-03; the standing known-issues list. @spec/done

## Constraints — do not violate {#constraints}

- ##WAL-C-VERDICT-STANDARD **The verdict standard.** PRESCRIBES → confirmed
  when coherent and every referent resolves (declared-future included);
  DESCRIBES → checked against the tree; unexercisable subject →
  unverifiable. `world` adds source 2 (host conformance); **§3.8 bounds
  source 2**: Go/TS are checked only by their own artefacts and tests, Rust
  is the exception (why: batch plan `#audience`). @impl/done
- ##WAL-C-BUILD-FIRST **BUILD-FIRST (owner, 2026-08-02).** A discipline rule
  is never weakened for being unused; an annotation is legitimate only as
  an interim naming a recorded build («Specified, not built (→ B-nnn)»,
  marker `@impl/plan`). «Система не заморожена». @impl/done
- ##WAL-C-CAMPAIGN-FRAME **The campaign frame (owner, 2026-08-02).** The
  map's waves execute through the campaign's phases; nothing starts from
  the map; what a mandate does not cover waits. @impl/done
- ##WAL-C-PRESENTATION-FORMAT **Presentation format (refined 2026-08-03,
  binding).** Суть проблемы / решения / рекомендации — сначала простым
  языком, НЕ требующим чтения спецификаций; спец-жаргон — только
  приложением; пункты/строки спек не цитировать; точность обязательна —
  настройка именем, файл путём, поведение конкретно; у структурного
  вопроса — дерево компонентов/решений; сначала ясно, потом технические
  детали. @impl/done
- ##WAL-C-DELEGATION **The E/T worker transport (owner directives
  2026-08-03; mechanics `campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md`):**
  the owner-owned switch `campaigns/packages-2026-09/SUBAGENT-MODE.toml`
  (now `claudez`) is re-read before EVERY fan-out; `claudez` →
  Claude-Code-on-GLM workers via the reworked claudez/claudez2 launchers
  (`-c`/`-p` verified — the ALPHA/BRAVO matrix; per-launcher state dirs;
  effort max built in); `native` → the harness's `opus5` subagents;
  fractality stays out. **Parallelism:** disjoint file perimeters → two
  lanes (claudez + claudez2) in separate worktrees, merged after review;
  conflict-prone many-place edits → ONE thread. **Observability:**
  stream-json logs written DIRECTLY into
  `C:\Users\olegc\git\v\cache\agents\{sorted/<task-id>,unsorted}/`,
  packet-mandated `PROGRESS:`/`TASK-DONE` heartbeats, ~30 s polls (log
  growth is the primary signal — workers skip heartbeats, measured; stall
  ≈ mtime ≳5 min), `meta.md` + stamped report at finalisation. **Every
  packet mandates the closing `WORKER-REPORT-<task-id>.md`** (inlined
  template; files list for the mechanical `git status` cross-check,
  acceptance with `file:line` evidence, verbatim self-verify, mandatory
  deviations) — the review map, never the review's replacement.
  Acceptance by artifacts, never by the final string. Verdicts, anchor
  routing, review and commits are never delegated in either mode; workers
  get no git verbs; briefs cite durable files only. @impl/done
- ##WAL-C-SYNC-ENGINES **A fix inside a package's crates is vendored
  forward the same pass** — `cargo xtask sync-engines` (the panel gates
  it; §5-E's law). @impl/done
- ##WAL-C-NO-MEASUREMENTS-ANSWER **«Замеров нет и нескоро будет»** — the
  standing answer (B-042, the map, three complete-targets); the question
  is never raised to the owner again. @impl/done
- ##WAL-C-DEFERRED-IS-OWNER-RULED **`deferred` in the registry = an
  owner-ruled row**, never a boss-side routing record (the reverted 58-row
  flip). The gate reads owed + rulings, not status counts. @impl/done
- ##WAL-C-REAL-MIRROR **The real mirror is `vibe progress mirror
  --campaign <zone>`**; `progress check` is NOT it; any anchor-set change
  requires the mirror before `merge-verdicts.py`. @impl/done
- ##WAL-C-VERDICT-FIRST **A false `confirmed` is repaired verdict-first:**
  re-judge to drift, let the registry mint, then edit. @impl/done
- ##WAL-C-STRIKE-PER-ANCHOR **A strike-by-ruling checks each anchor's own
  recorded reason** — the claim's carrier is often not the harvest
  table's anchor. @impl/done
- ##WAL-C-QUEUE-FROM-REGISTRY **The owner's queue derives from the
  registry, never from a harvest snapshot.** @impl/done
- ##WAL-C-PERIMETER **The perimeter law.** SPEC in `core-ai-native`,
  ENGINE in its five crates (vendored ×6), DRIVER per stack CLI,
  DEPLOYMENT in the consumer; ≥2 adopters in-tree (host + fractality);
  `legacy-spec/**` excluded; a `not-found` is a fact about the perimeter
  until checked. @impl/done
- ##WAL-C-READ-FURTHER **Read the document further before searching
  wider** (batch plan §6.1). @impl/done
- ##WAL-C-OWN-CORPUS **The campaign is inside its own corpus:** exclude
  `campaigns/*/run/**` from evidence; git figures name their HEAD. @impl/done
- ##WAL-C-CACHE-MERGE-ONLY **`run/cache.json` is load-and-merge only;
  never chain merge and seal; never hand-write
  `verified_at`/`processed_hash`.** WinError 5 on the cache swap —
  retry, idempotent. Verdict shape:
  `files.<path>.campaign.verdicts.<A>.v`; print via
  `PYTHONIOENCODING=utf-8`. @impl/done
- ##WAL-C-PROGRESS-WRITES **Every parsing `vibe progress` subcommand
  writes zone state; always pass `--campaign`; never point at
  `campaigns/progress-2026-08`** (B-010). @impl/done
- ##WAL-C-SELF-CHECK-EXCLUSION **No real `vibe` command while
  `tools/self-check.sh` runs.** Steps 0c (instruction-triple
  byte-compare) and 6b (`cargo xtask check-codegen` — needs the
  machine-local jtd-codegen binary; recipe:
  `tool:org.vibevm.ai-native/jtd-codegen`) are part of the panel. @impl/done
- ##WAL-C-STAGE-EXPLICIT **Never `git add -A` while a worker is out**;
  stage explicit paths. @impl/done
- ##WAL-C-DURABLE-CITATIONS **Briefs cite durable files only; a wind-down
  invalidates evidence tables citing `CONTINUE.md`/`spec/WAL.md`.** @impl/done
- ##WAL-C-SHELL-TRAPS **Shell traps that already fired:** `grep -v
  '\.vibe'` deletes our own packages; PowerShell `-match` is
  case-insensitive; Python `str.replace` with `\n` no-ops on CRLF — use
  editor tools; Git Bash heredocs eat `\\` in inline python — use script
  files; `git commit -q` глотает вывод — `echo $?`; a `json.dump` must
  match the registry file's indent (debt.json = 2) or the whole file
  reflows. @impl/done
- ##WAL-C-BOOT-PAIR **Boot pair marking:** `spec/boot/00-core.md` /
  `90-user.md` carry owner machine facts — mark additively; `refs/book/`
  NOTOUCH. @impl/done
- ##WAL-C-MISC **Small standing facts:** parse payload at
  `~/.vibe/progress-cache/…` carries no verdicts; `vibe.lock` +
  `[[mcp_server]]`/`[[binary]]` tables are B-046's autodiscovery rails;
  MT-02/MT-03 await the owner's manual sign-off; the redbook manifest
  carries the edition rule; `vibe progress baseline --campaign <zone>`
  writes the boundary baseline; `cargo outdated` cannot run over this
  workspace (audit 2026-08-03-03). @impl/done

## Done (collapsed — see `git log` and the §7 LOG) {#done}

##WAL-DONE **Phase D executed 2026-07-29 → 2026-08-03 and closed at a green
gate: 601 drift verdicts → 190, 94.3 % → 98.0 %; F-279 executed (B-013
closed, `tool:org.vibevm.ai-native/jtd-codegen` minted), the §11 gate
measured, the A–D audit run (panel + lock fixed in-run), the close trio
bound (`755d664a`/`dcc23250`/`9c965514`).** The same day, four more owner
exchanges built the **E/T worker transport end to end, each piece
live-verified:** the claudez/claudez2 launchers de-siamesed (per-launcher
state → `-c` thread isolation, the ALPHA/BRAVO matrix, both shells),
effort max built in; the observability contract (stream-json into
`cache/agents`, heartbeats, 30 s polls, meta finalisation) probed live
(6 turns / 37.9 s; two weak-writer lessons paid); the WORKER-REPORT
acceptance map probed live (probe-report-01 — exact template fill); and
**the Phase E mandate authorized** («даю добро», пятый обмен + the
Fable-minimisation economics). Earlier: the 2026-08-02 sitting drained the
approval queue whole; Phase C closed 2026-07-28; Phase B at zero. @impl/done

## In progress {#in-progress}

##WAL-INFLIGHT **Nothing is in flight.** No workers out, no unsealed
merges, no uncommitted state; mirrors synced at the wind-down fan-out. The
next session opens ON THE MANDATE (boot → prepare → execute wave А +
B-022/B-023), not on a report-and-wait — the добро is given and recorded. @impl/done

## Known issues {#known-issues}

- ##WAL-KI-OPEN **Open on the owner, none blocking:** F-129; F-122; F-126;
  F-127; F-128; F-120; the H-roster; F-069; the specmap ratchet's 37 gated
  orphans; F-125 likely closed by the 75.3/70.2 canon — verify before
  citing; the 2026-06-12-01 history-rewrite rider (third audit run). @impl/done
- ##WAL-KI-AUDIT **The audit's active subset (`AUDIT.md` §2026-08-03):**
  open — cargo-outdated unrunnable over this layout (-03), the
  dead_code-allow shadow 28 → 79 (-04); filed — DBT-0023 (fractality
  lock's quinn-proto 0.11.14 → its own session bumps it). @impl/done
- ##WAL-KI-BACKLOG **The backlog carries B-001…B-047** (`BACKLOG.md` +
  `#map`; `TOOLING-MAP.md` waves under the campaign frame). **B-013 done**
  (2026-08-03). B-015 parked until the owner's notice. @impl/done

## Session context {#session-context}

##WAL-CTX-BOOT **A cold session starts at the campaign quick-start**
(`spec://vibevm/terraforms/packages-actualization#quick-start`), reads
`CONTINUE.md` (the E-start recipe lives there), **the transport law
`campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md` WHOLE**, the plan's §5
Phase E + §6.1, the ledger's `#close-2026-08-03`, `BACKLOG.md` B-011/
B-022/B-023, `TOOLING-MAP.md` — and takes every number from the two
commands at the top of this file. `CONTINUE.md` is the cold-resume
snapshot; this file supersedes it wherever they diverge. @impl/done
