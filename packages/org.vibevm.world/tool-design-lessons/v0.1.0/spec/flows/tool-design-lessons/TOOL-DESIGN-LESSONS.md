# Tool Design Lessons {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** This file is the catalog index: *what* this
package is, *which* lessons it carries and where each one lives, the
three cross-cutting maxims that sit above the individual lessons, and
the meta-lesson that keeps a catalog like this from decaying. @impl/done

##the-lessons-live-in-two-sibling-documents The
lessons themselves live in two sibling documents; this one is the map. @impl/done

## What this catalog is {#what}

##THESE-ARE-PAID-FOR-LESSONS These are paid-for lessons from building a tool that manages its own
versions and the package ecosystem around it. @spec/done

##EACH-LESSON-IS-A-SCAR Each is a scar: a design
that seemed reasonable, shipped, and taught its cost. @spec/done

##A-LESSON-IS-CONTEXT-THE-LAW-AND-WHY A lesson is not a
tutorial — it is **context + the law + why**, self-contained, so you
can read the one that governs the decision in front of you and skip the
rest. @impl/done

##THE-AUDIENCE-IS-TOOL-AUTHORS The audience is tool authors: anyone building a self-updating CLI, an
installer, a version manager, or a package system. @impl/done

##THE-VOCABULARY-IS-GENERIC-ON-PURPOSE The vocabulary is generic
on purpose — "the tool", "the active version", "the instance
directory", "the package" — because the laws are portable even though
the mechanics that prove them were written against one platform. @impl/done

## Index {#index}

| ID | The law (one line) | Lives in |
|----|--------------------|----------|
| ##ROW-LESSON-S1 S1 @impl/done | The active version is a live pointer file read per launch, plus the running binary's own path; env is advisory. @impl/done | [`self-updating-tools.md#live-pointer`](self-updating-tools.md#live-pointer) @impl/done |
| ##ROW-LESSON-S2 S2 @impl/done | The unit of install and switch is a whole immutable instance directory; activation is a pointer flip. @impl/done | [`self-updating-tools.md#immutable-instances`](self-updating-tools.md#immutable-instances) @impl/done |
| ##ROW-LESSON-S3 S3 @impl/done | Never content-hash a large payload for identity; use a monotonic counter and cheap change detection. @impl/done | [`self-updating-tools.md#cheap-identity`](self-updating-tools.md#cheap-identity) @impl/done |
| ##ROW-LESSON-S4 S4 @impl/done | Hold sources by reference; never bulk-copy them into the tool's own storage. @impl/done | [`self-updating-tools.md#sources-by-reference`](self-updating-tools.md#sources-by-reference) @impl/done |
| ##ROW-LESSON-S5 S5 @impl/done | Edit durable environment state idempotently, additively, with consent — behind a seam tests can stub. @impl/done | [`self-updating-tools.md#durable-env-edits`](self-updating-tools.md#durable-env-edits) @impl/done |
| ##ROW-LESSON-S6 S6 @impl/done | Keep required tools in one table the doctor reads and a test asserts. @impl/done | [`self-updating-tools.md#runnable-knowledge`](self-updating-tools.md#runnable-knowledge) @impl/done |
| ##ROW-LESSON-S7 S7 @impl/done | Removal protects the active and the running instance; a full wipe needs an explicit flag and a reconfirm. @impl/done | [`self-updating-tools.md#safe-removal`](self-updating-tools.md#safe-removal) @impl/done |
| ##ROW-LESSON-P1 P1 @impl/done | A package is a project — the same layout, no package-only convention to learn. @impl/done | [`packaging-lessons.md#package-is-project`](packaging-lessons.md#package-is-project) @impl/done |
| ##ROW-LESSON-P2 P2 @impl/done | Ship the runtime, not a description of it. @impl/done | [`packaging-lessons.md#ship-runtime`](packaging-lessons.md#ship-runtime) @impl/done |
| ##ROW-LESSON-P3 P3 @impl/done | Identity is the source; exclude build artifacts by denylist, never a per-file allow-list. @impl/done | [`packaging-lessons.md#identity-is-source`](packaging-lessons.md#identity-is-source) @impl/done |
| ##ROW-LESSON-P4 P4 @impl/done | Build output goes to a gitignored location, never the committed tree or the identity hash. @impl/done | [`packaging-lessons.md#build-output-elsewhere`](packaging-lessons.md#build-output-elsewhere) @impl/done |
| ##ROW-LESSON-P5 P5 @impl/done | Vendor and commit the bootstrap toolchain beside the code that needs it. @impl/done | [`packaging-lessons.md#vendor-bootstrap`](packaging-lessons.md#vendor-bootstrap) @impl/done |
| ##ROW-LESSON-P6 P6 @impl/done | Spike the risky topology on the target platform before the irreversible move; keep an evidence-chosen fallback. @impl/done | [`packaging-lessons.md#spike-first`](packaging-lessons.md#spike-first) @impl/done |
| ##ROW-LESSON-P7 P7 @impl/done | Extract the general mechanism when the second consumer arrives, not before. @impl/done | [`packaging-lessons.md#build-on-demand`](packaging-lessons.md#build-on-demand) @impl/done |

## Cross-cutting maxims {#maxims}

##three-design-principles-run-under-the-lessons Three design principles run under most of the lessons above. @impl/done

