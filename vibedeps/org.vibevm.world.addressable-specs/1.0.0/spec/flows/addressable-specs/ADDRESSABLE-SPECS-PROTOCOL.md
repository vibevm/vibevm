# Addressable Specs Protocol {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** This file defines *what* addressability
is, *why* it is the first engineering requirement on a spec tree
shared between a human and a coding agent, the `spec://` URI scheme
and its anchor grammar, the single-source and placement rules that
follow, and the dependency graph addressable specs create for free. @status:impl/done

@fact:sibling-document-pointers How to write the units is in [`authoring-rules.md`](authoring-rules.md);
where the files live is in [`spec-tree-layout.md`](spec-tree-layout.md). @status:impl/done

## Specs are IPC, not documentation {#ipc}

@fact:HUMAN-AND-AGENT-ARE-TWO-PROCESSES-SHARING-ONE-REPOSITORY A human and a coding agent are two processes sharing one repository. @status:spec/done

@fact:THE-SPEC-TREE-IS-THE-ONLY-CHANNEL Between them there is no hallway conversation, no memory that
survives the session — the spec tree is the *only* channel through
which intent crosses the process boundary. @status:spec/done

@fact:documentation-is-optional Documentation is
optional; projects limp along without it. @status:spec/done

@fact:AN-IPC-CHANNEL-IS-NOT-OPTIONAL An IPC channel is not
optional: when it breaks, the system stops. @status:spec/done

@fact:ADDRESSABILITY-IS-THE-FIRST-REQUIREMENT Treating spec files as
IPC imposes engineering requirements documentation never had, and
the first of them — ahead of atomicity, ahead of conflict rules — is
**addressability**: every element in every file must be precisely
pointable. @status:impl/done

## Why addressability is requirement #1 {#why}

@fact:THE-HUMANS-ROLE-IS-COHERENCE-MANAGEMENT The human's role in the two-process system is coherence management:
notice that the agent deviated from the spec, and correct it. @status:spec/done

@fact:the-human-knows-instantly The
human knows *instantly* what is violated. @status:spec/done

@fact:THE-BOTTLENECK-IS-TELLING-THE-MACHINE The bottleneck is telling
the machine — feedback latency is where working days go to die. @status:spec/done

@fact:two-ways-lead Two ways to deliver the same correction: @status:impl/done

```
Way 1:  "You did the verification wrong."

Way 2:  "You are violating
         spec://com.example.shop/PROP-001#verification.timeout —
         the timeout must be 600 s, and you wrote 300 s."
```

|              | Way 1 — paraphrase | Way 2 — URI citation |
|--------------|--------------------|----------------------|
| @fact:ROW-AGENT-MUST Agent must @status:spec/done | guess what "verification" maps to here, guess what "wrong" means, form a hypothesis, attempt a fix @status:spec/done | open the file, jump to the anchor, read the value, compare, fix @status:spec/done |
| @fact:ROW-TOKEN-COST Token cost @status:spec/done | hundreds, spent on search and hypothesis @status:spec/done | about twenty @status:spec/done |
| @fact:ROW-RESULT Result @status:spec/done | may not match what the human meant @status:spec/done | exact hit @status:spec/done |

@fact:the-difference-is-an-order-of-magnitude The difference is an order of magnitude, paid on *every* correction,
several times per session — in metered tokens and in minutes of a
short human day. @status:spec/done

@fact:PARAPHRASE-KEEPS-A-NICHE Paraphrase keeps a niche: a sweeping refactor or a
philosophical re-orientation rightly starts from "re-read the whole
spec and rethink". @status:impl/done

@fact:FOR-POINT-CORRECTIONS-THE-URI-WINS For point corrections — a wrong constant, a
missing parameter, a violated invariant — the URI wins every time. @status:impl/done

## The URI scheme {#uri-scheme}

```
spec://<group>/<name>[@<version>]/<doc-path>#<section>[.<sub>…][~r<N>]
```

