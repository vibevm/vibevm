# CONTINUE — cold-resume snapshot (2026-08-06, wind-down №16)

**Do not quote numbers from here — measure:**
`python campaigns/packages-2026-09/tasks/summary.py` ·
`python campaigns/packages-2026-09/tasks/judging-debt.py` ·
`python campaigns/packages-2026-09/tasks/text-stability.py` ·
`python campaigns/packages-2026-09/tasks/drift-registry.py`
— **after** `vibe progress scan`, or they answer about the cache.
`spec/WAL.md` is rewritten by this same wind-down and **supersedes** this file.

## TL;DR

The programme of 2026-08-06 is **half executed**. Group Б is done in full,
group В is three of four, group А is untouched. 34 commits, panel green,
both mirrors synced.

The plan is still one file and it is still the plan:

> ### 📄 [`spec/terraforms/OWNER-PROGRAMME-2026-08-06-CAMPAIGN-v0.1.md`](spec/terraforms/OWNER-PROGRAMME-2026-08-06-CAMPAIGN-v0.1.md)
>
> Order fixed by the owner: **Б → В → А**. Б is closed, В1–В3 are closed,
> **В4 waits on А5**, and А is where the next session starts.

Nothing blocks. Nothing waits on the owner's hands.

## What this session built

| | |
|---|---|
| **markup migration** | `##ID` → `@fact:ID`, `@stage/state` → `@status:stage/state` — 27 407 substitutions in 327 files, by a committed reversible program. **Not one verdict moved.** |
| **`@fact/code:`** | a fence can become a fact's body — the owner's ruling D, built and applied to the fence that once lied to him |
| **group Б, all seven** | plan-closure rule · 22 citations + 5 dangling + 35 tombstones · markup validation in the panel · both verdict grains · doc-example policy · one path renderer · three facts recorded |
| **В1 omnichannel** | a capability lives in a library; host declared its floor in PROP-000 §21 |
| **В2 `lang`** | sixth package kind, amended into VIBEVM-SPEC §4.1 on the owner's instruction |
| **В3 `vibe tools`** | the registry of what a project can invoke — built as omnichannel's dogfood, two thin surfaces |
| **TUI boundary** | the boss's debt paid: finished = **a thin surface**, with the three rejected boundaries and their reasons |

## The one thing to internalise before touching anything

**The perimeter of a measurement is a claim, and it is almost always wider
than it looks.** This session got it wrong six times, and not once from a
bad pattern:

- a stale-file jump from 5 to 30 attributed to a migration that had nothing
  to do with it — the cache had simply been behind since an earlier commit;
- an argument built on a "leaky" stage dictionary that is not leaky
  (`@idea/plan` and `@impl` are both legal — the measuring script's list was
  short);
- a claimed clash between `##` and headings that **does not exist**: an ATX
  heading needs a space after the hashes;
- 42 citations counted where the rule governs 22 — the sweep included the
  disposable zone and live rows;
- the package kind missed in three JSON schemas, because the measurement
  searched for a comma-separated list and a schema writes an `enum` array;
- a fourth different count for the doc-example finding (~40 → 169 → 234 →
  174), each from a differently-drawn perimeter.

**And almost every one was caught by an instrument, not by care.** The panel
found the schemas. Conform demanded a behaviour oracle. A skill-template
test refused a tool no agent would ever be taught to call. `merge-verdicts`
refused to overwrite a verdict without an explicit word. `seal` refused a
file carrying facts this same session had left unjudged an hour earlier.

## Where work stands

- Branch `main`, tree clean, `.wt/` empty, every worker report archived
  **with its `meta.md`** (six were written this session; the archive had
  been recording testimony without a verdict).
- **Panel green**, `vibe check` clean.
- **Both mirrors synced** — github is reachable again; the "unreachable"
  entry that stood for two sessions is retired.
- Corpus 278 files, 0 unmarked facts. Verdicts ~11 870 at 98.2 %, and now
  in two grains: **62.4 % per-fact**, the rest document-level.
- **Judging debt is up, deliberately:** ~142 unjudged in 12 files. Most are
  facts this session WROTE (the omnichannel package alone carries ~56). They
  are visible, named, and cheapest to clear per file.

## Non-obvious findings worth carrying

- **One markup grammar had three readers**, and their failures ranked by
  loudness: the progress reader would have dropped facts, the map screamed
  *4654 units removed*, and the boot compiler **said nothing at all** — it
  merely stopped qualifying 466 labels, which would have collided silently.
- **The kind vocabulary is written fourteen times** and the compiler keeps
  only two honest. Two schemas had already drifted — they never learned
  `mcp`, which shipped long ago. Filed as B-070.
- **`git checkout` does not restore line endings**, because git does not
  consider them changed. A migration that rewrote them was reverted and the
  damage stayed on disk, invisible in `diff`.
- **A tool refusing to act is worth more than a tool that obliges.**
  `merge-verdicts` demanding `--force`, `seal` refusing a half-judged file,
  and conform demanding an oracle each cost a minute and saved a defect.
- **A re-judgement records what it replaces.** Erasing the earlier reading
  would make the corpus look as though it had never been wrong, and the
  ability to say "this was false, here is when" is what makes the rest
  believable.

## Repository map

