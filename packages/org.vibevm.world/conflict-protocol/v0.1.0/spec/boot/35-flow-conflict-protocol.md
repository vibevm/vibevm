# Flow: Conflict Protocol {#root}

<status stage="impl" state="done"/>

##PROJECT-RUNS-TWO-WRITERS-OVER-ONE-FILE-SET This project runs **two writers over one file set** — a human and a
coding agent both edit the spec tree, the tests, and the code. @impl/done

##CONTRADICTIONS-ARE-NORMAL-COOPERATION-NOT-AN-ERROR They
*will* write contradictory things; that is normal cooperation, not an
error. @spec/done

##WHAT-IS-FORBIDDEN-IS-RESOLVING-A-CONTRADICTION-SILENTLY What is forbidden is resolving a contradiction silently. @impl/done

## The hierarchy {#hierarchy}

##EVERY-DISAGREEMENT-IS-SETTLED-BY-FIXED-PRIORITY Every disagreement between layers is settled by fixed priority: @impl/done

```
Human  >  Spec  >  Tests  >  Code  >  WAL
```

- ##HUMAN-MAY-CHANGE-THE-SPEC-AND-NOBODY-ELSE-MAY-SILENTLY The human may change the spec; nobody else may — silently. @impl/done
- ##CODE-MUST-CONFORM-TO-THE-SPEC Code must conform to the spec, never the other way around. @impl/done
- ##TESTS-ARE-THE-SPEC-IN-EXECUTABLE-FORM Tests are the spec in executable form: a test that contradicts the
  spec is a bug in exactly one of the two, never both. @impl/done
- ##THE-VOLATILE-STATE-FILE-IS-A-RECORD-DEAD-LAST The volatile state file (WAL or equivalent) is a record, dead last:
  when it disagrees with anything above it, it is stale. @impl/done

##full-protocol-pointer Full protocol:
[`spec/flows/conflict-protocol/CONFLICT-PROTOCOL.md`](../flows/conflict-protocol/CONFLICT-PROTOCOL.md). @impl/done

## The REVIEW contract {#review}

##IF-YOU-BELIEVE-THE-SPEC-IS-WRONG-IMPLEMENT-IT-ANYWAY If you believe the spec is wrong: **implement the spec anyway**, add
a marker at the point of disagreement — @impl/done

```
<!-- REVIEW: <what you would change> because <reason> -->
```

##SURFACE-THE-MARKER-IN-THE-END-OF-SESSION-REPORT — and surface it in the end-of-session report. @impl/done

##THE-HUMAN-DECIDES-IN-THE-NEXT-CYCLE The human decides in
the next cycle. @impl/done

##NEVER-SILENTLY-OVERRIDE Never silently override. @impl/done

##three-lines-seconds-to-write-a-minute-to-read Three lines of text; seconds
to write; a minute to read. @spec/done

## When the spec is silent {#uncertainty}

##CLIMB-THE-LADDER-BEFORE-CHOOSING-A-CONSERVATIVE-DEFAULT Re-read the relevant spec section → re-read the relevant reference
chapter → check the closest analog in the project → if still unclear,
pick the conservative interpretation (the one cheapest to reverse),
mark it with a REVIEW, proceed, and flag it in the report. @impl/done

##NEVER-SILENTLY-INVENT-SEMANTIC-BEHAVIOR Never
silently invent semantic behavior. @impl/done

##full-ladder-pointer Full ladder:
[`spec/flows/conflict-protocol/uncertainty-protocol.md`](../flows/conflict-protocol/uncertainty-protocol.md). @impl/done

## Never {#never}

- ##NEVER-SILENTLY-MODIFY-A-NORMATIVE-SPEC-VALUE Never silently modify a normative spec value. @impl/done
- ##NEVER-RESOLVE-A-DISAGREEMENT-BY-ASSUMING-THE-CODE-IS-NEWER Never resolve a spec-vs-code disagreement by assuming the code is
  newer. Recency is not authority; the hierarchy is. @impl/done
- ##NEVER-REMOVE-SOMEONE-ELSES-REVIEW-MARKER Never remove someone else's REVIEW marker without resolving it. @impl/done
- ##NEVER-INVENT-SEMANTICS-WHEN-THE-SPEC-IS-SILENT Never invent semantics when the spec is silent — mark the choice
  and proceed conservatively. @impl/done

##recovery-drills-pointer Recovery drills for when the protocol has already been broken:
[`spec/flows/conflict-protocol/failure-modes.md`](../flows/conflict-protocol/failure-modes.md). @impl/done
