# Spec tree layout {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** The reference directory layout for an
addressable spec tree, the PROP / FEAT distinction, the decision
table for where each kind of fact lives, and the `.human/` private
buffer with its physical-invisibility rule. @status:impl/done

@fact:protocol-document-pointer The addressing scheme the
layout serves is defined in
[`ADDRESSABLE-SPECS-PROTOCOL.md`](ADDRESSABLE-SPECS-PROTOCOL.md). @status:impl/done

## The reference tree {#tree}

```
project/
├── spec/                      # the IPC buffer (shared state)
│   ├── BOOT.md                # entry point — the agent reads this first
│   ├── WAL.md                 # continuation state between sessions
│   ├── SPEC-PROTOCOL.md       # how specs are updated (conflict rules)
│   ├── common/
│   │   ├── PROP-000.md        # foundational decisions
│   │   └── structure.md       # module map
│   └── modules/
│       ├── payments/
│       │   ├── PROP-001.md    # standing decisions for this module
│       │   └── FEAT-001.md    # one feature slice
│       └── client/
├── src/                       # artifacts — generated, verifiable, re-creatable
├── tests/                     # executable specs
├── .human/                    # human-only buffer, excluded from agent view
│   └── shortcuts.md           # copy-paste corrections, private notes
├── .<agent>ignore             # what the agent never sees (.human/ at minimum)
└── CLAUDE.md / AGENTS.md / …  # harness entry files redirecting to spec/BOOT.md
```

@fact:SPEC-OR-SPECS-PICK-ONE `spec/` or `specs/` — either; pick one and never mix. @status:impl/done

@fact:three-zones-lead The tree
divides into three zones with different loss semantics: @status:impl/done

- @fact:ZONE-SHARED-STATE **shared
  state** (`spec/` — losing it is a catastrophe, it is the only channel
  between the two processes), @status:impl/done
- @fact:ZONE-ARTIFACTS **artifacts** (`src/`, `tests/` — losing
  a file is an inconvenience; it can be regenerated from the spec), @status:impl/done
- @fact:ZONE-PRIVATE-BUFFERS and
  **private buffers** (`.human/` — one process's memory, invisible to
  the other). @status:impl/done

@fact:HARNESS-ENTRY-FILES-ARE-THIN-REDIRECTS Harness entry files (`CLAUDE.md`, `AGENTS.md`, and whatever the next
tool demands) should be thin redirects into `spec/BOOT.md`. @status:impl/done

@fact:one-boot-sequence-many-doors One boot
sequence, many doors — the alternative is N slowly diverging copies. @status:spec/done

## PROP vs FEAT {#prop-vs-feat}

|          | PROP | FEAT |
|----------|------|------|
| @fact:ROW-PROP-FEAT-HOLDS Holds @status:impl/done | standing decisions, contracts, protocol rules @status:impl/done | one feature slice: scope, plan, acceptance criteria @status:impl/done |
| @fact:ROW-PROP-FEAT-LIFETIME Lifetime @status:impl/done | in force until superseded — never deleted @status:impl/done | completes and freezes when the slice ships @status:impl/done |
| @fact:ROW-PROP-FEAT-CHANGES Changes @status:impl/done | rarely; every semantic change is a changelog line @status:impl/done | freely while active; frozen afterwards @status:impl/done |
| @fact:ROW-PROP-FEAT-CITED-BY Cited by @status:impl/done | code markers, commits, other specs — long-lived anchors @status:impl/done | the WAL and the commits of its own campaign @status:impl/done |

@fact:PROP-IS-THE-LAW-FEAT-IS-A-PROJECT-UNDER-IT PROP is the law; FEAT is a project under that law. @status:impl/done

@fact:A-LASTING-DECISION-MOVES-UP-INTO-A-PROP When a FEAT
uncovers a lasting decision, the decision moves *up* into a PROP and
the FEAT cites it — the slice document never becomes the permanent
home of a permanent rule. @status:impl/done

