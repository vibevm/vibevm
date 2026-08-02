# WAL — Project Continuation State {#root}

_Updated: 2026-08-03 (**PHASE D CLOSED AT A GREEN GATE — the already-given
F-279 ruling was executed whole (schema home, codegen route, B-013, the
jtd-codegen tool package) with no owner question needed; the exit gate's
§11 steps 0–4 all pass; the A–D audit ran at the gate (5 findings, 2 fixed
in-run — the panel gained the codegen gate); the close trio is bound in the
commit map. Corpus 98.0 % (`ai-native` 98.3 %), registry 90 / 190 — owed 17,
every one on an owner-ruled build. PHASES E/T/F/G ARE DESIGNED AND DO NOT
START WITHOUT THE OWNER'S WORD.**)_

##WAL-NUMBERS-COME-FROM-COMMANDS **Every number below is reproduced by two
commands; run them rather than quoting this file** (why:
`spec://vibevm/terraforms/packages-actualization#quick-start` — a figure
quoted from a checkpoint decays; the stale «F-132's nine» queue line proved
it again). @impl/done

```bash
python campaigns/packages-2026-09/tasks/drift-registry.py   # obligations, routes, convergence
python campaigns/packages-2026-09/tasks/summary.py          # what the verdicts say now
```

## Current phase {#current-phase}

##WAL-PHASE **Progress Control (PROP-043) — wave 2, `packages-2026-09`:
Phase D CLOSED 2026-08-03 at a green gate; Phases E/T/F/G designed, not
started — the next session reports and waits for the owner's word.** Live
zone `campaigns/packages-2026-09/`; `campaigns/progress-2026-08/` is
archival and never pointed at (B-010). @impl/done

##WAL-STATE **State at close** (2026-08-03, HEAD `9c965514`; the commands
supersede): corpus **11 188 / 190 / 44 — 98.0 %** over 261 files and 11 422
units (`ai-native` **98.3 %**, `host` 99.9 %, `world` 95.5 %); registry
**90 obligations / 190 drifts — 32 owner-ruled deferrals, 58 open**;
CONVERGENCE **173 routed / 17 owed / 0 partial** — the 17 sit on six
`deferred` rows each naming its build and ruling (the ledger's
`campaigns/packages-2026-09/PHASE-D-HOST-OBLIGATIONS.md#close-2026-08-03`
table); resolved to history **139**; `baseline.json` written per A6 (2 221
units). Panel green including the NEW step 6b (`cargo xtask check-codegen`
— needs the machine-local jtd-codegen binary, recipe:
`tool:org.vibevm.ai-native/jtd-codegen`). @impl/done

## Next {#next}

1. ##WAL-NEXT-OWNER-WORD **Report to the owner and wait.** Phase D is
   closed; nothing is authorised past it. The candidate next step (not an
   authorisation): the owner's word on **Phase E**, whose mandate drains
   the recorded builds — the ledger close table (six rows → builds
   B-022/023/025/026/029/031/032/033/034), `BACKLOG.md`, and the
   `TOOLING-MAP.md` waves under the campaign frame. @spec/done
