# PROP-001 — fractality: foundation {#root}

_Status: in force, 2026-07-09. Owner-commissioned. This document carries the
vision, the system model, the architecture, and the invariants; the
executable roadmap lives in
[`plans/FRACTALITY-IGNITION-PLAN-v0.1.md`](plans/FRACTALITY-IGNITION-PLAN-v0.1.md)._

## 1. Vision {#vision}

fractality is the earliest form of an **agent operating system**. The
analogy is load-bearing, not decorative — each classical OS concern has an
agentic counterpart, and the product grows by filling this table honestly:

| OS concern | fractality counterpart |
|---|---|
| process | a **run**: one worker agent executing one task packet |
| scheduler / init | **mission-control**: spawns, supervises, adopts, reaps |
| process table, `/proc` | the run registry + call tree + metrics API |
| fork / call stack | runs spawning child runs; the tree is first-class |
| pipes / IPC | mission-control messages — the bus; files persist what flowed |
| ulimits / quotas | budgets: wall clock, turns, tokens, spend |
| users / credentials | **profiles**: provider, models, auth, permissions |
| kill, process groups | recursive kill of whole worker process trees |
| accounting | the journal: every event, every token, every price |

The economics that motivate all of it: one expensive, high-judgment **boss**
(Claude Code on the owner's Max subscription — today Opus-class, tomorrow
Mythos-class when it returns) should spend itself on judgment, review, and
architecture, while **swarms of cheap workers** (GLM 5.2 / GLM-5-Turbo via
z.ai's Claude Code integration, later other backends) do the
compute-intensive, delegable work on a budget the owner pays to that
provider. The boss's scarce context stays clean because workers return
compact files, not transcripts.

The far horizon, named so the near-term design never forecloses it:
mission-control on a dedicated server, federating agents across machines;
analytics and GUIs over its journal; the same metrics feeding
**meta-cognition** — agents (and humans) reasoning about how the swarm
thinks and where it wastes. This is the young-kernel stage: small, correct,
observable primitives first.

The name: **fractality** — delegation is self-similar. A worker may spawn
workers; the call tree at depth N looks like the tree at depth 1; the RLM
pattern (Campaign 3) is this same shape applied to *context* instead of
*tasks*. (And an homage in the spirit of Google's "Antigravity".)

## 2. System model and glossary {#model}

- **boss** — the interactive, expensive agent session that decides *what* to
  delegate. Not a fractality process; fractality serves it a CLI (later an
  MCP surface).
- **worker** — a headless agent process executing one packet. v0.1: Claude
  Code in non-interactive mode under a provider profile.
- **backend** — the adapter that knows how to spawn and read one kind of
  worker (`claude-code` first; the seam where Codex, VibeVM Pixel, or a bare
  API runner arrive later).
- **profile** — a named provider configuration: base URL, token *reference*,
  model slot mapping, permission posture, tool allow/deny, concurrency
  limits, prices. Profiles make the fabric model-agnostic.
- **task packet** — a versioned TOML file: goal, context, workspace mode,
  output contract, budgets, routing. The universal seam — packets outlive
  any particular backend.
- **run** — one packet executed by one worker: ULID identity, states
  `registered → running → done | failed | killed`, a run directory with the
  packet, transcript, result, and usage.
- **run tree** — runs spawned from within runs (the worker's environment
  carries its run id; nested spawns register as children).
- **mission-control (MC)** — the daemon owning spawn, supervision, budgets,
  kill, the registry, the journal, and the metrics API.
- **swarm** — N runs fanned out over a task decomposition, bounded by
  profile concurrency and budgets.
- **node** — the machine a process runs on: stable machine id +
  hostname + addresses, recorded by MC at registration and exposed to
  every run (`FRACTALITY_NODE_ID`, `fractality node`).
- **filesystem scope** — a registered storage area (e.g. the runs root)
  with proven identity: a rendezvous beacon file plus mount metadata.
  Two parties share a scope iff they read the same live beacon.
- **FileRef** — the claim-check: (scope, scope-relative path, byte
  range) — how bulk data rides the bus without riding the bus.

## 3. Architecture {#architecture}

```
 boss (Claude Code, Max sub)                 zai / other provider
 │  Bash: fractality run/spawn/…                     ▲ HTTPS (worker's own auth)
 ▼                                                   │
 fractality CLI ──HTTP localhost──► fractality-mission-control
 (thin client)                        │ owns children, pipes, budgets,
                                      │ job objects / process groups
                                      ▼
                            worker: claude -p (headless)
                            env built from scratch per profile
                            cwd: git worktree / scratch dir
                                      │
                                      ▼
                    run dir on disk (persistence plane, I2):
                    packet.toml · worker-stdout.jsonl · result.md
                    files/ · usage.json · status.json
```

Crate decomposition (one Cargo workspace inside this package, PROP-024
shape): `fractality-core` (domain: packets, runs, events, API DTOs; the
`WorkerBackend` trait), `fractality-backend-claude-code`,
`fractality-mission-control` (bin), `fractality-mc-client`,
`fractality-cli` (bin `fractality`). The CLI stays thin — spawn logic,
process ownership, and metering live in MC, because only a single
long-lived supervisor can own kill-trees, adoption after crashes, and
cross-run accounting.

### 3b. Boss sessions {#sessions}

Mission-control also records **boss sessions** (Campaign 2 — the
initiative system): one record per boss conversation in some harness,
registered by that harness's adapter at session start and closed at its
end. The harness name is a label (I4), never a code path. Runs carry an
optional `origin_session` — a best-effort attribution label exported
into the boss's environment at session start (`FRACTALITY_BOSS_SESSION`)
and read by the CLI at spawn time; a dangling label never invalidates a
run. Session events ride a sibling journal file (`sessions.jsonl`) with
its own replay fold, leaving the run journal untouched; the record
accumulates **facts only** (work-tool events, delegations — which zero
the work-tool slate, nudges sent, question alerts). All initiative
*policy* — thresholds, cooldowns, texts — lives in the client-side
engine, so MC stays the one telemetry store (I3) and never a policy
brain. The variable never enters a worker environment (I1): worker-side
spawns attribute through `FRACTALITY_RUN_ID` parenting.

## 4. Invariants {#invariants}

- **I1 — worker-env hygiene (security).** A worker's environment is
  constructed from a whitelist plus its profile. It **never** inherits
  `ANTHROPIC_*` / `CLAUDE_*` / harness-identity variables from the parent.
  This is what makes "the swarm cannot silently bill or impersonate the
  boss's subscription" a property, not a hope. Unit-tested; weakening it is
  an owner-level review point.
- **I2 — two planes: mission-control is the bus, files are the
  persistence.** Every command and every boss↔worker exchange flows
  through mission-control's API (boss ↔ MC ↔ pod ↔ worker). Files are the
  guaranteed, ultimate persistence of everything that flowed — run dirs
  as the durable record, potentially on distributed storage (NFS/Ceph) in
  the federation era — but **not the communication medium**. (Owner
  rulings, 2026-07-09: results are *delivered* as files — «передачу всех
  результатов нужно делать ТОЛЬКО через файлы на диске» — refined the
  same day: «весь командный интерфейс проходил через mission control.
  Файлы — это просто форма гарантированного, ультимативного персистенса
  […] но это не средство коммуникации».) Consequence: every CLI verb
  resolves through MC, so nothing breaks when MC moves off-box; humans
  may still open run dirs directly — the escape hatch stays literal.
- **I3 — one telemetry store.** MC's journal is the single accumulator of
  runs, events, usage, and profiling metadata; every consumer — `stats`,
  scoreboards, the future initiative system, GUIs, meta-cognition — reads
  MC's API. Nothing else accumulates its own shadow state. (Owner ruling,
  2026-07-09.)
- **I4 — agent-neutral core.** Backends and boss-side adapters are edges;
  packets, runs, profiles, and the journal are the center and mention no
  vendor. Claude Code specifics (flags, stream formats, hooks) live only in
  `fractality-backend-claude-code` and in boss-side skill/boot artifacts.
- **I5 — workers are uninstrumented (v0.1).** A worker needs zero
  fractality-awareness: it reads its task from the packet prompt, works,
  writes files. All supervision is done from outside (pipes, exit codes,
  transcripts). Worker-side telemetry hooks are a later, optional layer.
- **I6 — secrets never surface.** Token files are referenced by path,
  read at spawn, never logged, never echoed, never committed. Existence
  checks only in diagnostics.
- **I7 — locality is proven, never assumed.** Bulk payloads cross the
  bus as **claim-check references** — (filesystem scope, scope-relative
  path, byte range) — not as inlined bytes. A reference is dereferenced
  locally only under a **proven** scope match (the rendezvous beacon —
  plan D19); otherwise the bus serves the bytes itself. Every agent can
  learn where it runs (node identity: machine id, hostname, addresses)
  and where its filesystem comes from (scope identity: beacon + mount
  metadata — for network mounts that includes the server identity and
  export, the NFS/NAS case). Paths inside references are always
  scope-relative: the same NAS mounts at different points on different
  nodes. (Owner directive, 2026-07-09.)

## 5. Usage & terms-of-service posture {#tos}

What fractality does: the boss remains **one interactive session** on the
owner's consumer subscription — normal use. Swarm load goes to the provider
the swarm's owner pays for (z.ai plans for GLM), through that provider's
documented Claude Code integration surface (the standard
`ANTHROPIC_BASE_URL` / auth-token environment override, the same paved road
enterprise gateways use).

What fractality refuses to be: it does **not** multiplex a consumer
subscription across headless fleets, does not evade rate limits, does not
share accounts, and does not proxy one provider's traffic through another's
credentials. I1 is the enforcement: a worker cannot even *see* the boss's
credentials. Tariff hygiene is part of the same posture — e.g. workers get
web tools denied and documents are fetched locally once, because the GLM
plan's MCP-call quota is a metered resource (Campaign 1, Phase 5 playbooks).

## 6. Relationship to the host repository {#host}

fractality is incubated as a vibevm **workspace** (host `WORKSPACES.md`;
`flow:org.vibevm/wal-workspaces`) and packaged in the vibevm format
(`tool:org.vibevm.fractality/fractality`), but it is a standalone product:
no dependency on vibevm at build time or runtime, its own WAL and plans, its
own floor. Publishing to any registry is owner-word-only. When the product
outgrows the incubator, the workspace graduates to its own repository per
the workspaces protocol.

## 7. Evolution horizons (designed-for, not built) {#horizons}

Named so nobody mistakes their absence for oversight: async-rich lifecycle
(SSE events, await-any/all), an MCP surface for the boss, the
mission-control package split and API stabilization, GUI/analytics over the
journal, multi-machine federation, further backends (Codex; **VibeVM
Pixel** — the owner's planned Opus-native agent), the initiative system
(Campaign 2), the RLM protocol (Campaign 3), an execution-checkpointing
layer in the spirit of Entire.io's Checkpoints — per-turn workspace +
history snapshots with rewind and audit over runs; the owner tracks an
existing system, deliberately unadopted while it is young (plan DEF-12) —
and vibe-native distribution (`vibe bin exec` dispatch). Each enters
through a campaign with its own plan; none is licensed to complicate
v0.1.
