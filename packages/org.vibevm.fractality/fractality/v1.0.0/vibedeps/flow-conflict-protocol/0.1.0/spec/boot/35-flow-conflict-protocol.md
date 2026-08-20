# Flow: Conflict Protocol {#root}

This project runs **two writers over one file set** — a human and a
coding agent both edit the spec tree, the tests, and the code. They
*will* write contradictory things; that is normal cooperation, not an
error. What is forbidden is resolving a contradiction silently.

## The hierarchy {#hierarchy}

Every disagreement between layers is settled by fixed priority:

```
Human  >  Spec  >  Tests  >  Code  >  WAL
```

- The human may change the spec; nobody else may — silently.
- Code must conform to the spec, never the other way around.
- Tests are the spec in executable form: a test that contradicts the
  spec is a bug in exactly one of the two, never both.
- The volatile state file (WAL or equivalent) is a record, dead last:
  when it disagrees with anything above it, it is stale.

Full protocol:
[`spec/flows/conflict-protocol/CONFLICT-PROTOCOL.md`](../flows/conflict-protocol/CONFLICT-PROTOCOL.md).

## The REVIEW contract {#review}

If you believe the spec is wrong: **implement the spec anyway**, add
a marker at the point of disagreement —

```
<!-- REVIEW: <what you would change> because <reason> -->
```

— and surface it in the end-of-session report. The human decides in
the next cycle. Never silently override. Three lines of text; seconds
to write; a minute to read.

## When the spec is silent {#uncertainty}

Re-read the relevant spec section → re-read the relevant reference
chapter → check the closest analog in the project → if still unclear,
pick the conservative interpretation (the one cheapest to reverse),
mark it with a REVIEW, proceed, and flag it in the report. Never
silently invent semantic behavior. Full ladder:
[`spec/flows/conflict-protocol/uncertainty-protocol.md`](../flows/conflict-protocol/uncertainty-protocol.md).

## Never {#never}

- Never silently modify a normative spec value.
- Never resolve a spec-vs-code disagreement by assuming the code is
  newer. Recency is not authority; the hierarchy is.
- Never remove someone else's REVIEW marker without resolving it.
- Never invent semantics when the spec is silent — mark the choice
  and proceed conservatively.

Recovery drills for when the protocol has already been broken:
[`spec/flows/conflict-protocol/failure-modes.md`](../flows/conflict-protocol/failure-modes.md).
