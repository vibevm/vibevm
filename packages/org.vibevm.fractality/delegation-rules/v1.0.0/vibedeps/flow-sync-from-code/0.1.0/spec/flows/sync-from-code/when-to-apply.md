# When to run Sync-from-Code {#root}

Sync-from-Code is an exceptional protocol, not a default. This document
is the decision table for *should I run it right now?*

## Run it when {#run}

### Direct code edit that sticks {#direct-edit}

You (the human) opened a source file and changed a value, a constant,
or a piece of logic that the spec pins. The edit is deliberate and
permanent — you plan to keep it.

*Example:* adjusted `TIMEOUT = 600` in `src/verify.rs`. Spec still
says 300 s.

### Imperative chat command {#imperative}

You told the agent "change the timeout to 600 s" or "switch the hash
function to blake3" without updating the spec first. The agent
executed and the code now reflects the new choice.

*Example:* session transcript shows the user asked the agent to
replace `SHA256::digest` calls with `blake3::hash`. PROP-000 still
specifies SHA-256.

### Experiment converged {#experiment}

You were trying two candidates; one won. The winning code is staying.
The spec still describes the experiment-in-progress state, or still
names the loser as the current answer.

*Example:* tried both fixed and adaptive timeout; fixed-600 s won
on measured data. Code is fixed-600 s. Spec still says "adaptive,
TBD".

## Do not run it when {#skip}

### The code change is temporary {#skip-temp}

Debug scaffolding, ad-hoc probes, a reproducer you plan to delete
within the day. Do not dignify it with a sync. Record the skip
explicitly in the WAL so the next session does not try:

```markdown
## Constraints
- src/verify.rs: temporary trace logging for #42 reproduction,
  do NOT sync to spec.
```

The WAL entry is what keeps the next session honest. Without it, a
sync-happy agent will try to promote the trace logging to a
first-class spec feature.

### The change is mechanical {#skip-mech}

`cargo fmt`, import reorder, dead-code removal the compiler already
flagged, rename of a private symbol that has no public contract.
The spec lives at a higher level of resolution than these changes.
Sync would produce noise, not a decision record.

### You cannot name the reason {#skip-no-reason}

If the honest answer to "why did the code change?" is "it felt
better" or "I don't remember", stop. Sync-from-Code is not for
laundering unreasoned drift into the spec. Two paths:

1. **Recover the reason.** Re-read the session, the measurements,
   the issue. If a durable reason exists, surface it and run the
   protocol normally.
2. **Revert the code.** If no reason can be named, the code change
   itself is suspect. Revert.

Do not produce a spec change that reads "we do X because we do X".
That is not a decision, it is a tautology with a date attached.

### The spec section does not exist yet {#skip-bootstrap}

The code implements something the spec does not mention at all. This
is the forward-flow case: draft a new PROP/FEAT section, write the
intent-first, then implement or reconcile. Sync-from-Code is for
**updating** existing spec entries, not for bootstrapping them.

Putting a brand-new spec section together via Sync-from-Code
produces "spec that matches the code that was written without a
spec" — a classic retrofit, and everyone can tell.

## Quick decision flow {#flowchart}

```
Did code change since last spec-aligned state?   ─ no ─→ done
   │ yes
   ▼
Is the change temporary?                         ─ yes ─→ record in WAL, done
   │ no
   ▼
Is the change purely mechanical?                 ─ yes ─→ done (no sync)
   │ no
   ▼
Does the relevant spec section already exist?    ─ no ─→ draft spec normally
   │ yes
   ▼
Can you name the reason in one sentence?         ─ no ─→ recover or revert
   │ yes
   ▼
Run Sync-from-Code (SYNC-PROTOCOL.md).
```

## Boundary with other flows {#boundaries}

- **`flow:wal`** handles session continuity. A successful sync may
  trigger a WAL update; that update goes through the WAL flow, not
  this one.
- **`flow:atomic-commits`** handles commit discipline. A sync commit
  follows Conventional Commits and carries `docs(spec)` as its type;
  that framing is defined by the atomic-commits flow, not here.
- **`vibe build`** (M1.5+) handles the other direction — generating
  code from spec. A sync can be followed by a build, but they are
  independent flows.
