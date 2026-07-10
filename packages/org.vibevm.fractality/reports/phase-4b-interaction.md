# Phase 4b report — the interaction layer: delegation without yolo

_Campaign: FRACTALITY-IGNITION v0.1 · Phase 4b · executed 2026-07-10,
same session as Phase 4. The plan's §14 ledger stays the canonical
commit map._

## What the phase delivered

- **The ask_boss broker (D18 layer 3).** A worker's Claude Code now
  launches `fractality mcp-broker` as an MCP stdio server (hidden CLI
  subcommand, newline-delimited JSON-RPC): one tool, `ask_boss
  (question)`. Calling it parks the run — `running → waiting_on_boss`
  on the bus, `question.md` on the plane — and blocks exactly that one
  tool call until the boss answers; the reply returns as the tool
  result. The broker discovers its run and mission-control from the
  pod-injected env (`FRACTALITY_RUN_ID`, `FRACTALITY_HOME`) and never
  auto-starts daemons (a worker must not birth infrastructure).
- **The waiting_on_boss lifecycle.** Journal events `Question`/`Answer`
  with strict validation (an answer to a non-parked run is a 409; a
  re-asked question while parked updates the text; a fresh question
  clears any stale answer so the broker can never read a previous
  reply). `question`/`answer` fields fold into the run record;
  `question.md`/`answer.md` persist on the plane (I2: the bus carries,
  files record).
- **Boss verbs.** `fractality questions` (the triage inbox: id, age,
  question per line) and `fractality answer <id> [<text>|--file <f>]`.
  The sync `run` prints a loud PARKED notice with the copy-paste
  answer command, and exits with the D17 family code 4 when its wall
  cap expires while parked (the run itself keeps waiting).
- **Profile switch.** `permissions.ask_boss = true` wires everything:
  the invocation gains `--mcp-config <run>/mcp-broker.json` plus the
  `mcp__fractality__ask_boss` allow entry, the prompt gains the
  question protocol ("one precise question instead of guessing; never
  guess on destructive choices"), and the pod materializes the MCP
  config pointing at its own sibling `fractality` binary.
- **Static allowlist (D18 layer 1)** had already landed in Phase 4
  (`permissions.allow_tools` → `--allowed-tools`), pulled forward for
  MT-03's nesting demo.

## Scope honesty — what layer 2 is and is not

The plan's D18 sketch named a **pod permission broker**
(`--permission-prompt-tool` / PreToolUse `defer`) auto-deciding
allow/deny patterns per profile. v0.1 ships layers 1 (static
allowlist) and 3 (`ask_boss`) — per RP4's ruling these are "the way of
life"; dynamic per-call permission brokering is **deferred by name**
(see the deferrals note below). The practical posture: the boring
majority is pre-approved by profile; everything else fails closed; a
worker that needs a decision asks for it explicitly. What v0.1 does
NOT have: a worker blocked on a *permission prompt* (as opposed to an
ask_boss call) parks nothing — the tool call is simply denied and the
worker adapts or reports. MT-04 proves the layer that exists.

## Evidence

- **MT-04 (manual-test #4) — green first firing.** A live GLM-5-Turbo
  worker called ask_boss ~6 s after boot; `questions` showed the run;
  `question.md` carried the text; `fractality answer` resumed it; the
  worker wrote the answer **verbatim** into `greeting.txt` (it decided
  nothing itself); `wait` settled `completed exit=0`.
- **The phase prediction** (a parked worker survives ≥ 10 minutes idle
  and resumes cleanly) — checked by a dedicated firing with a
  20-minute wall budget and a deliberate 11-minute silence before the
  answer; result recorded in MT-04's addendum. A parked worker burns
  no tokens: the CC process blocks on one MCP tool result.
- Unit surface: the broker's JSON-RPC routing is a pure function
  (`route()`) with protocol-edge tests (initialize version echo,
  tools/list shape, unknown tool/method codes, notification silence);
  Question/Answer transition legality is pinned in the core fold and
  the MC validation layer.

## Important decisions

1. **The broker is the CLI binary itself** (`fractality mcp-broker`,
   hidden), not a separate executable: it reuses the client crate, the
   pod already prepends its own directory to the worker PATH, and the
   MCP config just points at the sibling binary. One artifact fewer.
2. **Park-and-block, not park-and-exit.** The worker's CC process
   stays alive, blocked on the tool call — CC's native `defer`/resume
   (F6) is the future refinement for freeing the seat; v0.1 chooses
   the mechanism that needs no session resurrection.
3. **One question slot per run.** A fresh question clears the previous
   answer; the record holds at most one open question. Multi-question
   queues are YAGNI until a real worker produces them.
4. **The wall budget bounds the wait** (per D18): a parked run past
   its `wall_secs` is budget-killed by the Phase 4 watchdog. The boss
   sizes the budget to the expected triage latency.

## Strange things / surprises

- **F15 struck the floor mid-phase**: the long-park MT-04 firing keeps
  a scratch daemon alive for ~13 minutes, and that daemon holds
  `fractality-mission-control.exe` against the floor's rebuild — the
  gate failed on a file lock, not on code. The dev law ("stop daemons
  before builds") now has a corollary: *long-running manual tests and
  floors do not share a timeline*. Sequenced around it this session;
  MC-mediated build arbitration remains the product answer (F15's
  original note).
- **tokio's stdio needs a feature flag** (`io-std`) the workspace had
  never needed before the broker — trivial, but the kind of thing that
  silently narrows which crates can host an MCP server.
- The broker's stderr goes to Claude Code's MCP log, not the pod
  transcript — worth knowing when debugging a silent ask_boss (the
  pod's worker-stderr.log will NOT show broker retries).

## Deliberately deferred / unfinished (named)

- **Dynamic permission brokering** (PreToolUse `defer` / permission-
  prompt-tool, per-pattern auto-decide) — the D18 layer 2 mechanism;
  natural Phase-5-era follow-up once real packets show which patterns
  recur.
- **settings.json materialization** in the worker config dir (the
  allow/deny lists currently ride argv only — one home, two surfaces
  when both exist).
- **Question notifications** — the boss polls `questions`; a push
  channel (hook, toast) joins the initiative-system work (Campaign 2).
- **`wait` does not surface parked states** — it waits through them
  silently (terminal-only); a `--verbose` parked echo would help swarm
  triage.

## Delegation scoreboard (the law's ledger)

- **Delegated this phase: 0** — with cause: the broker protocol seam
  (worker↔boss channel correctness) and the state-machine edges are
  exactly the "judgment × S/M" cells the new matrix keeps boss-side;
  writing the protocol tests WAS the review of the protocol.
- The phase nonetheless produced Phase-5 field data: the two context
  scenarios and the bounded-retry rule were exercised in Phase 4's
  delegations and are now codified in the delegation-rules package
  (Phase 5, same session).
