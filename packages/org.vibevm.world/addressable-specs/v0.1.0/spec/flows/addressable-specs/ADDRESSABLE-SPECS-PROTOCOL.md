# Addressable Specs Protocol {#root}

**Scope of this document.** This file defines *what* addressability
is, *why* it is the first engineering requirement on a spec tree
shared between a human and a coding agent, the `spec://` URI scheme
and its anchor grammar, the single-source and placement rules that
follow, and the dependency graph addressable specs create for free.
How to write the units is in [`authoring-rules.md`](authoring-rules.md);
where the files live is in [`spec-tree-layout.md`](spec-tree-layout.md).

## Specs are IPC, not documentation {#ipc}

A human and a coding agent are two processes sharing one repository.
Between them there is no hallway conversation, no memory that
survives the session — the spec tree is the *only* channel through
which intent crosses the process boundary. Documentation is
optional; projects limp along without it. An IPC channel is not
optional: when it breaks, the system stops. Treating spec files as
IPC imposes engineering requirements documentation never had, and
the first of them — ahead of atomicity, ahead of conflict rules — is
**addressability**: every element in every file must be precisely
pointable.

## Why addressability is requirement #1 {#why}

The human's role in the two-process system is coherence management:
notice that the agent deviated from the spec, and correct it. The
human knows *instantly* what is violated. The bottleneck is telling
the machine — feedback latency is where working days go to die.

Two ways to deliver the same correction:

```
Way 1:  "You did the verification wrong."

Way 2:  "You are violating
         spec://com.example.shop/PROP-001#verification.timeout —
         the timeout must be 600 s, and you wrote 300 s."
```

|              | Way 1 — paraphrase | Way 2 — URI citation |
|--------------|--------------------|----------------------|
| Agent must   | guess what "verification" maps to here, guess what "wrong" means, form a hypothesis, attempt a fix | open the file, jump to the anchor, read the value, compare, fix |
| Token cost   | hundreds, spent on search and hypothesis | about twenty |
| Result       | may not match what the human meant | exact hit |

The difference is an order of magnitude, paid on *every* correction,
several times per session — in metered tokens and in minutes of a
short human day. Paraphrase keeps a niche: a sweeping refactor or a
philosophical re-orientation rightly starts from "re-read the whole
spec and rethink". For point corrections — a wrong constant, a
missing parameter, a violated invariant — the URI wins every time.

## The URI scheme {#uri-scheme}

```
spec://<module>/<doc>#<section>[.<sub>]
```

| Segment     | Meaning | Example |
|-------------|---------|---------|
| `<module>`  | spec module — a directory under `spec/modules/`, or `common` | `com.example.shop` |
| `<doc>`     | document name, extension dropped | `PROP-001` |
| `<section>` | the `{#anchor}` of a heading in that document | `verification` |
| `.<sub>`    | dotted hierarchy inside the anchor namespace | `verification.timeout` |

Why a URI and not a bespoke notation? Because the model already
knows, from billions of URLs and RFCs in its training data, that
`something://path/to/thing#anchor` points at a specific resource.
Nothing has to be taught — the scheme exploits semantics the agent
already carries.

## Anchors {#anchors}

Inside a document, addressability is implemented with explicit
heading anchors — `{#id}` — a standard extended-Markdown syntax:

```markdown
# PROP-001: Payments protocol {#root}

## 5. Verification flow {#verification}

### 5.3 Timeout {#verification.timeout}
Unverified payments older than 600 seconds get status TIMEOUT.
```

That third heading is cited as
`spec://com.example.shop/PROP-001#verification.timeout`. GitHub,
GitLab, and most Markdown renderers turn `{#id}` into a link target,
so the same URI that steers the agent is clickable for the human in
the web Git UI. One address, two consumers.

The dot in `verification.timeout` is hierarchy: section `timeout`
inside section `verification`. Dots namespace the anchors —
`#verification.timeout` and `#connection.timeout` coexist in one
document without collision.

## Module names: reverse DNS {#modules}

When specs may ever leave the project — shared across repositories,
published, or merely grep'd from a monorepo — module names use
reverse-DNS notation: `com.example.shop`, dots again available for
submodules (`com.example.shop.payments`). The convention is Java
package naming, introduced by Sun in the mid-1990s and fixed in the
Java Language Specification: global uniqueness for free, piggybacked
on a uniqueness system that already exists — domain names, written
backwards.