@fact:URI-SCHEME-IS-THE-FULL-GRAMMAR This is the **whole** grammar — the section stopped publishing a subset of what implementations resolve (2026-08-04, closing the split the vibevm host measured): the authority is a package coordinate, the version and the revision pin are optional extensions, and a project's root is a package like any other (no host exception exists). @status:impl/done

| Segment     | Meaning | Example |
|-------------|---------|---------|
| @fact:ROW-SEGMENT-GROUP-NAME `<group>/<name>` @status:impl/done | the package coordinate: reverse-DNS group, then package name, joined by `/` (never `.`) — the root project included, addressed by its own declared coordinate @status:impl/done | `org.vibevm.world/wal` @status:impl/done |
| @fact:ROW-SEGMENT-VERSION `[@<version>]` @status:impl/done | **optional — a feature, never an obligation** (owner-ruled 2026-08-04): absent, the address resolves against the **freshest installed version** (semver-newest); explicit, it picks its exact slot, including a non-newest one @status:impl/done | `@0.8.0` @status:impl/done |
| @fact:ROW-SEGMENT-MODULE `<module>` @status:impl/done | inside `<doc-path>`: the spec module — a directory under `spec/modules/`, or `common` @status:impl/done | `modules/vibe-registry` @status:impl/done |
| @fact:ROW-SEGMENT-DOC `<doc>` @status:impl/done | inside `<doc-path>`: the document name, extension dropped @status:impl/done | `PROP-001` @status:impl/done |
| @fact:ROW-SEGMENT-SECTION `<section>` @status:impl/done | the `{#anchor}` of a heading in that document @status:impl/done | `verification` @status:impl/done |
| @fact:ROW-SEGMENT-SUB `.<sub>` @status:impl/done | dotted hierarchy inside the anchor namespace @status:impl/done | `verification.timeout` @status:impl/done |
| @fact:ROW-SEGMENT-REVISION-PIN `[~r<N>]` @status:impl/done | pins a **spec-unit revision** for drift detection — never a package version (the `@` half owns that) @status:impl/done | `#verification~r2` @status:impl/done |

@fact:why-a-uri-and-not-a-bespoke-notation Why a URI and not a bespoke notation? @status:spec/done

@fact:the-model-already-knows-the-uri-shape Because the model already
knows, from billions of URLs and RFCs in its training data, that
`something://path/to/thing#anchor` points at a specific resource. @status:spec/done

@fact:THE-SCHEME-EXPLOITS-SEMANTICS-THE-AGENT-CARRIES Nothing has to be taught — the scheme exploits semantics the agent
already carries. @status:spec/done

## Anchors {#anchors}

@fact:ANCHORS-ARE-EXPLICIT-HEADING-IDS Inside a document, addressability is implemented with explicit
heading anchors — `{#id}` — a standard extended-Markdown syntax: @status:impl/done

```markdown
# PROP-001: Payments protocol {#root}

## 5. Verification flow {#verification}

### 5.3 Timeout {#verification.timeout}
Unverified payments older than 600 seconds get status TIMEOUT.
```

@fact:the-third-heading-citation That third heading is cited as
`spec://com.example.shop/PROP-001#verification.timeout`. @status:impl/done

@fact:RENDERERS-TURN-THE-ANCHOR-INTO-A-LINK-TARGET GitHub,
GitLab, and most Markdown renderers turn `{#id}` into a link target,
so the same URI that steers the agent is clickable for the human in
the web Git UI. @status:spec/done

@fact:one-address-two-consumers One address, two consumers. @status:impl/done

@fact:THE-DOT-IS-HIERARCHY The dot in `verification.timeout` is hierarchy: section `timeout`
inside section `verification`. @status:impl/done

@fact:DOTS-NAMESPACE-THE-ANCHORS Dots namespace the anchors —
`#verification.timeout` and `#connection.timeout` coexist in one
document without collision. @status:impl/done

