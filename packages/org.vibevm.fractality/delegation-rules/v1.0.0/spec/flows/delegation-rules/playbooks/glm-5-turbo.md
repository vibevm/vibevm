# Playbook — GLM-5-Turbo (the small slot) {#root}

_Field-calibrated on the IGNITION campaign (2026-07-10): swarm module
writes ×3 and a five-test unit suite, all green first landing._

## Identity

- Routing: `model = "small"` on a GLM profile; model id `glm-5-turbo`
  (also the profile's `haiku_slot` — CC-internal small-model traffic).
- Class: Haiku-grade mechanical worker. Everything you would hand a
  fast small model: bounded, templated, spec-longer-than-thinking.

## Task shapes it wins

- Small file writes and edits from an exact spec (one module, one
  fixture, one config).
- Well-templated unit-test suites where the API and the test names are
  given verbatim (scenario 1 only — never expect it to discover
  context).
- Format conversions, fixture generation, mechanical renames within a
  stated file list.

## Budget defaults

```toml
[budget]
wall_secs = 600
max_turns = 15
max_output_tokens = 60000
```

## Tariff rules (D12)

Same profile mechanics as the big slot: web denied, flat-rate plan,
admission bounded by `max_concurrent`. Turbo turns are the cheapest in
the fleet — swarm-parallelize freely (a 3-worker swarm's wall clock
equals its slowest member, P4-proven).

## Known blind spots (paid-for)

- **Zero context discovery**: it reads what the work order names and
  nothing else; a scenario-2 boot order wastes its turns — if the
  context is not compilable, route to the big slot.
- **Instruction drift on long orders**: keep the work order under a
  screen; number the steps; one deliverable per packet.

## Escalation

One failed landing → re-route the same packet to `model = "big"`
(GLM-5.2). A second failure there → the boss reclaims (matrix, bounded
retries).
