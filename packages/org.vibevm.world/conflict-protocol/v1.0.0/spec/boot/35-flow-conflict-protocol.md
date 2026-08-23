# Flow: Conflict Protocol {#root}

<status stage="impl" state="done"/>

@fact:PROJECT-RUNS-TWO-WRITERS-OVER-ONE-FILE-SET This project runs **two writers over one file set** — a human and a
coding agent both edit the spec tree, the tests, and the code. @status:impl/done

@fact:CONTRADICTIONS-ARE-NORMAL-COOPERATION-NOT-AN-ERROR They
*will* write contradictory things; that is normal cooperation, not an
error. @status:spec/done

@fact:WHAT-IS-FORBIDDEN-IS-RESOLVING-A-CONTRADICTION-SILENTLY What is forbidden is resolving a contradiction silently. @status:impl/done

## The hierarchy {#hierarchy}

@fact:EVERY-DISAGREEMENT-IS-SETTLED-BY-FIXED-PRIORITY Every disagreement between layers is settled by fixed priority: @status:impl/done

```
Human  >  Spec  >  Tests  >  Code  >  volatile state
```

- @fact:HUMAN-MAY-CHANGE-THE-SPEC-AND-NOBODY-ELSE-MAY-SILENTLY The human may change the spec; nobody else may — silently. @status:impl/done
- @fact:CODE-MUST-CONFORM-TO-THE-SPEC Code must conform to the spec, never the other way around. @status:impl/done
- @fact:TESTS-ARE-THE-SPEC-IN-EXECUTABLE-FORM Tests are the spec in executable form: a test that contradicts the
  spec is a bug in exactly one of the two, never both. @status:impl/done
- @fact:THE-VOLATILE-STATE-FILE-IS-A-RECORD-DEAD-LAST The volatile state file is a record, dead last:
  when it disagrees with anything above it, it is stale. @status:impl/done

@fact:full-protocol-pointer Full protocol:
@spec://org.vibevm.world/conflict-protocol/flows/conflict-protocol/CONFLICT-PROTOCOL#root. @status:impl/done

## The REVIEW contract {#review}

@fact:IF-YOU-BELIEVE-THE-SPEC-IS-WRONG-IMPLEMENT-IT-ANYWAY If you believe the spec is wrong: **implement the spec anyway**, add
a marker at the point of disagreement — @status:impl/done

```
<!-- REVIEW: <what you would change> because <reason> -->
```

@fact:SURFACE-THE-MARKER-IN-THE-END-OF-SESSION-REPORT — and surface it in the end-of-session report. @status:impl/done

@fact:THE-HUMAN-DECIDES-IN-THE-NEXT-CYCLE The human decides in
the next cycle. @status:impl/done

@fact:NEVER-SILENTLY-OVERRIDE Never silently override. @status:impl/done

@fact:three-lines-seconds-to-write-a-minute-to-read Three lines of text; seconds
to write; a minute to read. @status:spec/done

## When the spec is silent {#uncertainty}

@fact:CLIMB-THE-LADDER-BEFORE-CHOOSING-A-CONSERVATIVE-DEFAULT Re-read the relevant spec section → re-read the relevant reference
chapter → check the closest analog in the project → if still unclear,
pick the conservative interpretation (the one cheapest to reverse),
mark it with a REVIEW, proceed, and flag it in the report. @status:impl/done

@fact:NEVER-SILENTLY-INVENT-SEMANTIC-BEHAVIOR Never
silently invent semantic behavior. @status:impl/done

@fact:full-ladder-pointer Full ladder:
@spec://org.vibevm.world/conflict-protocol/flows/conflict-protocol/uncertainty-protocol#root. @status:impl/done

## Never {#never}

- @fact:NEVER-SILENTLY-MODIFY-A-NORMATIVE-SPEC-VALUE Never silently modify a normative spec value. @status:impl/done
- @fact:NEVER-RESOLVE-A-DISAGREEMENT-BY-ASSUMING-THE-CODE-IS-NEWER Never resolve a spec-vs-code disagreement by assuming the code is
  newer. Recency is not authority; the hierarchy is. @status:impl/done
- @fact:NEVER-REMOVE-SOMEONE-ELSES-REVIEW-MARKER Never remove someone else's REVIEW marker without resolving it. @status:impl/done
- @fact:NEVER-INVENT-SEMANTICS-WHEN-THE-SPEC-IS-SILENT Never invent semantics when the spec is silent — mark the choice
  and proceed conservatively. @status:impl/done

@fact:recovery-drills-pointer Recovery drills for when the protocol has already been broken:
@spec://org.vibevm.world/conflict-protocol/flows/conflict-protocol/failure-modes#root. @status:impl/done