## Module names: reverse DNS {#modules}

@fact:MODULE-NAMES-USE-REVERSE-DNS When specs may ever leave the project — shared across repositories,
published, or merely grep'd from a monorepo — module names use
reverse-DNS notation: `com.example.shop`, dots again available for
submodules (`com.example.shop.payments`). @status:impl/done

@fact:the-convention-is-java-package-naming The convention is Java
package naming, introduced by Sun in the mid-1990s and fixed in the
Java Language Specification: global uniqueness for free, piggybacked
on a uniqueness system that already exists — domain names, written
backwards. @status:spec/done

@fact:there-is-a-chat-shortcut There is a chat shortcut: tell the agent "we are working inside
module `com.example.shop`; resolve spec URIs relative to that base". @status:impl/done

@fact:the-shortcut-costs-a-lookup It works, but it makes the model run a lookup akin to C++
argument-dependent lookup on every resolution. @status:spec/done

@fact:IN-SPEC-FILES-WRITE-THE-FULL-ADDRESS Acceptable typed once
in a chat; inside spec files, re-read dozens of times per session,
write the full address. @status:impl/done

@fact:PACKAGE-MODULE-AUTHORITY-IS-THE-FULL-COORDINATE **For a package, the module authority MUST be the package's full
coordinate `<group>/<name>`** — the name is the first path segment,
`/`-joined exactly as in a pkgref (e.g.
`org.vibevm.ai-native/rust-ai-native-lang`), never a bare
`rust-ai-native-lang`. @status:impl/done

@fact:THE-SLASH-MAKES-THE-BOUNDARY-DETERMINISTIC Where the surface allows it
(`spec://` authority, pkgref), `/` is the joiner — in neither the group
(`[a-z0-9.-]`) nor the name (`[a-z0-9-]`), so the boundary splits
deterministically with no grammar to consult. The dot is not thereby
forbidden in general: the name alphabet `[a-z0-9-]` already excludes it,
so a dot-free name makes the last dot the boundary just as deterministically
— the old prohibition (groups are themselves dotted) was refuted by these
same alphabets, not only by external ruling. @status:impl/done

@fact:A-JOINER-MUST-ADMIT-A-DETERMINISTIC-INVERSE-PARSE The requirement on a
joiner is not "not a dot" but that the coordinate admit a **deterministic
inverse parse**, and exactly two things satisfy it. Either the separator lies
outside both alphabets (`/` here, and `_` here) — then the split needs no
grammar at all; or the grammar guarantees that one half cannot contain the
separator, and then the *outermost* occurrence on that half's side is the
boundary: a separator-free second half is split at the last occurrence, a
separator-free first half at the first. An ecosystem whose grammar guarantees
neither half must take the first route. @status:impl/done

@fact:A-BLANKET-DOT-BAN-COSTS-THE-FLAT-SURFACES Flat surfaces — repository
names, artifact names — commonly forbid `/` outright, so a blanket
dot-prohibition forces those surfaces onto a joiner their own domain rules
contradict, where a both-LDH `<group>.<name>` is itself a valid reversed
FQDN. vibevm moved its flat carrier onto the dot by owner ruling 2026-08-13
(`PROP-029` `##CARRIER-REPO-SWAP`, `##joiner-why`); `_` stays legal wherever
the name grammar gives no guarantee. @status:impl/done

@fact:THE-FULL-COORDINATE-IS-MECHANICALLY-REFACTORABLE A
bare authority resolves only with ambient context (which package am I
in?); the full coordinate is a
self-contained global symbol, which is what makes every `spec://`
citation *mechanically* refactorable — an algorithm rewrites all
occurrences on a rename, no resolver and no model in the loop. @status:impl/done

@fact:this-is-the-addressing-half-of-prop-029 This is
the addressing half of vibevm's PROP-029 (fully-qualified addresses and
mechanical refactoring). @status:impl/done