There is a chat shortcut: tell the agent "we are working inside
module `com.example.shop`; resolve spec URIs relative to that base".
It works, but it makes the model run a lookup akin to C++
argument-dependent lookup on every resolution. Acceptable typed once
in a chat; inside spec files, re-read dozens of times per session,
write the full address.

**For a package, the module authority MUST be the package's full
coordinate `<group>.<name>`** — e.g. `org.vibevm.ai-native.rust-ai-native-lang`,
never a bare `rust-ai-native-lang`. A bare authority resolves only with
ambient context (which package am I in?); the full coordinate is a
self-contained global symbol, which is what makes every `spec://`
citation *mechanically* refactorable — an algorithm rewrites all
occurrences on a rename, no resolver and no model in the loop. This is
the addressing half of vibevm's PROP-029 (fully-qualified addresses and
mechanical refactoring).

## Single source of truth {#single-source}

Every fact has exactly one authoritative anchor. Citing the anchor
is free; copying its value into a second file is a time bomb: one
copy gets edited, the other does not, and a later session finds
600 s in one file and 300 s in the other with no way to know which
binds. It "fixes" the wrong copy — or worse, the code. One bug
becomes three, and untangling them means replaying weeks of git
history. Duplication is not redundancy — redundancy implies a
reconciliation mechanism, and copies have none.

The rule: a normative value lives at exactly one anchor. Every other
document cites the URI and lets the reader — human or model —
resolve it. If prose flow demands restating the value, the
restatement names its anchor in the same sentence, marking which
copy is the echo.

## Placement: Lost in the Middle {#placement}

An empirical result, not a style preference: language models attend
most reliably to the beginning and the end of a long context; facts
placed in the middle lose retrieval accuracy, with drops of up to
thirty percent measured ("Lost in the Middle", Liu et al., arXiv
preprint 2023; TACL 2024). A spec document is context; the same
U-curve applies. Constraints, acceptance criteria, and unbreakable
invariants therefore go in the opening paragraphs or a final
"Invariants" section — never buried mid-document. A mid-file
invariant is an invariant the agent statistically did not read.

## The graph consequence {#graph}

Addressable specs give the project a dependency graph for free. Code
marks what it implements; the spec records what verifies it:

```
// Implements: spec://com.example.shop/PROP-001#verification.timeout
```

```markdown
### 5.3 Timeout {#verification.timeout}
Test: payments_core::tests::timeout_marks_old_messages
```

These are bidirectional edges: when one side changes, the other must
be re-checked. No tooling is required to benefit — a plain
`grep -rn "PROP-001#verification.timeout"` answers "which code
implements this unit", the `Test:` line answers "which test verifies
it", and a failing test carries the address of the violated unit.
Tools can mechanize the check later; the graph is useful the day
the first marker lands.

## Re-derive for your project {#re-derive}

Do not transplant these documents as dogma — copy the prompt-task,
not the prompt-implementation. Paste this and review the plan:

```
Read spec/flows/addressable-specs/ in this repository — all three
documents. Adapt the addressable-specs practice to this concrete
project:
1. Propose the URI scheme instance: module names (reverse-DNS if
   these specs could ever be shared), document naming, anchor style.
2. Sweep the existing spec/docs tree: list every heading that states
   a decision, constraint, or contract but carries no {#anchor}.
3. List every invariant buried mid-file; propose moving each to the
   top of its document or into a final "Invariants" section.
4. Find normative values duplicated across files; for each, name the
   one authoritative anchor and the copies to replace with citations.
5. Output all of it as a migration plan (file, anchor, action) and
   stop. Do not edit anything until I approve.
```

## Summary {#summary}

- Spec files are the IPC channel between human and agent;
  addressability is that channel's first requirement.
- Correction by URI costs ~20 tokens and hits exactly; by
  paraphrase, hundreds — and it may miss.
- `spec://<module>/<doc>#<section>[.<sub>]`; anchors are `{#id}`,
  dots are hierarchy, modules reverse-DNS when specs can be shared.
- One fact, one anchor. Copies diverge silently; cite instead.
- Invariants live at the start or end of a file — the middle is
  where models stop reading.
- `Implements:` markers plus `Test:` lines form a bidirectional
  graph that pays off with zero tooling.