##the-maxims-are-about-how-a-tool-should-be-shaped They are
not about self-update or packaging specifically; they are about how a
tool that both reasons and acts should be shaped. @spec/done

### Split by strength, not as a workaround {#maxim-split}

##LET-THE-KNOWLEDGE-HOLDER-AUTHOR-AND-THE-CONTEXT-HOLDER-EXECUTE When one component holds durable domain knowledge and another holds the
live context, let the first **author** the instruction and the second
**execute** it. @impl/done

##a-composed-instruction-is-more-trustworthy-than-an-improvised-one A domain tool carries stable, algorithmic knowledge of
its own rules, so an instruction it composes is more trustworthy than
one improvised from scratch; the executor with the live context is the
better hand to carry it out. @spec/done

##THE-DIVISION-IS-BY-STRENGTH-NOT-A-WORKAROUND The division is by strength, not a
workaround for a missing feature — which is why it survives once the
missing feature arrives. @spec/done

### One operation, thin transports {#maxim-transports}

##DEFINE-THE-OPERATION-ONCE-AND-EXPOSE-IT-THROUGH-THIN-ADAPTERS Define a reasoning or acting operation **once**, as a transport-agnostic
core, and expose it through thin adapters — a one-shot command line, a
persistent server, an in-process call. @impl/done

##AN-ADAPTER-ONLY-MARSHALS-AND-THE-CORE-NEVER-KNOWS-ITS-CALLER Each adapter only marshals input
and output; the core never knows which one called it. @impl/done

##a-new-transport-costs-an-adapter-not-a-re-implementation The payoff is
that a new transport (or a new caller) costs an adapter, not a
re-implementation, and the operation's behaviour cannot drift between
the ways it is reached. @spec/done

### Fail loud, never degrade silently {#maxim-fail-loud}

##A-MISSING-REQUIRED-CAPABILITY-IS-AN-ERROR A missing required capability is an error, not a reason to quietly do
less. @impl/done

##STOP-WITH-A-MESSAGE-NAMING-WHAT-IS-MISSING If an operation needs a backend, a toolchain, or a permission it
does not have, it stops with a message that names exactly what is
missing and how to supply it. @impl/done

##silent-degradation-trains-users-to-mistrust-success Silent degradation trains users to
mistrust success, because they can no longer tell a real result from a
downgraded one. @spec/done

## The meta-lesson — record the why {#meta-lesson}

##EVERY-LESSON-RECORDS-THE-FAILURE-THAT-TAUGHT-IT Every lesson here records not just the law but the failure that taught
it, "so a cold reader sees *why*, not just *what*." @impl/done

##THAT-IS-THE-META-LESSON-AND-IT-IS-LOAD-BEARING That is the meta-lesson, and it is load-bearing. @impl/done

##a-decision-only-document-decays-into-cargo-cult A design document that states only
the decision — "the unit of install is a directory" — decays into
cargo-cult: the next author obeys the shape without the reason, cannot
tell when the reason has expired, and either ossifies a stale rule or
discards a live one by accident. @spec/done

##THE-REASON-IS-WHAT-LETS-A-FUTURE-READER-RE-DECIDE The reason is the only part that lets
a future reader re-decide. @spec/done

##RECORD-THE-CONSTRAINT-THAT-FORCED-THE-CHOICE Record the constraint that forced the
choice, and the choice becomes revisable instead of sacred. @impl/done

## Re-derive for your project {#re-derive}

##THE-LAWS-ARE-PORTABLE-THE-MECHANICS-ARE-YOURS The laws are portable; the mechanics are yours to re-derive from your
own platform's constraints. @impl/done

##re-derive-lead Hand this to an agent, or walk it yourself,
before writing code: @impl/done

```
You are designing a self-updating tool or a package format for
<project> on <target platform>. Before writing code, answer each
question and record the answer, with its reason, as a decision record:
1. Unit of install and switch: a single file, or a whole directory?
   What is in use while it runs, and how do you avoid overwriting it?
2. Active-version truth: an environment variable, or a file read each
   launch? Which one switches without a console reload?
3. Identity: the source, or the build output? What must it never
   include, and do you exclude by denylist or by allow-list?
4. Host requirements: where is the build's tool list written so a
   doctor can check it and a test can assert it?
5. Durable machine edits: which state, and how is the edit idempotent,
   additive, consented, and testable off the real box?
Answer from your platform's edges, not from this catalog's examples.
```

## Summary {#summary}

- ##SUM-THE-CATALOG-IS-FOURTEEN-SCARS The catalog is fourteen scars: S1–S7 on self-updating tools, P1–P7
  on packaging. Read the one that governs your decision. @impl/done
- ##SUM-THREE-MAXIMS-SIT-ABOVE-THEM Three maxims sit above them: split by strength, one operation behind
  thin transports, fail loud instead of degrading. @impl/done
- ##SUM-EVERY-LESSON-RECORDS-THE-FAILURE-THAT-TAUGHT-IT Every lesson records the failure that taught it — a decision without
  its reason decays into cargo-cult. @impl/done
- ##SUM-RE-DERIVE-THE-MECHANICS-FOR-YOUR-PLATFORM Re-derive the mechanics for your platform; the laws port, the
  implementation does not. @impl/done
