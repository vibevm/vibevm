# Landscape note — where fractality sits {#root}

_Ф0.s7 shallow scan, 2026-07-09. Positioning only — none of these is
studied for implementation (inventory S7). The point is to state what
fractality does that the field does not._

## The two families that exist

**Per-request routers/proxies** — e.g. `claude-code-router`
(musistudio): a local gateway (`localhost:8080`) that rewrites each API
call from one Claude Code process to whatever provider you configure,
by conditional rules. One process, many providers *per request*.
Verified from its README this session.

**Session/instance orchestrators** — e.g. `claude-swarm` (parruda,
YAML-defined agent teams over MCP), `claude-squad` (TUI managing
several Claude/Codex sessions in tmux+worktrees). Many processes, one
provider each, coordinated by config or a terminal multiplexer.

## Why fractality is neither

| Axis | routers | orchestrators | **fractality** |
|---|---|---|---|
| provider switch | per request, in-proc | per session, manual | per **worker profile**, isolated env (I1) |
| supervision | none (it's a proxy) | tmux / MCP wiring | **mission-control daemon**: registry, call tree, budgets, recursive kill |
| stdio / crashes | n/a | terminal-bound | **pod** per worker owns stdio, survives MC restart (D3) |
| boss↔worker channel | HTTP proxy | shared terminal | **MC bus**; files are persistence, not medium (I2) |
| non-yolo autonomy | n/a | usually skip-perms | allowlist + **pod broker + ask_boss** (D18) |
| metering | none | none | **journal is the single telemetry store** (I3) |
| bulk data | n/a | files ad hoc | **claim-check FileRefs**, S3-range semantics, proven fs-identity (D19) |
| distribution | single box | single box | designed for **federation** (pods = machine-local runners) |

The router answers "call a cheaper model." The orchestrators answer
"run several sessions at once." fractality answers a different
question: **"be the scheduler of an agent operating system"** — the
process table, call stack, quotas, kill, and accounting that a router
and a tmux grid both lack. The closest prior art in spirit is not in
this list at all; it is the young Linux kernel + GNU userland, which is
the analogy the foundation (PROP-001) builds on deliberately.

## What we take as validation, not code

The field proves the *demand* (cheap-model routing and multi-agent runs
are wanted) and the *paved road* (the `ANTHROPIC_BASE_URL` env override
is how everyone connects GLM to Claude Code — confirmed by z.ai's own
docs this session). fractality's bet is that the missing layer is
supervision + accounting + a channel discipline, built as one
cross-platform Rust product rather than a proxy or a shell of scripts.