@fact:NUMBER-PER-MODULE-AND-NEVER-RENUMBER Number per module (`PROP-001`, `FEAT-003`, …) and never renumber:
the number is part of the URI, and URIs are forever
([authoring rules §anchor-stability](authoring-rules.md#anchor-stability)). @status:impl/done

## What goes where {#what-goes-where}

| Fact | Home |
|------|------|
| @fact:ROW-HOME-ARCHITECTURE Architecture, stack, cross-module decision @status:impl/done | `spec/common/PROP-*` @status:impl/done |
| @fact:ROW-HOME-MODULE-CONTRACT One module's contract or invariant @status:impl/done | `spec/modules/<m>/PROP-*` @status:impl/done |
| @fact:ROW-HOME-FEATURE-SCOPE A feature's scope and acceptance criteria @status:impl/done | `spec/modules/<m>/FEAT-*` — or a campaign plan where the project runs slices as plans (`flow:campaign-plans`) @status:impl/done |
| @fact:ROW-HOME-NEXT-SESSION-STEP What the next session must do first @status:impl/done | `spec/WAL.md` @status:impl/done |
| @fact:ROW-HOME-SPEC-PROTOCOL How specs are updated, conflict rules @status:impl/done | protocol docs at the spec root @status:impl/done |
| @fact:ROW-HOME-BOOT-MINIMUM The session-boot minimum @status:impl/done | boot entry file (≤ 500 tokens) @status:impl/done |
| @fact:ROW-HOME-IMPLEMENTATION-DETAIL Implementation detail (*how*) @status:impl/done | code and doc comments — never the spec @status:impl/done |
| @fact:ROW-HOME-HUMAN-ONLY-NOTES Copy-paste snippets, human-only reminders @status:impl/done | `.human/` @status:impl/done |

@fact:CHOOSING-BETWEEN-THE-TWO-HOMES **Choosing between the two homes.** The row above names two
carriers for a feature's plan; which one carries it is a choice with
consequences, because it decides whether the plan stays addressable
after the work ships and whether it outlives the effort that produced
it. Make the choice explicitly and say it to whoever asked for the
work. @status:impl/done

@fact:A-FEATURE-DOCUMENT-CARRIES-A-FEATURE A **feature document** carries work that is a feature: it has a
name its user would recognise, it will be referred to again after it
ships, and it will not finish in one sitting. It is one file in the
feature's home, and it is addressable like any other specification
document — every anchored heading it carries becomes an address for
free, with nothing to register. @status:impl/done

@fact:A-CAMPAIGN-PLAN-CARRIES-A-PROGRAMME A **campaign plan** carries a programme rather than a feature:
many documents or many packages, a unit of work that is a batch
rather than a change, and a lifetime that ends when the campaign
closes. Its zone is disposable by design, so nothing that must
outlive the campaign belongs inside it. @status:impl/done

@fact:NEITHER-CARRIES-SMALL-WORK **Neither carries small work.** A three-line change planned into a
feature document is a document with more ceremony than content; the
current slice's checklist carries it, and that is the whole plan it
needs. @status:impl/done

@fact:THE-THRESHOLD-STAYS-QUALITATIVE The threshold between them stays qualitative on purpose. A line
count would be a number nobody measured, and a rule that files work
by size rather than by kind puts a rename and a subsystem in one
bucket whenever the count happens to agree. @status:impl/done

@fact:THE-CHOICE-BELONGS-TO-WHOEVER-ASKED-FOR-THE-WORK **The choice belongs to whoever asked for the work, not to the
agent doing it.** An agent that picks the carrier silently has
decided how long the work stays findable, which is not a decision it
was given: it proposes a carrier with its reason, and asks. @status:impl/done

@fact:DEFAULT-A-HOMELESS-FACT-INTO-THE-SPEC-TREE When a fact has no obvious home, default it into the spec tree
rather than a wiki, a gist, or a head: a teammate — human or agent —
who clones the repository must receive everything the project knows. @status:impl/done

## The `.human/` private buffer {#human-buffer}

@fact:HUMAN-BUFFER-HOLDS-TEXT-THE-AGENT-MUST-NEVER-SEE `.human/` holds text that must never enter the agent's context:
copy-paste correction snippets ("you are drifting, re-read the
spec"), negotiation notes, half-formed doubts. @status:impl/done

@fact:two-reasons-lead Two reasons to keep
it out: @status:impl/done

- @fact:REASON-TOKEN-COST **Token cost.** Every file the agent reads is context spent; a
  shortcuts file is pure overhead for any task. @status:spec/done
- @fact:REASON-REACTION-RISK **Reaction risk.** An agent that reads "you are drifting" mid-task
  may *respond* to it — reflecting on drift it has not committed,
  polluting the session with a correction nobody issued. @status:spec/done

@fact:PHYSICAL-INVISIBILITY-BEATS-LOGICAL-PROHIBITION The enforcement rule: **physical invisibility beats logical
prohibition.** @status:impl/done

@fact:DO-NOT-WRITE-A-NEVER-READ-LINE-IN-THE-BOOT-FILE Do not write "never read `.human/`" in the boot file
— that line itself costs tokens forever and invites the very
attention it forbids. @status:impl/done

@fact:LIST-THE-BUFFER-IN-THE-IGNORE-MECHANISM Instead, list `.human/` in the agent's ignore
mechanism (`.claudeignore`, `.aiexclude`, `.cursorignore` — whatever
the harness supports), so the directory does not exist as far as the
agent can see. @status:impl/done

@fact:a-firewall-beats-a-no-entry-sign A firewall beats a "no entry" sign. @status:spec/done

@fact:NO-IGNORE-FILE-MEANS-KEEP-IT-OUTSIDE-THE-TREE If the harness supports no ignore file, keep the buffer outside the
repository working tree entirely. @status:impl/done

@fact:the-principle-survives-the-mechanism The principle survives the
mechanism. @status:spec/done

## Naming maps to addressing {#naming}

@fact:layout-is-the-uri-scheme-lead The layout is the URI scheme made physical: @status:impl/done

- @fact:SEGMENT-MODULE-IS-THE-DIRECTORY the directory name under `spec/modules/` is the `<module>` segment; @status:impl/done
- @fact:SEGMENT-DOC-IS-THE-FILE-NAME the file name minus `.md` is the `<doc>` segment; @status:impl/done
- @fact:SEGMENT-FRAGMENT-IS-THE-ANCHOR the `{#anchor}` in the file is the fragment. @status:impl/done

@fact:A-URI-RESOLVES-WITH-ZERO-INDEX `spec://com.example.shop/PROP-001#verification.timeout` resolves with
zero index: `spec/modules/com.example.shop/PROP-001.md`, then find
`{#verification.timeout}`. @status:impl/done

@fact:KEEP-THE-MAPPING-ONE-TO-ONE Keep the mapping one-to-one — the moment
resolution needs a lookup table, every citation costs a search, and
the twenty-token correction stops being twenty tokens. @status:impl/done

@fact:REVERSE-DNS-WHEN-SPECS-MAY-BE-SHARED Use reverse-DNS module directory names when the specs could ever be
shared beyond this repository; short local names are fine when they
provably cannot. @status:impl/done

## Migrating an existing project {#migrating}

@fact:decisions-arrive-scattered Most projects arrive with decisions scattered across READMEs, wikis,
docstrings, and heads. @status:spec/done

@fact:delegate-the-inventory-lead Delegate the inventory: @status:impl/done

```
Inventory every Markdown file in this repository that states a
decision, requirement, or plan. For each: current path, what kind
of fact it holds (per the what-goes-where table in
spec/flows/addressable-specs/spec-tree-layout.md), its proposed
home in the spec tree, and which headings need {#anchor}s.
Output a migration table. Move nothing yet.
```

@fact:MIGRATE-IN-SMALL-REVIEWABLE-STEPS Review the table, then migrate in small, reviewable steps — the tree
is load-bearing, so it deserves the same care as a schema migration. @status:impl/done

## Summary {#summary}

- @fact:SUM-THREE-ZONES Three zones: shared state (`spec/`), artifacts (`src/`, `tests/`),
  private buffers (`.human/`). Their loss semantics differ; treat
  them accordingly. @status:impl/done
- @fact:SUM-PROP-IS-LAW-FEAT-IS-A-SLICE PROP is standing law, FEAT is a slice under it; lasting decisions
  migrate up. Numbers are part of URIs — never renumber. @status:impl/done
- @fact:SUM-EVERY-FACT-HAS-A-DESIGNATED-HOME Every fact has a designated home; when in doubt, it goes into the
  spec tree, where a fresh clone can find it. @status:impl/done
- @fact:SUM-HUMAN-BUFFER-IS-ENFORCED-BY-INVISIBILITY `.human/` is enforced by ignore-file invisibility, not by a rule
  the agent must read to obey. @status:impl/done
- @fact:SUM-NAMES-ARE-THE-URI-SEGMENTS Directory and file names *are* the URI segments — resolution must
  work with zero index. @status:impl/done
