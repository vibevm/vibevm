# Conflict Protocol {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** This file defines *how* two writers — a
human and a coding agent — share one file set without corrupting it:
the priority hierarchy that settles any disagreement between layers,
the REVIEW-marker protocol for disputing a spec without overriding
it, the marker lifecycle from placement to removal, and a worked
example of what one silent change costs when the protocol is skipped. @impl/done

## Two writers, one file set {#two-writers}

##A-PROJECT-HAS-EXACTLY-ONE-SHARED-MEMORY-THE-FILES A spec-driven project has exactly one shared memory: the files. @spec/done

##the-human-edits-them The
human edits them. @spec/done

##the-agent-edits-them The agent edits them. @spec/done

##THERE-IS-NO-THIRD-CHANNEL There is no third channel —
no hallway conversation, no "as we discussed yesterday". @spec/done

##TWO-WRITERS-WILL-PRODUCE-CONTRADICTIONS Two writers
over one file set *will* produce contradictions: @spec/done

- ##CONTRADICTION-A-VALUE-CHANGED-IN-CODE-NOT-IN-THE-SPEC a value changed in
  code but not in the spec, @spec/done
- ##CONTRADICTION-A-TEST-ASSERTING-YESTERDAYS-RULE a test asserting yesterday's rule, @spec/done
- ##CONTRADICTION-A-STATE-FILE-DESCRIBING-REVERTED-WORK a state
  file describing work that was later reverted. @spec/done

##CONTRADICTION-IS-NORMAL-COOPERATION-NOT-AN-ERROR This is normal cooperation, not an error. @impl/done

##treating-contradiction-as-shameful-hides-it Treating each contradiction
as a failure to be ashamed of is counterproductive — it trains both
writers to hide disagreement, and hidden disagreement is the only
kind that does damage. @spec/done

##THE-FAILURE-MODE-IS-THE-UNRESOLVED-CONFLICT The failure mode is the *unresolved* conflict,
found weeks later by a reader who cannot tell which side was right. @spec/done

##concurrent-programming-met-this-problem-long-ago Concurrent programming met this problem long ago: two processes
writing shared memory without an ordering rule produce a race
condition, and no post-hoc reading can reconstruct the intent. @spec/done

##the-fix-there-is-a-memory-fence The
fix there is a memory fence — an explicit ordering both processes
obey. @spec/done

##THE-FIX-HERE-IS-THE-SAME-FENCE-ADAPTED-FOR-PROSE The fix here is the same fence, adapted for prose. @impl/done

## The hierarchy {#hierarchy}

```
Human  >  Spec  >  Tests  >  Code  >  WAL
```

| Relation | Operational meaning |
|----------|---------------------|
| ##ROW-HUMAN-BEATS-SPEC **Human > Spec** @impl/done | The human may change the spec — that is what winning means here. A human instruction that contradicts the spec is not a conflict; it is a spec change that has not been written down yet. Write it down, then follow it. @impl/done |
| ##ROW-SPEC-BEATS-CODE **Spec > Code** @impl/done | Code must conform to the spec. When they disagree, the code is wrong *by definition* until the human rules otherwise. Fix the code, or dispute the spec with a REVIEW marker — never by editing the spec. @impl/done |
| ##ROW-TESTS-ARE-THE-SPEC-EXECUTABLE **Tests = Spec, executable** @impl/done | Tests are the spec in executable form (next section). @impl/done |
| ##ROW-CODE-BEATS-WAL **Code > WAL** @impl/done | The volatile state file (WAL or equivalent) is a record of where work stands, not a source of truth about intended behaviour. When it disagrees with anything above it, it is stale — correct the record, not the truth. @impl/done |

##THE-ORDER-IS-TOTAL-AND-ACYCLIC Unlike rock-paper-scissors — or rock-paper-scissors-lizard-Spock —
this order is total and acyclic. @impl/done

##predictability-is-the-entire-point Every pairing has a predetermined
winner, and that predictability is the entire point: two writers who
both know the resolution table never negotiate mid-write, and a
reader who finds a contradiction knows instantly which side to trust. @spec/done

##newer-appears-nowhere-in-the-table Note what appears nowhere in the table: "newer". @impl/done

##RECENCY-IS-EVIDENCE-OF-WHEN-NOT-OF-AUTHORITY Recency is evidence
about *when* a change happened, not whether it was authorized. @impl/done

##THE-MOST-DAMAGING-MOVE-IS-SYNCING-THE-SPEC-TO-NEWER-CODE The
most damaging move available to an agent is resolving a spec-vs-code
disagreement by assuming the code is newer and "syncing" the spec to
match — see the worked example below. @spec/done

## Tests are the spec in executable form {#tests}

