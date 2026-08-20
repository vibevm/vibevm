# The delegation decision matrix {#root}

_The routing calculus for a fractality boss: which tasks go to workers,
which stay, and on which model slot a delegated task lands. Authored
clean-room from the codex-first study note (DC1–DC6) and the IGNITION
campaign's live field data. This document is **policy**: a boss applies
it per task; the Phase-6 boot snippet loads it; the Campaign-2
initiative system will read the same axes as data._

## The one law {#law}

**Delegate when verification is cheaper than generation.**

Every routing question reduces to an economic asymmetry: the boss will
pay either way — once to *do* the task, or once to *specify* it plus
once to *verify* the result. Delegation wins exactly when

```
cost(specify) + cost(verify) < cost(do it yourself)
```

measured in boss-attention, not wall clock (worker wall clock is cheap
and parallel; boss context and judgment are the metered resource). A
task whose verification requires re-deriving the work — reading every
line with full context — has no delegation margin, whatever its size.
A task whose result a gate, diff, or acceptance command can prove
almost always does.

The seed heuristic (kept from the study, DC1): *if the prompt reads as
a work order, delegate; if writing the prompt forces the decisions, the
task is design — keep it.* The matrix below turns that instinct into a
procedure.

## The four axes {#axes}

Score the task on four axes before routing. Each axis has flat,
enumerable values — no scales, no vibes.

1. **Error cost** — what a wrong result destroys.
   - `reversible`: a bad result costs one review round; git/state can
     roll back; gates catch regressions.
   - `irreversible`: secrets, publishes, force-pushes, deletions of
     shared state, anything whose reversal costs real work or cannot
     happen at all.
2. **Context transferability** — what the worker must know.
   - `compilable`: the full context fits in a prompt the boss can
     write in minutes — exact files, exact patterns, exact commands
     (contract scenario 1).
   - `boot-loadable`: the context is large but on disk — the worker
     can be ordered to read named corpus files first (contract
     scenario 2).
   - `untransferable`: the context IS the boss's session — unstated
     owner intent, mid-conversation decisions, taste. No file names
     it.
3. **Verifiability** — how the result is proven.
   - `mechanical`: acceptance commands, gates, goldens, or a bounded
     diff read prove it (the packet's `task.acceptance` can carry the
     proof).
   - `judgment`: proving it means the boss re-reads deeply with full
     context — architecture, spec prose, security posture.
4. **Size** — honest boss-time to do it directly.
   - `S` (≤ ~15 min), `M` (≤ ~1 h), `L` (> 1 h).

## The verdict procedure {#verdict}

Apply in order; the first rule that fires decides. Steps 1–3 are
KEEP-gates; a task that survives them is delegated by step 4.

1. **Error cost `irreversible` → KEEP.** The never-delegate set
   (below) is this rule enumerated; fractality also enforces most of
   it as mechanism (I1 env isolation, D18 fail-closed tools).
2. **Context `untransferable` → KEEP.** If no prompt or corpus order
   can carry the context, the delegation would ship a guess.
