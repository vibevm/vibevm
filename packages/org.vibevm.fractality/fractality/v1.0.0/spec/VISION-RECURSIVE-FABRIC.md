# VISION — the recursive fabric {#root}

_Genre: **vision / design lore** (flow:spec-genres) — the owner's
directional mandate, recorded 2026-07-10 ahead of Campaign 3.
**Non-binding as contract:** normative shapes are extracted into
PROPs by the campaigns that build them; on divergence
[`PROP-001`](PROP-001-foundation.md) and the campaign plans win.
Two-way link: PROP-001 §7 (horizons) names this file. The owner's
purpose, verbatim: «сквозь него стоит посмотреть на ближайшие
шаги» — near-term steps are to be read through this lens._

## 0. The mandate (owner, 2026-07-10, verbatim) {#mandate}

> Любой агент должен иметь возможность стать боссом. Обычного
> агента промоутит до босса его босс (в интерактивном режиме либо
> перезапускает). То есть, это рекурсивная вложенность боссов
> произвольной степени, при необходимости. Без необходимости не
> нужно, и необходимость вычисляется по разным критериям (агент
> должен быть моделью medium или big, задача должна быть сложной и
> иметь большую глубину, и так далее — это нужно учесть при
> рождении нового агента-подчиненного или при промоушене его
> интерактивно.
>
> Должна быть реализация RLM-спуска и всего RLM-процесса.
>
> Любой агент на любом уровне вложенности должен иметь возможность
> эскалировать задачу вверх иерархии, если не может решить ее —
> например, ему нужны дополнительные сведения.
>
> Любой агент вместо эскалации может позвать в качестве advisor
> более большую модель в иерархии (Mythos > Fable > Opus > GLM 5.2
> > Sonnet > GLM-5-Turbo > Haiku), если большей нету (ты на вершине
> иерархии по той или иной причине, например Opus но доступа до
> Fable нет) то модель может спросить совета модели своего размера
> но другой (желательно, например Opus лучше спросить GPT-5.5 или
> GPT-5.6) либо модели того же размера.
>
> Понятно, что у нас своя специфика, у нас есть поды и так далее,
> надо ее учитывать. И всё это в контексте RLM процедуры.

(Numbering normalized to four pillars below; the owner's message
numbered them 1/2/3/3. Two reference schematics accompanied the
mandate — reproduced in §2.)

### Addendum — attachable terminals (owner, 2026-07-10, later the same session, verbatim) {#mandate-addendum}

> Один из важных апгрейдов в будущем: возможность запускать
> отдельные агенты не просто как процессы, а как полноценные
> виртуальные headless-терминалы. К которым потом можно
> подключиться через GUI и посмотреть, что там внутри. Структуру с
> mission control и подами это не отменяет, это её апгрейд — под
> будет запускать headless-терминалы. Которые будут полноценными
> терминалами, с которыми потом можно будет подключиться, например,
> из Gnome Terminal или Warp, и работать интерактивно. Этот факт
> может оказать важное влияние на какие-то последующие технические
> решения организации общения mission-control, pod, агентов, и так
> далее.

(Recorded as pillar V5 below.)

## 1. The pillars {#pillars}

### V1 — any agent can become a boss {#v1-promotion}

**Boss is a role, not a node type.** Today PROP-001 §2 draws boss
and worker as different species (interactive session vs headless
packet-runner); the run tree already lets runs spawn child runs.
V1 goes further: every agent in the fabric carries *latent* boss
capability, and the fabric can install the boss surface onto it.

- **Who promotes:** the agent's own boss — never self-promotion.
  Two paths: **at birth** (a subordinate is spawned already wearing
  the boss surface because its task warrants it) or **interactive
  promotion** (the boss restarts / re-launches a running agent with
  the boss surface added).
- **Recursion, bounded by need:** nesting of bosses to arbitrary
  depth *when needed*; flat by default. Depth is a cost, not a
  virtue.
- **The need is computed, not assumed** — criteria the owner named:
  the agent's model class must be **medium or big** (a small model
  never carries a boss surface); the task must be complex and deep
  enough to amortize a delegation layer; "and so on" — the list is
  open and belongs in **policy data** (the `delegation-rules`
  package; D6 "routing becomes data"), evaluated at every
  subordinate birth and at every interactive promotion.

### V2 — the RLM procedure, whole {#v2-rlm}

**RLM descent and the full RLM process must exist in the fabric.**
PROP-001 §1 already names the shape: the RLM pattern is fractality's
self-similarity applied to *context* instead of *tasks*. The vision
insists on the complete procedure — the descent (a boss recursively
decomposing over cheap sub-calls rather than swallowing a giant
context whole), and everything around it (how results ascend and
aggregate, how budgets meter the recursion).

