# fractality

An agent operating system in its earliest form.

One expensive, high-judgment agent (the **boss**) should not spend its
context and its owner's budget on work a cheap model does well. fractality
gives the boss a delegation fabric: task packets go in, isolated worker
agents run them — Claude Code processes under *other* model providers (first:
GLM 5.2 / GLM-5-Turbo via z.ai), in their own environments, worktrees, and
config dirs — and everything comes back as **files on disk**: a result
document, a branch, a transcript, a usage record.

At the center sits **`fractality-mission-control`** — a small Rust daemon
that is the scheduler of this operating system: it spawns and supervises
workers, holds the run registry and the call tree (agents spawning agents —
hence the name), enforces budgets, kills whole process trees on demand, and
meters everything. Its journal is the single telemetry store: scoreboards,
delegation analytics, future GUIs and meta-cognition tooling all read from
it. The long horizon — dedicated servers federating agents across machines —
is deliberately designed-for but not built yet; think of the years when the
Linux kernel and the GNU userland were young.

Design commitments (full text: [`spec/PROP-001-foundation.md`](spec/PROP-001-foundation.md)):

- **Process-level provider isolation**, not proxying: a worker is a separate
  Claude Code process whose environment is constructed from scratch — it
  never inherits the boss's credentials. Enforced by tests.
- **Files are the only content channel** between boss and workers.
- **Cross-platform Rust**, one binary per role, no interpreter zoos.
- **Agent-neutral core:** Claude Code is the first worker backend and the
  first boss harness, not a hard dependency of the model.
- **Clean-room** with respect to every studied reference
  ([`spec/refs/INVENTORY.md`](spec/refs/INVENTORY.md)).
- **Fair use:** one interactive boss session on a consumer subscription;
  swarm load goes to the provider the swarm's owner pays for. No
  subscription multiplexing, no rate evasion.

Status: **pre-alpha, pre-code.** The IGNITION campaign plan
([`spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md`](spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md))
is the authoritative roadmap: spikes → mission-control core → delegate-out →
collect-back → swarm → delegation policy → boss integration. Campaign 2
(initiative system) and Campaign 3 (RLM protocol) follow it.

License: UPL-1.0.
