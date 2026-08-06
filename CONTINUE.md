# CONTINUE — cold-resume snapshot (2026-08-06, wind-down №15: the course changed)

**Do not quote numbers from here — measure:**
`python campaigns/packages-2026-09/tasks/summary.py` ·
`python campaigns/packages-2026-09/tasks/judging-debt.py` ·
`python campaigns/packages-2026-09/tasks/text-stability.py` ·
`python campaigns/packages-2026-09/tasks/drift-registry.py`.
`spec/WAL.md` is rewritten by this same wind-down and **supersedes** this file.

## TL;DR

**The course changed, and that is the single most important thing to know.**
The standing course was «drain the backlog first». A long owner conversation on
2026-08-06 **authorised eighteen work items** while the session closed **one**
backlog row. It is a new programme, not a drain.

**Everything from that conversation — the items, the order, the reasoning, the
rejected alternatives and the three places the boss was wrong — is in one file:**

> ### 📄 [`spec/terraforms/OWNER-PROGRAMME-2026-08-06-CAMPAIGN-v0.1.md`](spec/terraforms/OWNER-PROGRAMME-2026-08-06-CAMPAIGN-v0.1.md)
>
> **Read it first and in full. It is the plan.** Order fixed by the owner:
> **Б (hygiene) → В (taxonomy) → А (index)**.

Nothing blocks. The panel was green at the last read. The tree is clean.

## What this session built