Clean-room note: the reference sources are inventoried
([`refs/INVENTORY.md`](refs/INVENTORY.md) S3 — the MIT reference
implementation, S4 — arXiv 2512.24601), with deep study
deliberately deferred to Campaign 3; `notes/rlm-study.md` is that
campaign's opening act. This document records *intent*, not the
method — no design decisions are made here ahead of the study note.

### V3 — escalation up the hierarchy {#v3-escalation}

**Any agent at any depth can push its task UP** when it cannot
solve it — missing information, insufficient capability, a blocked
environment. Descent (delegation) without a matching ascent channel
produces silent failure at the leaves; V3 makes "I cannot do this,
here is what I need" a first-class outcome, not a failure mode. The
top of every chain is the human owner.

Embryo today: parked questions + profile-level answer rules (the
Ф5 slice) and `fractality answer` — a question channel. V3
generalizes it into a *task* channel: the packet itself can come
back up, annotated with what is missing.

### V4 — advisor instead of escalation {#v4-advisor}

**A sideways channel that transfers nothing but judgment.** Instead
of escalating (giving the task back), an agent may consult a
**bigger model** and keep ownership: the advisor is called
on-demand, returns advice, and holds no loop, no task, no state.

The capability ladder (owner, verbatim order):

```
Mythos > Fable > Opus > GLM 5.2 > Sonnet > GLM-5-Turbo > Haiku
```

- Default: advise with a rung **above** the caller.
- **At the effective top** (no bigger rung reachable on this box —
  e.g. an Opus boss without Fable access): prefer a peer-size model
  of a **different family** (e.g. Opus consulting GPT-5.5/GPT-5.6),
  else a same-size model.
- The ladder is **policy data, not hardcode** — providers, model
  names, and availability drift per box and per month; the
  `delegation-rules` matrix and profiles decide which rungs exist
  here (PROP-001 §2 profiles make the fabric model-agnostic).

### V5 — attachable headless terminals (the runtime upgrade) {#v5-terminals}

**Workers upgrade from bare processes to attachable terminal
sessions.** A future fabric launches each agent not as a headless
process with piped stdio, but as a **full virtual terminal running
headless** — a real PTY-backed session a human can later *attach
to* from a GUI terminal (Gnome Terminal, Warp, …) and work in
interactively, seeing everything that happened inside.

- **Nothing about MC/pods is displaced** — owner verbatim: this is
  an upgrade of the structure, not a replacement; *the pod launches
  the headless terminals*. The pod stays the launcher; the launched
  unit gains a terminal identity on top of its run identity.
- **Design-shaping now, build later.** The owner flags this may
  importantly influence upcoming decisions about MC ↔ pod ↔ agent
  communication. Near-term designs must not foreclose it:
  - **The transport seam:** never bake in "a worker's stdio is a
    private pipe owned by MC alone". The capture path
    (`worker-stdout.jsonl`) must tolerate the stream source being a
    PTY with **multiple readers** — and, once a human attaches,
    **keystrokes arriving mid-run** from outside the fabric.
  - **Files stay the persistence plane (I2):** attach is a live
    *view*; the durable record still lands in the run dir
    regardless of who is watching.
  - **Attach is an access surface:** authenticated, and journaled
    like any other event (I3) — who attached, when, to which run.
- **Synergies with the other pillars:**
  - **V1 interactive promotion gets its natural mechanism** —
    attach to a running subordinate, judge it live, re-launch it
    wearing the boss surface.
  - **The live-observation law gets its product form** — today it
    is hand-run (log files + watchers + heartbeat markers); an
    attachable terminal replaces tailing with looking.

## 2. The two reference schematics {#schematics}

The owner supplied two standard patterns to fix the structural
difference (reproduced from the images accompanying the mandate):

```
  ORCHESTRATOR pattern                     ADVISOR pattern

  ┌──────────────┐   fan out   ┌──────────┐    ┌─────────────┐  tool call   ┌───────────┐
  │ Orchestrator │ ──────────► │ Worker 1 │↺   │  Executor   │ ───────────► │  Advisor  │
  │   (Fable 5)  │ ──────────► │ Worker 2 │↺   │  (Sonnet 5) │ ◄┄┄┄┄┄┄┄┄┄┄┄ │ (Fable 5) │
  │  plan · main │ ──────────► │ Worker 3 │↺   │ runs every  │ sends advice │ on-demand │
  │  loop ↺      │             │(Sonnet 5)│    │ turn · main │              └───────────┘
  └──────────────┘             └──────────┘    │ loop ↺      │
                              (worker loops)   └─────────────┘
```

Structurally: the **orchestrator owns the loop and transfers
execution** (fan-out; big model plans, small models work); the
**advisor transfers nothing** (the executor — typically the smaller
model — keeps the main loop and ownership; advice is an input, not
a hand-off). Note the model placement inverts between the patterns.

