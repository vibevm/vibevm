# Flow: Managed Blocks {#root}

<status stage="impl" state="done"/>

##PROJECT-SHIPS-THE-MANAGED-BLOCKS-PRACTICE This project ships the **managed blocks** practice — a discipline
for tool authors: how a tool writes into a file it does not own (an
agent-instruction file, a shell rc, an ssh config, a shared project
config) without destroying what the other tenants wrote. @impl/done

##the-law-fits-on-one-line The law
fits on one line: @impl/done

```
Own exactly one delimited block; never touch a byte outside it.
```

## When to read the protocol {#when}

##READ-THE-PROTOCOL-BEFORE-WRITING-INTO-A-SHARED-FILE **Before** designing or reviewing any feature that writes into a
file the tool does not fully own, read
[`MANAGED-BLOCKS-PROTOCOL.md`](../flows/managed-blocks/MANAGED-BLOCKS-PROTOCOL.md)
first: marker design, the absent / present / malformed state
machine, the three verbs (create / update / remove), plan-time
classification. @impl/done

##shortcuts-are-already-catalogued The shortcuts you are tempted by — a sidecar file, a
smart detector, auto-repair, "just regenerate the file" — are
already catalogued with their failure modes in
[`rejected-designs.md`](../flows/managed-blocks/rejected-designs.md). @impl/done

##adoption-guide-pointer Migrating an existing overwriting tool, the fixture table for the
state machine, and what belongs inside the block:
[`adoption-guide.md`](../flows/managed-blocks/adoption-guide.md). @impl/done

## Never {#never}

- ##NEVER-WRITE-OUTSIDE-YOUR-OWN-BLOCK Never write outside your own block. Every byte beyond your markers
  is another tenant's property. @impl/done
- ##NEVER-GATE-A-DESTRUCTIVE-WRITE-ON-A-NONDETERMINISTIC-DETECTOR Never gate a destructive write on a nondeterministic detector —
  the block is found by a deterministic byte scan or not at all. @impl/done
- ##NEVER-AUTO-REPAIR-A-MALFORMED-BLOCK Never auto-repair a malformed block. Hard stop, precise report;
  the human decides. @impl/done
- ##NEVER-REWRITE-A-FILE-WHEN-THE-RESULT-IS-BYTE-IDENTICAL Never rewrite a file when the result is byte-identical. @impl/done
