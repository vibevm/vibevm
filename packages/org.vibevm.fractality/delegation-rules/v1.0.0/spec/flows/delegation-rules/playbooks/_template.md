# Playbook — <model name> (<slot>) {#root}

_The extension surface (matrix §routing): one card per worker model,
this schema, field-calibrated before it routes real work. Copy, fill,
and cite live runs — a playbook without field data is a hypothesis and
must say so._

## Identity

- Routing: `model = "<big|small>"` on profile `<name>`; model id
  `<exact provider id>`.
- Class: one line — what tier of work this model is for.

## Task shapes it wins

- List the shapes with evidence (run ids or "hypothesis").

## Budget defaults

```toml
[budget]
wall_secs = <n>
max_turns = <n>
max_output_tokens = <n>
```

## Tariff rules

- What the provider meters (calls, tokens, tool invocations) and what
  the profile must therefore deny or cap (D12 mechanisms, not prose).

## Known blind spots (paid-for)

- Only observed failures belong here, each with what it cost and the
  countermeasure baked into future work orders.

## Escalation

- Where a failed landing goes next (bigger slot / boss reclaim), per
  the matrix's bounded-retry rule.
