# Playbook — GLM-5.2 (the big slot) {#root}

_Field-calibrated on the IGNITION campaign (2026-07-09/10): six live
one-shot deliveries, five green on first landing._

## Identity

- Routing: `model = "big"` on a GLM profile; model id `glm-5.2[1m]`
  (the `[1m]` suffix selects the 1M-token context variant).
- Class: large flat-rate implementer. Strongest at **big,
  well-specified one-shot tasks** — the owner's founding observation,
  confirmed live.

## Task shapes it wins

- A whole module / test suite / parser with exact APIs, patterns, and
  a self-verify command compiled into the prompt (scenario 1).
- Grep-sweep + classify across many files with an explicit output
  format.
- First-draft summaries and fact-sheet extraction from large local
  documents (scenario 2: order it to read named files first).
- Boilerplate at scale where the template is stated once and applied
  many times.

## Budget defaults

```toml
[budget]
wall_secs = 1800
max_turns = 40
max_output_tokens = 200000
```

One-shot bias: prefer a bigger, complete work order over follow-up
turns. If a second follow-up is needed, the packet was underspecified
— reclaim and resplit rather than ping-pong (matrix, bounded
retries).

## Tariff rules (D12, mechanism-backed)

- Web tools are denied by profile; never write a work order that
  needs live web access — fetch documents locally first
  (`fractality fetch`, boss-side).
- Flat-rate plan: token spend is not the constraint; the 5-hour
  prompt budget and monthly MCP-call pool are. Keep `max_concurrent`
  within the profile's cap.

## Known blind spots (paid-for)

- **Version currency / MSRV**: assumes newer toolchains and crate
  versions than the tree pins — state pins explicitly; the boss
  checks version claims (F9, F13).
- **Poisoned cwd**: inherits the shell's directory and will happily
  roam a wrong tree for minutes — pin the cwd in the launch command
  itself.
- **Silent planning stalls**: long thinking with no output looks like
  a hang under end-buffered stdout — demand `PROGRESS:` heartbeat
  lines and a final `TASK-DONE` marker in every work order; watch
  file mtimes, not the pipe.
- **Constraint conflicts**: given contradictory constraints (e.g.
  "self-verify" vs "don't touch the manifest" on a broken tree) it
  bends one creatively rather than stopping — never hand it a work
  order whose self-verify transits files under live edit.

## Escalation

Failed landing (acceptance red or diff rejected) → the boss reclaims
the task. There is no bigger slot to escalate to.
