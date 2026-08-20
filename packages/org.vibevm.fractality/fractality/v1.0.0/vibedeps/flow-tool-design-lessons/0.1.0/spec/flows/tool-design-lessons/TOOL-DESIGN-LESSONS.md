# Tool Design Lessons {#root}

**Scope of this document.** This file is the catalog index: *what* this
package is, *which* lessons it carries and where each one lives, the
three cross-cutting maxims that sit above the individual lessons, and
the meta-lesson that keeps a catalog like this from decaying. The
lessons themselves live in two sibling documents; this one is the map.

## What this catalog is {#what}

These are paid-for lessons from building a tool that manages its own
versions and the package ecosystem around it. Each is a scar: a design
that seemed reasonable, shipped, and taught its cost. A lesson is not a
tutorial — it is **context + the law + why**, self-contained, so you
can read the one that governs the decision in front of you and skip the
rest.

The audience is tool authors: anyone building a self-updating CLI, an
installer, a version manager, or a package system. The vocabulary is
generic on purpose — "the tool", "the active version", "the instance
directory", "the package" — because the laws are portable even though
the mechanics that prove them were written against one platform.

## Index {#index}

| ID | The law (one line) | Lives in |
|----|--------------------|----------|
| S1 | The active version is a live pointer file read per launch, plus the running binary's own path; env is advisory. | [`self-updating-tools.md#live-pointer`](self-updating-tools.md#live-pointer) |
| S2 | The unit of install and switch is a whole immutable instance directory; activation is a pointer flip. | [`self-updating-tools.md#immutable-instances`](self-updating-tools.md#immutable-instances) |
| S3 | Never content-hash a large payload for identity; use a monotonic counter and cheap change detection. | [`self-updating-tools.md#cheap-identity`](self-updating-tools.md#cheap-identity) |
| S4 | Hold sources by reference; never bulk-copy them into the tool's own storage. | [`self-updating-tools.md#sources-by-reference`](self-updating-tools.md#sources-by-reference) |
| S5 | Edit durable environment state idempotently, additively, with consent — behind a seam tests can stub. | [`self-updating-tools.md#durable-env-edits`](self-updating-tools.md#durable-env-edits) |
| S6 | Keep required tools in one table the doctor reads and a test asserts. | [`self-updating-tools.md#runnable-knowledge`](self-updating-tools.md#runnable-knowledge) |
| S7 | Removal protects the active and the running instance; a full wipe needs an explicit flag and a reconfirm. | [`self-updating-tools.md#safe-removal`](self-updating-tools.md#safe-removal) |
| P1 | A package is a project — the same layout, no package-only convention to learn. | [`packaging-lessons.md#package-is-project`](packaging-lessons.md#package-is-project) |
| P2 | Ship the runtime, not a description of it. | [`packaging-lessons.md#ship-runtime`](packaging-lessons.md#ship-runtime) |
| P3 | Identity is the source; exclude build artifacts by denylist, never a per-file allow-list. | [`packaging-lessons.md#identity-is-source`](packaging-lessons.md#identity-is-source) |
| P4 | Build output goes to a gitignored location, never the committed tree or the identity hash. | [`packaging-lessons.md#build-output-elsewhere`](packaging-lessons.md#build-output-elsewhere) |
| P5 | Vendor and commit the bootstrap toolchain beside the code that needs it. | [`packaging-lessons.md#vendor-bootstrap`](packaging-lessons.md#vendor-bootstrap) |
| P6 | Spike the risky topology on the target platform before the irreversible move; keep an evidence-chosen fallback. | [`packaging-lessons.md#spike-first`](packaging-lessons.md#spike-first) |
| P7 | Extract the general mechanism when the second consumer arrives, not before. | [`packaging-lessons.md#build-on-demand`](packaging-lessons.md#build-on-demand) |

## Cross-cutting maxims {#maxims}

Three design principles run under most of the lessons above. They are
not about self-update or packaging specifically; they are about how a
tool that both reasons and acts should be shaped.

### Split by strength, not as a workaround {#maxim-split}

When one component holds durable domain knowledge and another holds the
live context, let the first **author** the instruction and the second
**execute** it. A domain tool carries stable, algorithmic knowledge of
its own rules, so an instruction it composes is more trustworthy than
one improvised from scratch; the executor with the live context is the
better hand to carry it out. The division is by strength, not a
workaround for a missing feature — which is why it survives once the
missing feature arrives.

### One operation, thin transports {#maxim-transports}

Define a reasoning or acting operation **once**, as a transport-agnostic
core, and expose it through thin adapters — a one-shot command line, a
persistent server, an in-process call. Each adapter only marshals input
and output; the core never knows which one called it. The payoff is
that a new transport (or a new caller) costs an adapter, not a
re-implementation, and the operation's behaviour cannot drift between
the ways it is reached.

### Fail loud, never degrade silently {#maxim-fail-loud}

A missing required capability is an error, not a reason to quietly do
less. If an operation needs a backend, a toolchain, or a permission it
does not have, it stops with a message that names exactly what is
missing and how to supply it. Silent degradation trains users to
mistrust success, because they can no longer tell a real result from a
downgraded one.

## The meta-lesson — record the why {#meta-lesson}

Every lesson here records not just the law but the failure that taught
it, "so a cold reader sees *why*, not just *what*." That is the
meta-lesson, and it is load-bearing. A design document that states only
the decision — "the unit of install is a directory" — decays into
cargo-cult: the next author obeys the shape without the reason, cannot
tell when the reason has expired, and either ossifies a stale rule or
discards a live one by accident. The reason is the only part that lets
a future reader re-decide. Record the constraint that forced the
choice, and the choice becomes revisable instead of sacred.

## Re-derive for your project {#re-derive}

The laws are portable; the mechanics are yours to re-derive from your
own platform's constraints. Hand this to an agent, or walk it yourself,
before writing code:

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

- The catalog is fourteen scars: S1–S7 on self-updating tools, P1–P7
  on packaging. Read the one that governs your decision.
- Three maxims sit above them: split by strength, one operation behind
  thin transports, fail loud instead of degrading.
- Every lesson records the failure that taught it — a decision without
  its reason decays into cargo-cult.
- Re-derive the mechanics for your platform; the laws port, the
  implementation does not.
