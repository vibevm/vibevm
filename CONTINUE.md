# CONTINUE — cold-resume snapshot (2026-08-06, wind-down №17)

> ## ⚠️ ЭТОТ ФАЙЛ ОТСТАЛ. Актуальное состояние — 2026-08-09
>
> Работа, которую этот снимок называет следующей (А5b → аудит TUI → хвост P1 →
> дешёвые строки), **исполнена 2026-08-06**. Всё, что ниже раздела «Where work
> stands», по-прежнему верно как карта репозитория и список действующих решений;
> **список следующих работ — нет**.
>
> **С тех пор** сессия 2026-08-09 провела исследование эволюции форматов
> (вопрос владельца о генерации типов каталога из схем). Три круга: четыре
> веб-исследователя, пять GLM-воркеров, финальное ревью отдельной моделью.
> Решения нет — до дизайна не дошли.
>
> **Всё состояние этой работы — в хэндоффе, ВНЕ репозитория:**
>
> ```
> C:\Users\olegc\git\v\discovery\vibevm-schema-evolution-discovery\12-HANDOFF.md
> ```
>
> Загрузочная последовательность туда не заглядывает — этот указатель
> единственный. В хэндоффе: пять вопросов владельца с состоянием каждого, полный
> список обязательного чтения (12 документов, ~12 400 строк), пять мест, где
> прежний разбор экономил усилия, и что в каждом меняется.
>
> **Промт для следующей сессии** — [`NEXT-SESSION-PROMPT.md`](NEXT-SESSION-PROMPT.md),
> переписан 2026-08-09. Он требует прочитать материал **до** разговора с
> владельцем и запрещает начинать работу без его слова.
>
> **Новая директива владельца, действует всегда:** усилий не экономить; объём
> работ не является доводом. Записана в `spec/boot/90-user.md`, читается при
> загрузке каждой сессии.

**Do not quote numbers from here — measure:**
`python campaigns/packages-2026-09/tasks/summary.py` ·
`python campaigns/packages-2026-09/tasks/judging-debt.py` ·
`python campaigns/packages-2026-09/tasks/text-stability.py` ·
`python campaigns/packages-2026-09/tasks/drift-registry.py`
— **after** `vibe progress scan`, or they answer about the cache.
`spec/WAL.md` is rewritten by this same wind-down and **supersedes** this file.

## TL;DR

**The programme of 2026-08-06 is executed as far as it can go without the
owner.** Group Б was closed in the previous session; В1–В3 too; this session
closed **А1, А2, А3, А4 and А5a**, and stopped **А6** and **В4** on
measurements that change what the owner decided rather than on difficulty.

**The corpus owes nothing unjudged and nothing orphaned** — 142 and 5 this
morning, both zero now. Ten files sealed.

The next session's work is named in [`NEXT-SESSION-PROMPT.md`](NEXT-SESSION-PROMPT.md),
in the order the owner asked for: **А5b → the TUI thinness audit → the P1 tail
→ the cheap rows → then drain `BACKLOG.md` and refresh `TOOLING-MAP.md`.**

The plan file is still the plan for what remains of the programme:

> ### 📄 [`spec/terraforms/OWNER-PROGRAMME-2026-08-06-CAMPAIGN-v0.1.md`](spec/terraforms/OWNER-PROGRAMME-2026-08-06-CAMPAIGN-v0.1.md)

## What this session built

| | |
|---|---|
| **А1** | the index server publishes itself — and **refuses to start** if `state/` is not gitignored, because publishing is a `git add -A` away from the server's own bearer tokens |
| **А2** | a private index becomes readable. The unmeasured question was answered first: the client could not authenticate **at all**. The half a mechanical reading would miss — a refused probe is now distinguishable from a missing index |
| **А3** | the organisation image is cached, with a cheap conditional freshness check that is what makes «on by default» honest rather than an improvement on it |
| **А4** | webhooks specified with the operator guide **inside the spec**, on the owner's ruling; its fenced walkthrough deliberately unmarked, because it describes a route that does not exist |
| **А5a** | the map can be **searched**, not only asked about — three filters under a hard ceiling, library + CLI + MCP |
| **debt** | 176 facts judged, ten files sealed, unjudged and orphaned both to zero |

## The three things worth reading before touching anything