3. **Verifiability `judgment` AND size `S|M` → KEEP.** Deep review of
   a small result costs as much as doing it — no margin. (Judgment × L
   is the one discretionary cell: delegate a first draft only when a
   mechanical acceptance slice exists — e.g. "draft the module, the
   tests must pass"; the judgment residue stays with the boss.)
4. **Otherwise DELEGATE**, routed by size × verifiability:
   - `S` × `mechanical` → **small slot** (`model = "small"`; one-shot,
     scenario 1).
   - `M|L` × `mechanical` → **big slot** (`model = "big"`; coarse
     one-shot, scenario 1 when compilable, scenario 2 when
     boot-loadable).
   - `L` × `judgment` → big slot draft under the discretionary rule
     above, or KEEP.

**Choosing no scenario is banned** (workspace contract law): a
delegated task is either precision-compiled (scenario 1) or ordered to
boot from named files (scenario 2). A big task with a thin prompt
produces plausible non-conformant output that costs more to review
than to rewrite.

## The never-delegate set {#never-delegate}

Hard boundary, not preference (DC2). These stay with the boss under
every economic reading:

- **Secrets and credential surfaces** — token files, auth stores,
  anything whose value must never transit a prompt or transcript
  (fractality workers cannot inherit them — I1 — and must never be
  handed them).
- **Destructive or irreversible operations** — publishes, releases,
  history rewrites, deletions of shared state, licence changes.
- **Architecture, spec, and plan authoring** — decisions that outlive
  the session are the human–boss channel's cargo; a worker's
  architecture is a guess with confidence.
- **Ambiguity-as-design** — when writing the work order would itself
  resolve the open questions, the writing IS the task (DC1).
- **Review of delegated output** — verification is the boss's half of
  the bargain; delegating it collapses the economics (nobody proves
  anything).
- **Tiny edits** — sub-minute changes cost more to specify than to do.

## Sizing the packet {#sizing}

- **Big models get coarse one-shots** (owner mandate, confirmed in the
  field): a whole module with its tests and a self-verify command
  beats five round-trips. Include: goal, exact paths, exact APIs or
  patterns to follow, non-goals, acceptance commands, output contract.
- **Small models get bounded mechanical transforms**: fixtures,
  conversions, renames, boilerplate, well-templated test suites —
  shapes where the spec is longer than the thinking.
- **Bounded retries (DC6):** one failed landing on the small slot →
  re-route to the big slot; one failed landing on the big slot → the
  boss reclaims the task. Never ping-pong a packet more than twice —
  the retry budget is part of the verification cost, and past it the
  economics have inverted.

## The boss-as-reviewer loop {#review-loop}

Delegated output is advisory until proven (DC5):

1. The packet carries `task.acceptance` — commands the pod runs in the
   workspace; pass/fail is recorded on the run, not asserted in chat.
2. The boss reads the diff as a contributor PR: does it do what the
   work order said, nothing else?
3. Gates stay the truth: the workspace floor must be green after
   merge, whatever the worker claimed.
4. Every delegation is field data — surprises (a blind spot, a stall,
   a wrong-context landing) feed the playbook of the model that
   produced them.

## Routing is data {#routing}

This procedure has an **executable form**: the fractality initiative
engine ships it as data + calculus
(`fractality-initiative/src/matrix.toml` + `route.rs`; surfaced as
`fractality route --error-cost … --context … --verify … --size …`),
and its tests pin the calculus to this document's worked-verdicts
table — the two cannot drift without failing the floor. This markdown
stays the normative prose; the engine is its projection (Campaign 2
D6).

The matrix names *slots*, not vendors: `model = "big"` / `"small"`
resolve through the run's profile (D6), so the same policy governs GLM
today and any future backend. Per-model behavior — strengths, budget
defaults, tariff rules, blind spots — lives in the
[playbooks](playbooks/), one card per model, `_template.md` for new
ones. Tariff hygiene is mechanism, not advice (D12): workers get web
tools denied by profile; documents are fetched once, locally.

## Worked verdicts {#worked}

| Task | Axes (error/context/verify/size) | Verdict |
|---|---|---|
| Write unit tests for a runner with a stated API | reversible / compilable / mechanical / S | delegate → small |
| Implement a parser + goldens from a live fixture | reversible / compilable / mechanical / M | delegate → big |
| Sweep 27 files swapping a URI scheme | reversible / compilable / mechanical / S–M | delegate → small (big if cross-file coupling) |
| Draft the swarm-phase architecture | reversible / untransferable / judgment / L | keep |
| Rotate the z.ai token | irreversible / — / — / S | keep (never-delegate) |
| Summarize a 200-page vendor doc into a fact sheet | reversible / boot-loadable / mechanical / M | delegate → big, scenario 2 |
| Fix a one-line typo the boss already sees | reversible / compilable / mechanical / tiny | keep (tiny edit) |