| | |
|---|---|
| **B-032 closed** | choosing the planning carrier is a rule now — the criterion where placement is decided, the composition half in the plan's own format, one pointer between them, citations moved, row deleted |
| **B-019(б) slice 1** | **56 command nodes** in the map (`vibe` 29, `vibe-index` 14, `xtask` 13), recognised by clap's own derive so a new subcommand cannot be added without appearing |
| **`via_redirect`** | a docblock promised two surfaces and had neither; one half made true, the other corrected |
| **fact lifecycle + debt clearance** | PROP-043 §10.1/§10.2 — what happens to a fact when it is edited, added or removed, and how the debt is cleared incrementally |
| **judging debt is measurable** | `tasks/judging-debt.py`, and every session reports it on resume |
| **B-063, B-064, B-065, B-066 filed** | markup validation in no gate · the `vibedeps` leak in the engine · the org cache · index auto-publication (the owner's blocker) |
| **`AUDIT` -04, -14, -10 re-measured** | every number moved |

## The one thing to internalise before touching anything

**Nine numbers failed to reproduce this session, and not one because of a bad
pattern.** Every time, the perimeter of the measurement was narrower than the
perimeter of the claim:

- a census of one binary's command surface taken as a count of map nodes (29);
- then a measurement scoped to `crates/` (43) — which missed a third binary
  living outside it;
- then the map itself (71) — of which **29 were false**, because two crates
  declare `pub enum Command` and the join matched on type name alone;
- **56** once the join went crate-local. One reading in between was **0**, and
  that was a stale build fingerprint, not a bug — `cargo clean -p <crate>`.

Same shape elsewhere: 57 suppressions were 55 (a comment mentioning the
attribute counted as one); «52 carry a reason» reproduces under no reading;
a doc sweep's count is wrong **for the third time** against a directory
unchanged since July; the four-layer architectural model has **zero**
occurrences in the package the backlog said it lived in.

**And the boss was wrong three times in front of the owner** — the primary
registry is GitHub, not GitVerse; the roster argument for a new package kind
was false because the boot lane already answers it; the four-layer model does
not belong in the code discipline. All three are recorded as corrections in the
programme file, because a plan that hides its wrong turns invites them again.

## Where work stands

- Branch `main`, tree clean, `.wt/` removed, every worker report archived with
  its `meta.md` under `C:\Users\olegc\git\v\cache\agents\sorted\`.
- **Panel green** at the last read (`self-check: all green`, exit 0).
- `vibe check` **clean** — the six freshness warnings are gone after the
  reinstall.
- **Corpus:** 275 files, 0 unmarked facts. **Judging debt: 47 unjudged facts in
  4 files, 5 orphan verdicts, 4 stale files** — all written this week; measure
  it, do not quote it.
- **`gitverse` is behind**; **`github` is UNREACHABLE** — ssh to
  `git@github.com` is redirected to `127.92.0.49`. Not a divergence, must not be
  forced. **The only thing needing the owner's hands.**

## Non-obvious findings worth carrying

- **A claim inside a fenced code block cannot be judged.** The owner guide's
  parenthetical «this is in the gate panel» was false and survived because it
  sat inside a ```bash block, where no anchor reaches. **Second instance of this
  law in one week.** What to do about it is named, not answered.
- **A `cd` before `( claudez -c … )` sends the correction to the repository root
  instead of the worker** — conversations are keyed by (state dir, cwd), and the
  resumed stray thread holds write access to the real tree. Caught at the
  session-start hook, zero tool calls, nothing touched. The `cd` goes **inside**
  the parentheses.
- **The seal gate refuses a file carrying any unjudged marker** — correct, and
  today the only mechanism that makes an added fact visible at all.
- **«Stale file» is not «a judged fact moved»** — a file goes stale when facts
  are merely added. A corpus can carry stale files and owe zero re-judgements.
- **Editing a spec document moves the committed map.** The recorded law spoke of
  a new file and a new edge; a five-row table edit does it too, and cost a red
  panel to learn.
- **The same five orphan verdicts were measured in July**, written into a phase
  batch plan, and were still untouched in August — filed in the one place the
  project's own rules call disposable.

## Repository map

- `spec/` — PROP/FEAT contracts (`common/`, `modules/`), `boot/`, `design/`,
  `terraforms/` (**the programme lives here**), `WAL.md`.
- `campaigns/packages-2026-09/` — the live campaign: `harvest/`, `tasks/`
  (`summary.py`, `judging-debt.py`, `text-stability.py`, `drift-registry.py`,
  `merge-verdicts.py`), `run/`, `SUBAGENT-LAUNCHERS.md` + `SUBAGENT-MODE.toml`.
- `packages/org.vibevm.ai-native/` — the discipline: `core-ai-native/v0.8.0/`
  (engines, vendored ×6 = 51 pairs), `{rust,go,typescript}-ai-native-lang/`,
  `*-mcp/`. `packages/org.vibevm.world/` — the cross-cutting flows.
- `crates/` — the host, 19 crates + xtask.
- Root: `BACKLOG.md` (51 rows), `TASKS.md`, `AUDIT.md`, `TOOLING-MAP.md`,
  `ROADMAP.md`, `specmap.json` (gated), `CLAUDE.md`/`AGENTS.md`/`GEMINI.md`
  (byte-identical).

## Decisions in force

- **A plan is temporary.** When executed it must be deletable with nothing
  breaking. Content moves **into the specifications** on closure; statements
  point at spec elements, never at plan rows; tombstones inside a plan are
  temporary process support. *(Owner, 2026-08-06 — the ruling that reshaped the
  cleanup and produced the programme file.)*
- **Content moved into a spec is judged in the same pass that moves it.**
- **Measure before building** — nineteen builds of already-built things stopped
  in three days.
- **A grep lies in both directions**, and «found nothing» is not a fact either.
- **A file-length budget never chooses the shape of a public type.**
- **`implements` is a claim about running code.**
- **Mark, don't suppress · signal, not a wall · cure the silence.**
- **Rollout is `cargo xtask mirror` only**, fast-forward, never `--force`.
- **After any engine or stack-crate edit — `cargo xtask sync-engines`** as its
  own step; a new `scope!` file or `#[verifies]` edge — `cargo xtask specmap` in
  the same landing.
- **Delegation:** claudez workers execute; verdicts, review and commits are the
  boss's.

## Recent commits

```
658ab296 docs(plan): the judging-debt question and its answer join the programme
b3a27b77 docs(session): a resume reports the judging debt, and three index rows are filed
95e25cbc feat(progress): the life of a fact under an active campaign, and how its debt is cleared
4ec11b45 docs(plan): the owner's programme of 2026-08-06, ordered and with its reasoning
90a12d77 chore(vibedeps): the installed copies catch up with a day of package edits
e24a2e24 docs(backlog): slice 1 of the command node is built, and what it cost (B-019)
95750706 feat(specmap): a command becomes an entity of the map (B-019б, slice 1)
86f110e1 docs(design): the command-node acceptance is 43, and the 29 was mine (B-019б)
65d28e10 docs(campaign): a cd before the correction sends it to the host, not the worker
be470065 chore(specmap): the map catches up with two spec edits, and the panel is why
8e101ac7 docs(backlog): the false claim survived because a fence cannot be judged (B-063)
03152b8f docs(backlog): markup validation is in no gate, and the guide said it was (B-063)
579e8a48 fix(design): five table rows in the command-node design carried no marker
9efc9293 fix(vibe-cli): a docblock promised two surfaces for the redirect stub and had neither
7573c407 docs(tasks): the second sitting of the backlog drain, and what it left the owner
d85a4a39 docs(audit): the doc sweep's count is wrong for the third time (-10)
92bd616f docs(audit): the index-schema question is not about the gate (-14)
b6dfbfa3 docs(audit): three of the dead-code row's four numbers do not reproduce (-04)
0f80a804 feat(flows): choosing the planning carrier becomes a rule, and B-032 closes
c75d153e docs(campaign): what B-032 asks for, measured before anyone builds it
21cc80ce docs(backlog): part (b) gets its design, and part (a)'s number turns out to have moved
a9098b53 docs(backlog): nothing in a manifest says a package is an AI-Native language (B-046)
8560adfc docs(backlog): the surface norm's proposed home does not exist (B-047)
27df675b chore(specmap): the command-node design enters the map
83adc55a docs(design): what it costs to make a command an entity of the map (B-019б)
78aa1ed7 docs(audit): the P1's second question gets a measured unit cost (2026-08-06-01)
34dfb52f chore(campaign): the three facts that moved with the kind-check build are re-judged
```

## Quick-start

```sh
python campaigns/packages-2026-09/tasks/summary.py
python campaigns/packages-2026-09/tasks/judging-debt.py    # what the corpus owes
python campaigns/packages-2026-09/tasks/text-stability.py
bash tools/self-check.sh              # real exit, read the tail; bare form in background
cargo xtask specmap --check
cargo xtask sync-engines              # after ANY engine or stack-crate edit
cargo xtask mirror                    # rollout, fast-forward only
```

_The WAL is the canonical living state; believe it over this file where they
diverge. The programme file is the plan; believe the owner's rulings in it over
anything derived._