**1. Instruments caught six errors; attention caught none.** The map's ratchet
found five untagged items (my packet had dropped the `scope!` clause — the
documented failure mode of a packet assembled mid-session). Conform caught two
environment reads outside its sanctioned list, one of them pre-existing and
newly unsanctioned only because its file had moved. Conform again caught an
`.unwrap()` hiding behind a real invariant and a false assumption. The length
budget threw a file over 600 **after formatting**, twice. `merge-verdicts`
refused two batches — and one refusal proved my own evidence text was lying
about itself.

**2. A measurement refused, and it saved five verdicts.** Five «orphaned
verdicts» had stood since 2026-07-28. The anchors were all present; what was
lost was **addressability**, to a missing blank line — two facts on consecutive
lines are one paragraph, and only the first keeps an address. The repair was
three blank lines. The prepared repair — prune the five — would have destroyed
five valid judgements to tidy a number. Filed as **B-074**: in this markup
whitespace is load-bearing, and the one mechanism that can lose an address
leaves no trace in any gate.

**3. The corpus's largest evidence blob names a file that has never existed.**
`PROP-005` carries 279 verdicts across four paragraphs, one of which covers
**276**. Inside it: «the shipped JTD at
`crates/vibe-index/schemas/index-entry.jtd.json`». No such file, no such
directory, and `git log` across all refs shows it was never added or deleted.
The earlier P1 specimen was findable because the corpus contradicted itself;
this one has no contradicting twin. Recorded in `AUDIT.md` under the open P1.

## Where work stands

- Branch `main`, tree clean, `.wt/` empty (and now gitignored), every worker
  report archived **with its `meta.md`** — four workers this session.
- **Panel green** (`bash tools/self-check.sh`, exit 0, read from the tail).
- **26 commits ahead of origin — NOTHING IS PUSHED.** The rollout is
  `cargo xtask mirror` and this wind-down runs it; if it did not, that is the
  first thing to check.
- Corpus 278 files. Verdicts ~12 065 at 98.2 %, **63.0 % per-fact**.
- Judging debt: **0 unjudged, 0 orphaned**, 42 stale (mostly this session's own
  edits — `text-stability.py` names which facts actually moved).

## Non-obvious findings worth carrying

- **A dated measurement is kept dated, not rewritten.** `command-nodes.md`'s
  «what is measured today» section had four readings moved by the very build it
  commissioned. Rewriting them would keep the conclusion and erase the
  reasoning; the section is reframed as evidence for a decision and one fact
  carries the re-measurement.
- **A review correction can destroy coverage.** The `-c` round I sent А2 was
  right and removed the only tests of the positive case — the mock servers are
  plain HTTP, so once the scheme gate moved to the attachment step those tests
  became false. The gap was mine; I closed it rather than spending a third round.
- **`vibe tools` shipped with no spec document at all** — its only durable
  record was a disposable plan and a rewritten WAL. А5a deliberately did not
  repeat that.
- **A worker's refusal is the useful output.** А1 reported that the index stamps
  a fresh timestamp on every write, so a redundant upsert still commits, and did
  not work around it. That is B-072.

## Repository map

- `spec/` — PROP/FEAT contracts (`common/`, `modules/`), `boot/` (`STATIC.md`
  is the priority lane), `design/`, `terraforms/` (**the programme**), `WAL.md`.
- `campaigns/packages-2026-09/` — the live campaign: `harvest/`, `tasks/`
  (`summary.py`, `judging-debt.py`, `text-stability.py`, `drift-registry.py`,
  `merge-verdicts.py`), `run/`, `SUBAGENT-LAUNCHERS.md` + `SUBAGENT-MODE.toml`.
- `crates/` — 19 crates + xtask. New this session: `vibe-index/src/publish.rs`,
  `vibe-index/src/scanner/org_cache.rs`, `vibe-index/src/cli/rescan_org.rs`,
  `vibe-registry/src/index_client/` (was one file),
  `vibe-trace/src/search.rs` + `search/tests.rs`, `vibe-cli/src/commands/query.rs`,
  `vibe-mcp/src/tools/query.rs`.
- Root: `BACKLOG.md` (**21 live rows, 38 tombstones**), `TOOLING-MAP.md`
  (**dated 2026-08-04, two days behind the tree — one fact now says so**),
  `TASKS.md`, `AUDIT.md`, `NEXT-SESSION-PROMPT.md`, `specmap.json` (gated),
  `CLAUDE.md`/`AGENTS.md`/`GEMINI.md` (byte-identical).

