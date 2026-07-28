# When to run Sync-from-Code {#root}

<status stage="spec" state="done"/>

##SYNC-FROM-CODE-IS-AN-EXCEPTIONAL-PROTOCOL-NOT-A-DEFAULT Sync-from-Code is an exceptional protocol, not a default. @impl/done

##this-document-is-the-decision-table This document
is the decision table for *should I run it right now?* @impl/done

## Run it when {#run}

### Direct code edit that sticks {#direct-edit}

##RUN-WHEN-A-DIRECT-EDIT-TOUCHES-WHAT-THE-SPEC-PINS You (the human) opened a source file and changed a value, a constant,
or a piece of logic that the spec pins. @impl/done

##THE-EDIT-IS-DELIBERATE-AND-PERMANENT The edit is deliberate and
permanent — you plan to keep it. @impl/done

##example-direct-edit *Example:* adjusted `TIMEOUT = 600` in `src/verify.rs`. Spec still
says 300 s. @impl/done

### Imperative chat command {#imperative}

##RUN-WHEN-AN-IMPERATIVE-CHAT-COMMAND-CHANGED-THE-CODE You told the agent "change the timeout to 600 s" or "switch the hash
function to blake3" without updating the spec first. @impl/done

##THE-AGENT-EXECUTED-AND-THE-CODE-REFLECTS-THE-NEW-CHOICE The agent
executed and the code now reflects the new choice. @impl/done

##example-imperative-chat-command *Example:* session transcript shows the user asked the agent to
replace `SHA256::digest` calls with `blake3::hash`. PROP-000 still
specifies SHA-256. @impl/done

### Experiment converged {#experiment}

##RUN-WHEN-AN-EXPERIMENT-CONVERGED You were trying two candidates; one won. @impl/done

##THE-WINNING-CODE-IS-STAYING The winning code is staying. @impl/done

##THE-SPEC-STILL-DESCRIBES-THE-EXPERIMENT-OR-NAMES-THE-LOSER The spec still describes the experiment-in-progress state, or still
names the loser as the current answer. @impl/done

##example-experiment-converged *Example:* tried both fixed and adaptive timeout; fixed-600 s won
on measured data. Code is fixed-600 s. Spec still says "adaptive,
TBD". @impl/done

## Do not run it when {#skip}

### The code change is temporary {#skip-temp}

##SKIP-DEBUG-SCAFFOLDING-PROBES-AND-SHORT-LIVED-REPRODUCERS Debug scaffolding, ad-hoc probes, a reproducer you plan to delete
within the day. @impl/done

##DO-NOT-DIGNIFY-A-TEMPORARY-CHANGE-WITH-A-SYNC Do not dignify it with a sync. @impl/done

##record-the-skip-in-the-wal-lead Record the skip
explicitly in the WAL so the next session does not try: @impl/done

```markdown
## Constraints
- src/verify.rs: temporary trace logging for #42 reproduction,
  do NOT sync to spec.
```

##THE-WAL-ENTRY-KEEPS-THE-NEXT-SESSION-HONEST The WAL entry is what keeps the next session honest. @impl/done

##without-it-a-sync-happy-agent-promotes-the-hack Without it, a
sync-happy agent will try to promote the trace logging to a
first-class spec feature. @spec/done

### The change is mechanical {#skip-mech}

##SKIP-FORMATTING-IMPORT-ORDER-DEAD-CODE-AND-PRIVATE-RENAMES `cargo fmt`, import reorder, dead-code removal the compiler already
flagged, rename of a private symbol that has no public contract. @impl/done

##THE-SPEC-LIVES-AT-A-HIGHER-LEVEL-OF-RESOLUTION The spec lives at a higher level of resolution than these changes. @impl/done

##SYNC-WOULD-PRODUCE-NOISE-NOT-A-DECISION-RECORD Sync would produce noise, not a decision record. @impl/done

### You cannot name the reason {#skip-no-reason}

##IF-THE-HONEST-ANSWER-IS-IT-FELT-BETTER-STOP If the honest answer to "why did the code change?" is "it felt
better" or "I don't remember", stop. @impl/done

##SYNC-IS-NOT-FOR-LAUNDERING-UNREASONED-DRIFT Sync-from-Code is not for
laundering unreasoned drift into the spec. Two paths: @impl/done

1. ##PATH-RECOVER-THE-REASON **Recover the reason.** Re-read the session, the measurements,
   the issue. If a durable reason exists, surface it and run the
   protocol normally. @impl/done
2. ##PATH-REVERT-THE-CODE **Revert the code.** If no reason can be named, the code change
   itself is suspect. Revert. @impl/done

##DO-NOT-PRODUCE-A-SPEC-CHANGE-THAT-READS-WE-DO-X-BECAUSE-WE-DO-X Do not produce a spec change that reads "we do X because we do X". @impl/done

##a-tautology-with-a-date-attached-is-not-a-decision That is not a decision, it is a tautology with a date attached. @impl/done

### The spec section does not exist yet {#skip-bootstrap}

##SKIP-WHEN-THE-CODE-IMPLEMENTS-SOMETHING-THE-SPEC-NEVER-MENTIONS The code implements something the spec does not mention at all. @impl/done

##this-is-the-forward-flow-case-lead This
is the forward-flow case: @impl/done

- ##FORWARD-FLOW-DRAFT-A-NEW-SPEC-SECTION draft a new PROP/FEAT section, @impl/done
- ##FORWARD-FLOW-WRITE-THE-INTENT-FIRST write the
  intent-first, @impl/done
- ##FORWARD-FLOW-THEN-IMPLEMENT-OR-RECONCILE then implement or reconcile. @impl/done

##SYNC-UPDATES-EXISTING-ENTRIES-IT-DOES-NOT-BOOTSTRAP-THEM Sync-from-Code is for
**updating** existing spec entries, not for bootstrapping them. @impl/done

##a-brand-new-section-via-sync-is-a-classic-retrofit Putting a brand-new spec section together via Sync-from-Code
produces "spec that matches the code that was written without a
spec" — a classic retrofit, and everyone can tell. @spec/done

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

- ##BOUNDARY-FLOW-WAL **`flow:wal`** handles session continuity. A successful sync may
  trigger a WAL update; that update goes through the WAL flow, not
  this one. @impl/done
- ##BOUNDARY-FLOW-ATOMIC-COMMITS **`flow:git-atomic-commits`** handles commit discipline. A sync commit
  follows Conventional Commits and carries `docs(spec)` as its type;
  that framing is defined by the git-atomic-commits flow, not here. @impl/done
- ##BOUNDARY-VIBE-BUILD **`vibe build`** (M1.5+) handles the other direction — generating
  code from spec. A sync can be followed by a build, but they are
  independent flows. @spec/done
