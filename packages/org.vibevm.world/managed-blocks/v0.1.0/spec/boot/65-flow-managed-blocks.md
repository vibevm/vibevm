# Flow: Managed Blocks {#root}

This project ships the **managed blocks** practice — a discipline
for tool authors: how a tool writes into a file it does not own (an
agent-instruction file, a shell rc, an ssh config, a shared project
config) without destroying what the other tenants wrote. The law
fits on one line:

```
Own exactly one delimited block; never touch a byte outside it.
```

## When to read the protocol {#when}

**Before** designing or reviewing any feature that writes into a
file the tool does not fully own, read
[`MANAGED-BLOCKS-PROTOCOL.md`](../flows/managed-blocks/MANAGED-BLOCKS-PROTOCOL.md)
first: marker design, the absent / present / malformed state
machine, the three verbs (create / update / remove), plan-time
classification. The shortcuts you are tempted by — a sidecar file, a
smart detector, auto-repair, "just regenerate the file" — are
already catalogued with their failure modes in
[`rejected-designs.md`](../flows/managed-blocks/rejected-designs.md).
Migrating an existing overwriting tool, the fixture table for the
state machine, and what belongs inside the block:
[`adoption-guide.md`](../flows/managed-blocks/adoption-guide.md).

## Never {#never}

- Never write outside your own block. Every byte beyond your markers
  is another tenant's property.
- Never gate a destructive write on a nondeterministic detector —
  the block is found by a deterministic byte scan or not at all.
- Never auto-repair a malformed block. Hard stop, precise report;
  the human decides.
- Never rewrite a file when the result is byte-identical.