- `spec/` — PROP/FEAT contracts (`common/`, `modules/`), `boot/`
  (`STATIC.md` is the priority lane — **not** `INLINE.md`, an address
  corrected in four homes today), `design/`, `terraforms/` (**the
  programme**), `WAL.md`.
- `campaigns/packages-2026-09/` — the live campaign: `harvest/`, `tasks/`
  (`summary.py` now prints both verdict grains, `judging-debt.py`,
  `text-stability.py`, `drift-registry.py`, `merge-verdicts.py`), `run/`,
  `SUBAGENT-LAUNCHERS.md` + `SUBAGENT-MODE.toml`.
- `packages/org.vibevm.world/` — 28 flows, **omnichannel is new**.
  `packages/org.vibevm.ai-native/` — the discipline; the three `-lang`
  packages are now `kind = "lang"`.
- `crates/` — 19 crates + xtask. New: `vibe-workspace/src/tools.rs`,
  `vibe-cli/src/commands/tools.rs`, `vibe-core/src/package_ref/kind.rs`.
- `tools/migrate-markup.py` — committed, reversible, self-testing.
- Root: `BACKLOG.md` (55 rows, 35 of them tombstones), `TASKS.md`,
  `AUDIT.md` (7 open), `TOOLING-MAP.md`, `specmap.json` (gated),
  `CLAUDE.md`/`AGENTS.md`/`GEMINI.md` (byte-identical).

## Decisions in force

- **A plan is temporary** — deletable once executed; content moves into the
  specs; statements cite spec elements, never plan rows; tombstones are
  process support. Two homes, one pointer.
- **A fence is an example until marked** `@fact/code:`; then it is a fact's
  body and comes due when it moves.
- **Every marker names its key** — `@fact:` / `@status:`. The legacy
  spelling is still read; the canonical form for hashing is the LEGACY one,
  which is what let 27 407 substitutions disturb no verdict.
- **A capability lives in a library**; surfaces are thin. vibevm's floor:
  library + CLI + MCP, TUI where one exists. LSP/IDE deliberately not
  declared.
- **TUI is finished when it is thin** — the deletion test, not a screen
  count.
- **`lang` is the sixth kind**; `stack` now means a family bundle only.
- **Documentation form follows the failure**: install examples qualified,
  uninstall/update short, file contents qualified.
- **Measure before building** · **a grep lies in both directions** ·
  **state the perimeter with the number**.
- Rollout is `cargo xtask mirror` only. After any engine or stack-crate
  edit — `cargo xtask sync-engines`. A new `scope!` file or edited `spec/`
  text — `cargo xtask specmap` in the same landing.

## Recent commits

```
d5857b76 docs(tui): what "finished" means for the text interface (owner ruling)
e4e9f233 test(vibe-mcp): a behaviour oracle for the list_tools cell
4c8cf648 feat(workspace): `vibe tools` — the registry of what a project can invoke (В3)
6bea3908 refactor(vibe-core): the kind vocabulary moves to its own cell
af1693b6 fix(schemas): three wire contracts learn the kinds they had fallen behind
af53bd97 feat(core): `lang` becomes the sixth package kind (В2)
f83352fa docs(backlog): the kind vocabulary is written fourteen times (B-070)
e04d83a1 feat(world): a capability lives in a library, and its surfaces are thin (В1)
9bbd84b1 feat(campaign): the verdict at the centre of the P1 is re-judged per fact
0654056c feat(campaign): four drifts close against the code that now exists (Б4, part 2)
a78044df docs(qualified-naming): which form documentation shows (Б5)
3057c07f docs: the three facts that need no decision (Б7)
00401915 fix(vibe-core): one renderer for the paths both surfaces print (Б6)
5a337e73 docs(audit): the P1's shape changes once both grains are counted
71d29e09 feat(campaign): the verdict summary prints both grains (Б4, part 1)
0907e23b build(self-check): markup validation joins the panel (Б3, B-063 closed)
e667eeb7 docs(backlog): thirty-five closed rows become tombstones (Б2, part 3)
5c121c5a docs: twenty-two citations leave the rows they were built to outlive (Б2, part 2)
e540c5f1 docs: five citations pointing at rows that no longer exist (Б2, part 1)
28a17593 feat(flows): a plan is temporary, and closing one is a defined operation (Б1)
6198a8b6 docs(progress): the fence rule is written down, and the guide uses it
9f447630 feat(progress): a fence can become a fact's body (B-068)
3a7fecf6 refactor(markup): every marker names its own key
9431dadd feat(specmap): the map reads the qualified fact anchor too
979f376a feat(progress): the markup reads a qualified spelling beside the legacy one
```

## Quick-start

```sh
cargo run -q -p vibe-cli --bin vibe -- progress scan --campaign campaigns/packages-2026-09
cargo run -q -p vibe-cli --bin vibe -- progress mirror --campaign campaigns/packages-2026-09
python campaigns/packages-2026-09/tasks/summary.py        # both grains now
python campaigns/packages-2026-09/tasks/judging-debt.py
bash tools/self-check.sh          # real exit, read the tail; bare form in background
cargo xtask specmap --check
cargo xtask sync-engines          # after ANY engine or stack-crate edit
cargo xtask mirror                # rollout, fast-forward only
vibe tools                        # NEW: what this project can invoke
```

_The WAL is the canonical living state; believe it over this file where they
diverge. The programme file is the plan; believe the owner's rulings in it
over anything derived._