## Single source of truth {#single-source}

@fact:EVERY-FACT-HAS-EXACTLY-ONE-AUTHORITATIVE-ANCHOR Every fact has exactly one authoritative anchor. @status:impl/done

@fact:copying-a-value-is-a-time-bomb Citing the anchor
is free; copying its value into a second file is a time bomb: one
copy gets edited, the other does not, and a later session finds
600 s in one file and 300 s in the other with no way to know which
binds. @status:spec/done

@fact:it-fixes-the-wrong-copy It "fixes" the wrong copy — or worse, the code. @status:spec/done

@fact:one-bug-becomes-three One bug
becomes three, and untangling them means replaying weeks of git
history. @status:spec/done

@fact:DUPLICATION-IS-NOT-REDUNDANCY Duplication is not redundancy — redundancy implies a
reconciliation mechanism, and copies have none. @status:spec/done

@fact:A-NORMATIVE-VALUE-LIVES-AT-EXACTLY-ONE-ANCHOR The rule: a normative value lives at exactly one anchor. @status:impl/done

@fact:EVERY-OTHER-DOCUMENT-CITES-THE-URI Every other
document cites the URI and lets the reader — human or model —
resolve it. @status:impl/done

@fact:A-RESTATEMENT-NAMES-ITS-ANCHOR If prose flow demands restating the value, the
restatement names its anchor in the same sentence, marking which
copy is the echo. @status:impl/done

## Never cite something built to disappear {#disposable-targets}

@fact:A-PLAN-ROW-IS-NOT-AN-ADDRESS A plan, a backlog row, a campaign document — anything a project
creates in order to finish and delete — is **not an address**. Cite the
specification element the work established, never the row that
commissioned it. @status:impl/done

@fact:WHY-A-DISPOSABLE-TARGET-IS-WORSE-THAN-A-MISSING-ONE This is stricter than it sounds, and the reason is that the
failure is silent. A citation to a deleted anchor at least breaks
loudly. A citation to a plan row that still exists keeps resolving —
to a row describing work as it was *imagined*, long after the work
shipped and changed shape. The reader finds a plausible, dated, wrong
answer and has no signal that it is wrong. @status:spec/done

@fact:ON-CLOSURE-CONTENT-MOVES-INTO-THE-SPECS **On closure, significant content moves into the
specifications**, and every statement pointing at the closed item is
re-pointed at the spec element that now carries the fact. Where the
item established no contract — it was a finding, not a rule — the
citation becomes plain text and the row number survives only as
history. @status:impl/done

@fact:REBUILDING-THE-CITATIONS-IS-PART-OF-CLOSING **Rebuilding the citations is part of closing, not tidying
afterwards.** An item is closed when no tail remains: content moved,
statements re-pointed, and the moved content judged in the same pass
that moved it. Deferring that step is how a project accumulates
addresses that outlive their targets. @status:impl/done

@fact:THE-LIFECYCLE-HALF-LIVES-IN-CAMPAIGN-PLANS The other half of this rule — that a plan is a temporary
artifact and what its tombstones are for — belongs to the campaign-plans
flow, at
`spec://org.vibevm.world/campaign-plans/flows/campaign-plans/CAMPAIGN-PLAN-FORMAT#temporary`.
One law, two homes, one pointer: this half is about addressing, that
half is about a plan's life. @status:impl/done

## Placement: Lost in the Middle {#placement}

@fact:MODELS-ATTEND-TO-THE-BEGINNING-AND-THE-END An empirical result, not a style preference: language models attend
most reliably to the beginning and the end of a long context; facts
placed in the middle lose retrieval accuracy, with drops of up to
thirty percent measured ("Lost in the Middle", Liu et al., arXiv
preprint 2023; TACL 2024). @status:spec/done

@fact:a-spec-document-is-context A spec document is context; the same
U-curve applies. @status:spec/done