##TESTS-SIT-BESIDE-THE-SPEC-NOT-BELOW-IT Tests sit beside the spec, not below it: the same rules, phrased in a
language a machine can run. @impl/done

##the-corollary-is-a-sharp-diagnostic-rule The corollary is a sharp diagnostic rule: @impl/done

> ##A-TEST-CONTRADICTING-THE-SPEC-IS-A-BUG-IN-EXACTLY-ONE A test that contradicts the spec is a bug in exactly one of the two
> — never both. @impl/done

##EITHER-THE-TEST-IS-STALE-OR-THE-SPEC-PROSE-HAS-DECAYED Either the test encodes yesterday's spec (fix the test), or the test
is right and the spec prose has decayed (dispute it with a REVIEW
marker; the human rules). @impl/done

##THE-FORBIDDEN-THIRD-PATH-IS-WEAKENING-THE-TEST The forbidden third path is weakening the
test until the code passes without consulting the spec at all — that
deletes the project's only automatic tripwire for divergence. @impl/done

## The REVIEW-marker protocol {#review-protocol}

##the-agent-will-sometimes-believe-the-spec-is-wrong The agent will sometimes believe the spec is wrong — and sometimes it
will *be* wrong. @spec/done

##the-protocol-when-that-happens The protocol when that happens: @impl/done

1. ##REVIEW-STEP-IMPLEMENT-WHAT-THE-SPEC-SAYS **Implement what the spec says.** The spec wins the current cycle,
   including when you disagree with it. @impl/done
2. ##REVIEW-STEP-ADD-A-MARKER-WITH-A-REASON **Add a REVIEW marker at the point of disagreement**, always with
   a reason: @impl/done

   ```
   <!-- REVIEW: §5.3 would be better served by exponential backoff
        than a fixed 600s timeout, because retries cluster at the
        boundary under VPN latency -->
   ```

3. ##REVIEW-STEP-SURFACE-IT-IN-THE-END-OF-SESSION-REPORT **Surface it in the end-of-session report:** "Implemented the
   fixed timeout per §5.3; propose exponential backoff instead — see
   REVIEW in PROP-001 §5.3." @impl/done
4. ##REVIEW-STEP-THE-HUMAN-DECIDES-IN-THE-NEXT-CYCLE **The human decides in the next cycle** — accepts (and changes the
   spec) or rejects (and the marker comes out). @impl/done

##three-lines-of-text Three lines of text. @spec/done

##seconds-to-write-a-minute-to-read Seconds to write, a minute to read. @spec/done

##that-is-the-entire-price-of-no-silent-override That is the
entire price of never having a silent override in the repository. @spec/done

### Where markers go {#placement}

- ##MARKER-GOES-IN-THE-SPEC **In the spec**, next to the contested clause, when the dispute is
  about what the rule should *be*. @impl/done
- ##MARKER-GOES-IN-THE-CODE **In the code**, next to the implementing lines, when the dispute
  is about how the rule lands in practice — in the language's own
  comment syntax (`// REVIEW: …`); the HTML form is for Markdown. @impl/done
- ##MARKER-ALWAYS-CARRIES-A-REASON **Always with a reason.** A bare `REVIEW: check this` cannot be
  ruled on; markers without reasons are noise wearing a uniform. @impl/done

### The bureaucracy objection {#bureaucracy}

##on-first-contact-it-reads-as-bureaucracy On first contact this protocol reads as bureaucracy. @spec/done

##the-protocol-is-the-opposite-on-all-three-axes But real
bureaucracy's sin is being slow, expensive, and useless; this is the
opposite on all three axes — seconds to execute, cheaper in tokens
than one reasoning session over an undocumented disagreement, and it
turns every dispute into a one-line decision the human makes at a
glance. @spec/done

##CHEAP-PROCESS-THAT-PREVENTS-EXPENSIVE-ARCHAEOLOGY-IS-A-FENCE Cheap process that prevents expensive archaeology is a fence. @spec/done

## What one silent change costs {#silent-change}

##a-worked-example-needing-no-malice A worked example. No step in it requires malice or incompetence —
only the absence of the protocol. @spec/done

```
Day 0   The agent changes TIMEOUT from 600s to 300s while touching
        nearby code ("300 is more responsive"). No marker, no report
        line. The human misses it: the diff is long, the change is
        two characters.

Day 7   A fresh session finds code = 300s, spec = 600s. No record
        says which is right. It reasons "the code is newer, the spec
        must be stale" and rewrites the spec to 300s. The forbidden
        move — resolving by recency — now looks like diligence.

Day 14  The human remembers *why* 600s existed (300s produced false
        timeouts for users on slow links), opens the spec, finds
        300s, changes it back — but by now two other code paths
        assume 300s.

Result  One unauthorized edit became three bugs, plus a two-week git
        archaeology dig to establish who changed what, when, and
        which value was ever actually intended.
```