The fabric's three channels, side by side:

| channel | direction | what moves | who owns the loop | fractality seam |
|---|---|---|---|---|
| delegation (V1) | down | execution (a packet) | the subordinate | MC spawn / run tree — exists today |
| escalation (V3) | up | the task itself, annotated | the superior | packet outcome + answer channel — embryo (Ф5) |
| advice (V4) | sideways-up | judgment only | the caller, unchanged | does not exist today — new call shape |

## 3. Fractality specifics the vision must respect {#specifics}

Named so the vision is read *through* the fabric we actually have —
pods, MC, files — not as a green-field diagram:

- **The run tree is the recursion substrate** (PROP-001 §2): child
  runs are already first-class; V1 adds *delegation authority* at
  interior nodes, not a new process shape. Promotion = MC granting
  a run the boss surface (CLI access + initiative hooks), most
  honestly by re-launch in v1 (a headless CC process cannot
  hot-swap its surface).
- **I1 (worker-env whitelist)** binds every new channel: an advisor
  call across providers must not leak `ANTHROPIC_*`/`CLAUDE_*` any
  more than a worker spawn does; cross-family advisors mean new
  token surfaces under the same secrets-hygiene laws.
- **I2/I3 (files persist; one telemetry store):** escalations and
  advisor calls are journaled events with prices, like every spawn
  — meta-cognition over "who asked whom, at what cost, and did it
  help" is the §1 far-horizon promise and needs the data recorded
  from day one.
- **D7 (strictly factual surfaces):** any surface that *suggests*
  promotion, escalation, or an advisor cites recorded facts — which
  is exactly the acceptance-backed credibility plumbing of
  [PP-002](../../plans/postponed/PP-002-def-c2-2b-worker-credibility.md).
- **Budgets are recursive already** (PROP-001 §1 quotas): depth
  limits and "whose budget pays for the advisor" are metering
  questions, not new machinery.

## 4. Near-term steps through this lens {#near-term}

- **PP-001 (rule RP5, fire MT-C2-05):** unchanged and *more*
  urgent — a flat boss's cold-delegation propensity is the base
  case of recursive propensity. If visibility + nudges cannot move
  one boss to delegate once, depth multiplies that zero. Baseline
  before Campaign 3.
- **PP-002 (credibility facts):** upgraded in weight — the same
  acceptance-schema plumbing is the substrate of V1's need
  computation ("is this subordinate worth a boss surface?") and
  V4's advisor economics ("did advice ever change an outcome?").
  Candidate to fold into Campaign 3's mandate rather than stand
  alone.
- **Campaign 3 (RLM, DEF-2):** the mandate should be cut
  *deliberately* across the pillars — V2 descent alone, or V2 +
  the ascent/sideways channels (V3/V4), or staged campaigns. This
  document exists so that whatever is cut out is cut by decision,
  not by omission.
- **`delegation-rules`:** the ladder (§1 V4), the promotion
  criteria (§1 V1), and escalation/advice policy are matrix
  columns — data, versioned in the policy package, per D6.
- **MC ↔ pod ↔ agent transport decisions:** any upcoming protocol
  work (C3's RLM plumbing, PP-002's acceptance schema, bus/stream
  changes) treats worker I/O as *terminal-session-shaped*, not
  pipe-shaped — or at minimum keeps the seam where a PTY-hosted,
  attachable worker (V5) slots in without a rewrite.

## 5. Open questions (for the Campaign 3 mandate) {#open-questions}

- Interactive promotion mechanics: re-launch with the boss surface
  is the honest v1 — is in-place promotion (a running agent gaining
  tools mid-flight) ever worth the complexity?
- Depth guardrails: a hard depth cap in policy, or budgets alone?
- Advisor accounting: the caller's budget line, a dedicated advice
  budget, or the superior's?
- Advisor plumbing: is an advisor a worker-shaped run with an
  `advice` packet type (packets outlive backends — likely), or a
  lighter direct call?
- Cross-family advisors (GPT-5.5/5.6) require new backend adapters
  — sequencing vs the Codex/VibeVM-Pixel backends PROP-001 §7
  already names.
- Terminal-session substrate (V5): what hosts the PTY — a
  multiplexer we own (tmux/screen-shaped), an SSH-served session,
  or an OS facility (ConPTY on Windows vs Unix pty — the named GUI
  clients are Linux/macOS-first, this dev box is Windows)? And what
  do Gnome Terminal / Warp actually speak to attach — most likely a
  client command the run exposes (`fractality attach <run>`), not a
  bespoke protocol?
- Does an attached human's interactive turn burn the run's budget,
  and how does attach interact with kill-trees (may MC reap a run a
  human is sitting in)?