@fact:INVARIANTS-GO-AT-THE-OPENING-OR-THE-END Constraints, acceptance criteria, and unbreakable
invariants therefore go in the opening paragraphs or a final
"Invariants" section — never buried mid-document. @status:impl/done

@fact:A-MID-FILE-INVARIANT-WAS-NOT-READ A mid-file
invariant is an invariant the agent statistically did not read. @status:spec/done

## The graph consequence {#graph}

@fact:ADDRESSABLE-SPECS-GIVE-A-DEPENDENCY-GRAPH-FOR-FREE Addressable specs give the project a dependency graph for free. @status:impl/done

@fact:CODE-MARKS-WHAT-IT-IMPLEMENTS-THE-SPEC-WHAT-VERIFIES-IT Code
marks what it implements; the spec records what verifies it: *(the two
forms below are the plain-text ones, and they are what needs no tooling.
Where a project mechanizes the graph, both records are commonly authored
on the **code** side instead — the implements edge as a language-native
tag on the item, and the verification edge as the same kind of tag on
the **test** rather than as a `Test:` line in the document — and the
spec-side answer, "what verifies this unit", is then rendered from the
graph rather than maintained by hand. Either form yields the same
bidirectional edge; only the authoring side moves.)* @status:impl/done

```
// Implements: spec://com.example.shop/PROP-001#verification.timeout
```

```markdown
### 5.3 Timeout {#verification.timeout}
Test: payments_core::tests::timeout_marks_old_messages
```

@fact:THESE-ARE-BIDIRECTIONAL-EDGES These are bidirectional edges: when one side changes, the other must
be re-checked. @status:impl/done

@fact:NO-TOOLING-IS-REQUIRED-TO-BENEFIT No tooling is required to benefit — a plain
`grep -rn "PROP-001#verification.timeout"` answers "which code
implements this unit", the `Test:` line answers "which test verifies
it", and a failing test carries the address of the violated unit. Where
the graph is mechanized, the same two questions are answered by the
rendered view over the code-side tags instead (§[graph](#graph)). @status:impl/done

@fact:the-graph-pays-off-from-the-first-marker Tools can mechanize the check later; the graph is useful the day
the first marker lands. @status:impl/done

## Re-derive for your project {#re-derive}

@fact:COPY-THE-PROMPT-TASK-NOT-THE-IMPLEMENTATION Do not transplant these documents as dogma — copy the prompt-task,
not the prompt-implementation. @status:impl/done

@fact:re-derive-prompt-lead Paste this and review the plan: @status:impl/done

```
Read this flow's documents (your project installed them — typically `vibedeps/flow-addressable-specs/<version>/spec/flows/addressable-specs/`, check `vibe.lock`) in this repository — all three
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

- @fact:SUM-SPECS-ARE-THE-IPC-CHANNEL Spec files are the IPC channel between human and agent;
  addressability is that channel's first requirement. @status:impl/done
- @fact:SUM-URI-CORRECTION-COSTS-TWENTY-TOKENS Correction by URI costs ~20 tokens and hits exactly; by
  paraphrase, hundreds — and it may miss. @status:spec/done
- @fact:SUM-THE-URI-SCHEME `spec://<module>/<doc>#<section>[.<sub>]`; anchors are `{#id}`,
  dots are hierarchy, modules reverse-DNS when specs can be shared. @status:impl/done
- @fact:SUM-ONE-FACT-ONE-ANCHOR One fact, one anchor. Copies diverge silently; cite instead. @status:impl/done
- @fact:SUM-INVARIANTS-AT-THE-EDGES Invariants live at the start or end of a file — the middle is
  where models stop reading. @status:impl/done
- @fact:SUM-THE-BIDIRECTIONAL-GRAPH `Implements:` markers plus `Test:` lines form a
  bidirectional graph that pays off with zero tooling — and where the graph is
  mechanized, the same two edges are authored as code-side tags and the
  spec-side view is rendered (§[graph](#graph)). @status:impl/done