## Decisions in force

- **A capability lives in a library; surfaces are thin.** Floor: library + CLI +
  MCP, TUI where one exists. LSP/IDE deliberately undeclared — and an
  undeclared surface is not a debt.
- **A plan is temporary** — deletable once executed; content moves into the
  specs; statements cite spec elements, never plan rows.
- **Content moved into a specification is judged in the same pass that moves
  it.** Enforced by `seal` refusing a half-judged file.
- **A re-judgement records what it replaces**, including that the earlier
  verdict was correct when formed.
- **A fence is an example until marked** `@fact/code:`.
- **Every marker names its key** — `@fact:` / `@status:`. The canonical form
  for hashing is the LEGACY one.
- **The TUI is finished when it is thin** — deletion on paper. Perimeter now
  written down: the two `tui/` trees, 63 files, 18 426 lines, 258 tests.
- **State the perimeter with the number** · **measure before building** ·
  **a grep lies in both directions**.
- Rollout is `cargo xtask mirror` only. After any engine or stack-crate edit —
  `cargo xtask sync-engines`. A new `scope!` file or edited `spec/` text —
  `cargo xtask specmap` in the same landing.

## Recent commits

```
38d2d2d3 docs(backlog): three rows stop being true, and the map says how far behind it is
609215dc docs(backlog): В4's engine boundary was settled; its consumer boundary was not
471e3b1b feat(trace): the map can be searched, not only asked about (А5a)
57d9e8c4 feat(index): the organisation image is cached, and a cheap check keeps it honest (А3)
d51723c8 docs(backlog): a second fact anchor in one paragraph is swallowed silently (B-074)
d77a975b fix(design): three blank lines, and the corpus owes nothing on either count
f13ad415 docs(backlog): generating the index's wire types costs more than the table said (B-073)
c0b6d7d4 docs(audit): the largest evidence blob names a file that has never existed
bd5eeeec docs(design): command nodes judged — and the corpus owes nothing unjudged
b0e5c797 docs(campaign): the omnichannel protocol judged; the package now owes nothing
cb6cf79d docs(campaign): the omnichannel package's own README, judged and sealed
d5700553 docs(campaign): the omnichannel boot snippet is judged and sealed
ffdd3525 docs(campaign): the markup and debt laws, judged by the session that ran them
e74b23a1 docs(campaign): a plan's temporariness, judged against a day of closing plans
c45faff1 docs(campaign): the two-homes rule judged, and four files stop being stale
ec380862 docs(campaign): the plan-closure law is judged against the tree that follows it
7410aaf5 chore(git): worker worktrees become unstageable, not merely un-staged
ed2ca404 feat(registry): a private index becomes readable (А2)
5f486cd7 docs(backlog): two findings the А1 review turned up (B-071, B-072)
8a8aba12 feat(index): the server carries its own result to the host (А1)
18c3dcd8 docs(tui): the boundary's own measurement gets the perimeter it never stated
bf4703e1 docs(campaign): the surface-floor facts are judged, one by one
be3fc4f4 feat(index): webhooks, and the guide that changes when the contract does (А4)
ece4505b docs(progress): the owner's guide stops teaching the spelling it retired
dc8bdb86 fix(campaign): the stability instrument reads the markup the corpus now writes
```

## Quick-start

```sh
cargo run -q -p vibe-cli --bin vibe -- progress scan --campaign campaigns/packages-2026-09
cargo run -q -p vibe-cli --bin vibe -- progress mirror --campaign campaigns/packages-2026-09
python campaigns/packages-2026-09/tasks/summary.py
python campaigns/packages-2026-09/tasks/judging-debt.py
bash tools/self-check.sh          # real exit, read the tail; bare form in background
cargo xtask specmap --check
cargo xtask sync-engines          # after ANY engine or stack-crate edit
cargo xtask mirror                # rollout, fast-forward only
vibe query --kind command --limit 5   # NEW: search the map
vibe-index rescan-org --help          # NEW: the unconditional org walk
```

_The WAL is the canonical living state; believe it over this file where they
diverge. `NEXT-SESSION-PROMPT.md` carries the order the owner asked for._