2. ##WAL-NEXT-OWNER-COURT **On the owner, none blocking:** the audit's
   open rows (2026-08-03-03 cargo-outdated layout; 2026-08-03-04 the
   dead_code-allow shadow — triage or accept next run; the 2026-06-12-01
   history-rewrite rider, third run carried); DBT-0023 (fractality lock's
   quinn-proto — the specspace's own session runs the bump); MT-02/MT-03
   manual sign-offs; the standing known-issues list below. @spec/done

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
  recorded build («Specified, not built (→ B-nnn)», marker `@impl/plan`).
  «Система не заморожена, она должна развиваться». @impl/done
- ##WAL-C-CAMPAIGN-FRAME **The campaign frame (owner, 2026-08-02).** The
  map's waves (`TOOLING-MAP.md`) execute through the campaign's phases (E
  after D's gate); nothing starts from the map; what a mandate does not
  cover waits. @impl/done
- ##WAL-C-PRESENTATION-FORMAT **Presentation format (refined again
  2026-08-03, binding).** Суть проблемы / решения / рекомендации — сначала
  простым языком, НЕ требующим чтения спецификаций; спец-жаргон и
  кампанейская терминология — только приложением к сути, не вперемешку;
  пункты/строки спецификаций в предъявлении не цитировать (владелец их
  читать не будет); точность обязательна — настройка называется именем,
  файл путём, поведение конкретно («две настройки» без имён недопустимо);
  у вопроса с внутренней структурой — дерево компонентов/решений, чтобы
  снизить когнитивную нагрузку; при конфликте ясности и точности — сначала
  ясно, потом точные технические детали (why: the owner's format rebuke,
  2026-08-03 — «я даже не понимаю, о чем речь»). @impl/done
- ##WAL-C-NO-MEASUREMENTS-ANSWER **The no-measurements standing answer:**
  «замеров нет и нескоро будет» — recorded in B-042, the map, and all three
  stacks' complete-targets. **The question is never raised to the owner
  again.** @impl/done
- ##WAL-C-DEFERRED-IS-OWNER-RULED **`deferred` in the registry = an
  owner-ruled row**, never a boss-side routing record: the 58-row bulk flip
  was made and REVERTED within the hour; the gate reads owed + rulings, not
  status counts. @impl/done
- ##WAL-C-REAL-MIRROR **The real mirror is `vibe progress mirror --campaign
  <zone>`** (per-file views under `run/mirror/`) — `progress check` is NOT
  it; any anchor-set change requires the mirror before `merge-verdicts.py`. @impl/done
- ##WAL-C-VERDICT-FIRST **A false `confirmed` is repaired verdict-first:**
  re-judge to drift, let the registry mint, then edit (executed five times
  in the sitting of 2026-08-02). @impl/done
- ##WAL-C-STRIKE-PER-ANCHOR **A strike-by-ruling checks each anchor's own
  recorded reason** — the claim's carrier is often not the harvest table's
  anchor (ts-oracle: `RUST-SIDE-OWNS-TERMINATION`, not SHUTDOWN). @impl/done
- ##WAL-C-QUEUE-FROM-REGISTRY **The owner's queue derives from the
  registry, never from a harvest snapshot** (the stale «F-132's nine»
  lesson). @impl/done
- ##WAL-C-PERIMETER **The perimeter law.** SPEC in `core-ai-native`, ENGINE
  in its five crates (vendored ×6), DRIVER per stack CLI, DEPLOYMENT in the
  consumer; ≥2 adopters in-tree (host + fractality); `legacy-spec/**`
  excluded; a `not-found` is a fact about the perimeter until checked. @impl/done
- ##WAL-C-READ-FURTHER **Read the document further before searching wider**
  (batch plan §6.1). @impl/done
- ##WAL-C-OWN-CORPUS **The campaign is inside its own corpus:** exclude
  `campaigns/*/run/**` from evidence; git figures name their HEAD. @impl/done
- ##WAL-C-CACHE-MERGE-ONLY **`run/cache.json` is load-and-merge only; never
  chain merge and seal; never hand-write `verified_at`/`processed_hash`.**
  Merge may hit a transient WinError 5 on the cache swap — retry, it is
  idempotent. Verdict shape: `files.<path>.campaign.verdicts.<A>.v`; print
  via `PYTHONIOENCODING=utf-8`. @impl/done
- ##WAL-C-PROGRESS-WRITES **Every parsing `vibe progress` subcommand writes
  zone state; always pass `--campaign`; never point at
  `campaigns/progress-2026-08`** (B-010). @impl/done
- ##WAL-C-SELF-CHECK-EXCLUSION **No real `vibe` command while
  `tools/self-check.sh` runs** (the floor's capture step). Steps 0c
  (CLAUDE/AGENTS/GEMINI byte-compare) and **6b (`cargo xtask
  check-codegen` — fails actionably without the machine-local jtd-codegen
  binary)** are part of the panel. @impl/done
- ##WAL-C-STAGE-EXPLICIT **Never `git add -A` while a worker is out**; stage
  explicit paths. @impl/done
- ##WAL-C-DURABLE-CITATIONS **Briefs cite durable files only; a wind-down
  invalidates evidence tables citing `CONTINUE.md`/`spec/WAL.md`.** @impl/done
- ##WAL-C-SHELL-TRAPS **Shell traps that already fired:** `grep -v '\.vibe'`
  deletes our own `org.vibevm` packages; PowerShell `-match` is
  case-insensitive; Python `str.replace` with `\n` no-ops on CRLF — use
  editor tools; Git Bash heredocs eat `\\` in inline python — use script
  files; `git commit -q` глотает вывод — контроль `echo $?`; **a
  `json.dump` over a registry file must match its indent (debt.json is
  indent=2) or the whole file reflows**. @impl/done
- ##WAL-C-BOOT-PAIR **Boot pair marking:** `spec/boot/00-core.md` /
  `90-user.md` carry owner machine facts — mark additively; `refs/book/`
  NOTOUCH. @impl/done
- ##WAL-C-DELEGATION **Delegated execution for phases E/T follows the
  owner-owned switch `campaigns/packages-2026-09/SUBAGENT-MODE.toml`
  (directive 2026-08-03):** `claudez` → Claude-Code-on-GLM workers via the
  reworked claudez/claudez2 launchers (`-c`/`-p` verified — the ALPHA/BRAVO
  matrix; two parallel lanes in worktrees for disjoint-perimeter tasks, ONE
  thread for conflict-prone many-place edits; effort max built in), `native`
  → the harness's built-in `opus5` subagents; fractality stays out (batch
  plan `#delegation`). The boss re-reads the switch before EVERY fan-out —
  a flip acts immediately. Verdicts, anchor routing and review are never
  delegated in either mode. Mechanics:
  `campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md`. @impl/done
- ##WAL-C-MISC **Small standing facts:** parse payload at
  `~/.vibe/progress-cache/…` carries no verdicts; `vibe.lock` +
  `[[mcp_server]]`/`[[binary]]` tables are the autodiscovery rails B-046
  rides; MT-02/MT-03 await the owner's manual sign-off; the redbook
  manifest carries the standing edition rule; `vibe progress baseline
  --campaign <zone>` is the boundary-baseline writer; `cargo outdated`
  cannot run over this workspace (audit 2026-08-03-03). @impl/done

## Done (collapsed — see `git log` and the §7 LOG) {#done}

##WAL-DONE **Phase D executed 2026-07-29 → 2026-08-03 and closed at a
green gate: 601 drift verdicts → 190, 94.3 % → 98.0 %.** The final day:
the F-279 ruling executed whole (schema → `core-ai-native/v0.8.0/schemas/`
+ canonical example; xtask codegen re-routed, **B-013 closed**, D42
re-judged, **F-279 resolved — 139 to history**; the
`tool:org.vibevm.ai-native/jtd-codegen` package minted — recipe, not
binary; six vendored engine copies write-through'd), the §11 gate measured
green end to end, the A–D audit ran at the gate (panel gained step 6b;
quinn-proto advisory closed host-side, DBT-0023 filed for fractality), the
close trio bound in §7.1 (`755d664a` · `dcc23250` · `9c965514`). Earlier:
the 2026-08-02 sitting drained the whole approval queue (~ten exchanges,
D29–D41, B-033…B-047, `TOOLING-MAP.md`, the pin build, step 0c); Phase C
closed 2026-07-28 at 6 847/6 847; Phase B at zero unmarked. @impl/done

## In progress {#in-progress}

##WAL-INFLIGHT **Nothing is in flight.** No delegated passes, no unsealed
merges, no uncommitted state; the mirrors get the fan-out at this
session's close. The next session's first act is a report to the owner —
Phase E does not start without his word. @impl/done

## Known issues {#known-issues}

- ##WAL-KI-OPEN **Open on the owner, none blocking:** F-129 (wal package's
  two wind-downs); F-122 (realised by волна 9's re-vendor); F-126 (tcg
  naming family — partially superseded by the F-216 repair); F-127 (Go
  `-race` 15×/0×); F-128 (`spec/boot/INLINE.md` naming); F-120 (kind-line
  guide); the H-roster (owner: `core-ai-native/appendix/`); F-069
  (aggregator grammar); the specmap ratchet's 37 gated orphans; F-125's
  three-numbers question likely closed by the 75.3/70.2 canon — verify
  before citing; the 2026-06-12-01 history-rewrite rider (third audit run
  carried). @impl/done
- ##WAL-KI-AUDIT **The audit's active subset lives in `AUDIT.md`
  (2026-08-03 section):** open — cargo-outdated unrunnable over this
  layout (-03), the dead_code-allow shadow 28 → 79 (-04, triage or accept
  next run); filed — DBT-0023 (fractality lock's quinn-proto 0.11.14,
  RUSTSEC-2026-0185; its workspace, its session). @impl/done
- ##WAL-KI-BACKLOG **The backlog carries B-001…B-047** (`BACKLOG.md` +
  `#map`; drainage shape `TOOLING-MAP.md` under the campaign frame).
  **B-013 is `done`** (2026-08-03, the F-279 closure). B-015 security
  stays parked until the owner's explicit notice. @impl/done

## Session context {#session-context}

##WAL-CTX-BOOT **A cold session starts at the campaign quick-start**
(`spec://vibevm/terraforms/packages-actualization#quick-start`), reads
`CONTINUE.md`, the batch plan §§3.6/3.7/3.8 + §6.1, §7's LOG from the end
(the 2026-08-03 close entry and the ~ten 2026-08-02 entries), the ledger's
`#close-2026-08-03` table — and takes every number from the two commands
at the top of this file. `CONTINUE.md` is the cold-resume snapshot; this
file supersedes it wherever they diverge. @impl/done