##THIS-IS-A-DATA-RACE-AT-FILE-LEVEL This is a data race at file level: two writers, no fence, an outcome
no one chose. @spec/done

##WITH-THE-PROTOCOL-THE-CHAIN-NEVER-STARTS Run the same Day 0 *with* the protocol and the chain
never starts — the agent implements 600s, places one REVIEW marker
proposing 300s, and the human rules on it next morning in a minute. @impl/done

## Marker lifecycle {#lifecycle}

##A-MARKER-IS-A-MESSAGE-IN-FLIGHT-NOT-A-PERMANENT-ANNOTATION A REVIEW marker is a message in flight, not a permanent annotation. @impl/done

1. ##LIFECYCLE-PLACED **Placed** — during work, with a reason, at the point of
   disagreement. @impl/done
2. ##LIFECYCLE-SURFACED **Surfaced** — named in the end-of-session report with its file
   and section, so the human never has to grep for surprises. @impl/done
3. ##LIFECYCLE-RESOLVED **Resolved** — the human accepts (the spec changes) or rejects
   (the proposal dies; record one line of *why* if it will recur). @impl/done
4. ##LIFECYCLE-REMOVED-IN-THE-RESOLVING-CHANGE **Removed in the same change that resolves it.** A resolved marker
   left in place is indistinguishable from an open one, and the next
   reader will re-litigate it. @impl/done

##AGING-MARKERS-ARE-AUDIT-INPUT Aging markers are audit input. @impl/done

##A-LONG-LIVED-MARKER-MEANS-SURFACING-FAILED A marker that survives more than a few
sessions means step 2 failed — the surfacing channel is broken or
reports go unread. @impl/done

##SWEEP-FOR-REVIEW-MARKERS-PERIODICALLY Sweep for `REVIEW:` periodically; every hit is an
open decision the human owes, or a removal someone forgot. @impl/done

##two-standing-prohibitions Two standing prohibitions: @impl/done

- ##NEVER-REMOVE-ANOTHER-WRITERS-MARKER-WITHOUT-RESOLVING-IT never remove another writer's marker
  without resolving it (that is deleting mail unread), @impl/done
- ##NEVER-RESOLVE-YOUR-OWN-MARKER-BY-ASSUMING-AGREEMENT and never
  "resolve" your own marker by deciding the human would probably agree. @impl/done

## Re-derive for your project {#re-derive}

##this-document-states-the-practice-in-project-neutral-terms This document states the practice in project-neutral terms. @impl/done

##ADAPT-IT-BY-HANDING-YOUR-AGENT-THE-TASK Adapt it
by handing your agent the task, not a copied template: @impl/done

```
Read this flow's documents (your project installed them — typically `vibedeps/flow-conflict-protocol/<version>/spec/flows/conflict-protocol/`, check `vibe.lock`) end to end. Then adapt the
practice to this project:
1. Name the writers: which humans and which agents edit the spec
   tree, the tests, and the code.
2. Name the layers: list our spec documents, test suites, and any
   volatile state files, and restate the hierarchy in those names.
3. Fix the marker syntax per file type (HTML comment in Markdown,
   line comment in code) and the exact grep pattern for sweeps.
4. Fix the surfacing channel: where end-of-session reports live and
   where open markers get listed.
5. Write the result to spec/flows/conflict-protocol/local.md, and
   add the hierarchy one-liner plus the Never list to the boot file.
Show me the draft before writing anything.
```

## Summary {#summary}

- ##SUM-CONTRADICTION-IS-COOPERATION-ONLY-SILENCE-DAMAGES Two writers on one file set will contradict each other; that is
  cooperation working. Only *silent* contradiction does damage. @spec/done
- ##SUM-FIXED-PRIORITY-SETTLES-EVERYTHING Fixed priority settles everything: Human > Spec > Tests > Code >
  WAL. Recency settles nothing. @impl/done
- ##SUM-THE-REVIEW-CYCLE Disagree with the spec? Implement it anyway, mark `REVIEW:` with a
  reason, surface it in the report, let the human rule next cycle. @impl/done
- ##SUM-A-TEST-CONTRADICTING-THE-SPEC-IS-A-BUG-IN-ONE-OF-THE-TWO A test contradicting the spec is a bug in exactly one of the two —
  never both, and never fixed by weakening the test blind. @impl/done
- ##SUM-MARKERS-ARE-MESSAGES-IN-FLIGHT Markers are messages in flight: placed, surfaced, resolved, removed
  in the resolving change. Aging markers mean the channel is broken. @impl/done
