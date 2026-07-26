# Spec tree layout {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** The reference directory layout for an
addressable spec tree, the PROP / FEAT distinction, the decision
table for where each kind of fact lives, and the `.human/` private
buffer with its physical-invisibility rule. @impl/done

##protocol-document-pointer The addressing scheme the
layout serves is defined in
[`ADDRESSABLE-SPECS-PROTOCOL.md`](ADDRESSABLE-SPECS-PROTOCOL.md). @impl/done

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

##SPEC-OR-SPECS-PICK-ONE `spec/` or `specs/` — either; pick one and never mix. @impl/done

##three-zones-lead The tree
divides into three zones with different loss semantics: @impl/done

- ##ZONE-SHARED-STATE **shared
  state** (`spec/` — losing it is a catastrophe, it is the only channel
  between the two processes), @impl/done
- ##ZONE-ARTIFACTS **artifacts** (`src/`, `tests/` — losing
  a file is an inconvenience; it can be regenerated from the spec), @impl/done
- ##ZONE-PRIVATE-BUFFERS and
  **private buffers** (`.human/` — one process's memory, invisible to
  the other). @impl/done

##HARNESS-ENTRY-FILES-ARE-THIN-REDIRECTS Harness entry files (`CLAUDE.md`, `AGENTS.md`, and whatever the next
tool demands) should be thin redirects into `spec/BOOT.md`. @impl/done

##one-boot-sequence-many-doors One boot
sequence, many doors — the alternative is N slowly diverging copies. @spec/done

## PROP vs FEAT {#prop-vs-feat}

|          | PROP | FEAT |
|----------|------|------|
| ##ROW-PROP-FEAT-HOLDS Holds @impl/done | standing decisions, contracts, protocol rules @impl/done | one feature slice: scope, plan, acceptance criteria @impl/done |
| ##ROW-PROP-FEAT-LIFETIME Lifetime @impl/done | in force until superseded — never deleted @impl/done | completes and freezes when the slice ships @impl/done |
| ##ROW-PROP-FEAT-CHANGES Changes @impl/done | rarely; every semantic change is a changelog line @impl/done | freely while active; frozen afterwards @impl/done |
| ##ROW-PROP-FEAT-CITED-BY Cited by @impl/done | code markers, commits, other specs — long-lived anchors @impl/done | the WAL and the commits of its own campaign @impl/done |

##PROP-IS-THE-LAW-FEAT-IS-A-PROJECT-UNDER-IT PROP is the law; FEAT is a project under that law. @impl/done

##A-LASTING-DECISION-MOVES-UP-INTO-A-PROP When a FEAT
uncovers a lasting decision, the decision moves *up* into a PROP and
the FEAT cites it — the slice document never becomes the permanent
home of a permanent rule. @impl/done

##NUMBER-PER-MODULE-AND-NEVER-RENUMBER Number per module (`PROP-001`, `FEAT-003`, …) and never renumber:
the number is part of the URI, and URIs are forever
([authoring rules §anchor-stability](authoring-rules.md#anchor-stability)). @impl/done

## What goes where {#what-goes-where}

| Fact | Home |
|------|------|
| ##ROW-HOME-ARCHITECTURE Architecture, stack, cross-module decision @impl/done | `spec/common/PROP-*` @impl/done |
| ##ROW-HOME-MODULE-CONTRACT One module's contract or invariant @impl/done | `spec/modules/<m>/PROP-*` @impl/done |
| ##ROW-HOME-FEATURE-SCOPE A feature's scope and acceptance criteria @impl/done | `spec/modules/<m>/FEAT-*` @impl/done |
| ##ROW-HOME-NEXT-SESSION-STEP What the next session must do first @impl/done | `spec/WAL.md` @impl/done |
| ##ROW-HOME-SPEC-PROTOCOL How specs are updated, conflict rules @impl/done | protocol docs at the spec root @impl/done |
| ##ROW-HOME-BOOT-MINIMUM The session-boot minimum @impl/done | boot entry file (≤ 500 tokens) @impl/done |
| ##ROW-HOME-IMPLEMENTATION-DETAIL Implementation detail (*how*) @impl/done | code and doc comments — never the spec @impl/done |
| ##ROW-HOME-HUMAN-ONLY-NOTES Copy-paste snippets, human-only reminders @impl/done | `.human/` @impl/done |

##DEFAULT-A-HOMELESS-FACT-INTO-THE-SPEC-TREE When a fact has no obvious home, default it into the spec tree
rather than a wiki, a gist, or a head: a teammate — human or agent —
who clones the repository must receive everything the project knows. @impl/done

## The `.human/` private buffer {#human-buffer}

##HUMAN-BUFFER-HOLDS-TEXT-THE-AGENT-MUST-NEVER-SEE `.human/` holds text that must never enter the agent's context:
copy-paste correction snippets ("you are drifting, re-read the
spec"), negotiation notes, half-formed doubts. @impl/done

##two-reasons-lead Two reasons to keep
it out: @impl/done

- ##REASON-TOKEN-COST **Token cost.** Every file the agent reads is context spent; a
  shortcuts file is pure overhead for any task. @spec/done
- ##REASON-REACTION-RISK **Reaction risk.** An agent that reads "you are drifting" mid-task
  may *respond* to it — reflecting on drift it has not committed,
  polluting the session with a correction nobody issued. @spec/done

##PHYSICAL-INVISIBILITY-BEATS-LOGICAL-PROHIBITION The enforcement rule: **physical invisibility beats logical
prohibition.** @impl/done

##DO-NOT-WRITE-A-NEVER-READ-LINE-IN-THE-BOOT-FILE Do not write "never read `.human/`" in the boot file
— that line itself costs tokens forever and invites the very
attention it forbids. @impl/done

##LIST-THE-BUFFER-IN-THE-IGNORE-MECHANISM Instead, list `.human/` in the agent's ignore
mechanism (`.claudeignore`, `.aiexclude`, `.cursorignore` — whatever
the harness supports), so the directory does not exist as far as the
agent can see. @impl/done

##a-firewall-beats-a-no-entry-sign A firewall beats a "no entry" sign. @spec/done

##NO-IGNORE-FILE-MEANS-KEEP-IT-OUTSIDE-THE-TREE If the harness supports no ignore file, keep the buffer outside the
repository working tree entirely. @impl/done

##the-principle-survives-the-mechanism The principle survives the
mechanism. @spec/done

## Naming maps to addressing {#naming}

##layout-is-the-uri-scheme-lead The layout is the URI scheme made physical: @impl/done

- ##SEGMENT-MODULE-IS-THE-DIRECTORY the directory name under `spec/modules/` is the `<module>` segment; @impl/done
- ##SEGMENT-DOC-IS-THE-FILE-NAME the file name minus `.md` is the `<doc>` segment; @impl/done
- ##SEGMENT-FRAGMENT-IS-THE-ANCHOR the `{#anchor}` in the file is the fragment. @impl/done

##A-URI-RESOLVES-WITH-ZERO-INDEX `spec://com.example.shop/PROP-001#verification.timeout` resolves with
zero index: `spec/modules/com.example.shop/PROP-001.md`, then find
`{#verification.timeout}`. @impl/done

##KEEP-THE-MAPPING-ONE-TO-ONE Keep the mapping one-to-one — the moment
resolution needs a lookup table, every citation costs a search, and
the twenty-token correction stops being twenty tokens. @impl/done

##REVERSE-DNS-WHEN-SPECS-MAY-BE-SHARED Use reverse-DNS module directory names when the specs could ever be
shared beyond this repository; short local names are fine when they
provably cannot. @impl/done

## Migrating an existing project {#migrating}

##decisions-arrive-scattered Most projects arrive with decisions scattered across READMEs, wikis,
docstrings, and heads. @spec/done

##delegate-the-inventory-lead Delegate the inventory: @impl/done

```
Inventory every Markdown file in this repository that states a
decision, requirement, or plan. For each: current path, what kind
of fact it holds (per the what-goes-where table in
spec/flows/addressable-specs/spec-tree-layout.md), its proposed
home in the spec tree, and which headings need {#anchor}s.
Output a migration table. Move nothing yet.
```

##MIGRATE-IN-SMALL-REVIEWABLE-STEPS Review the table, then migrate in small, reviewable steps — the tree
is load-bearing, so it deserves the same care as a schema migration. @impl/done

## Summary {#summary}

- ##SUM-THREE-ZONES Three zones: shared state (`spec/`), artifacts (`src/`, `tests/`),
  private buffers (`.human/`). Their loss semantics differ; treat
  them accordingly. @impl/done
- ##SUM-PROP-IS-LAW-FEAT-IS-A-SLICE PROP is standing law, FEAT is a slice under it; lasting decisions
  migrate up. Numbers are part of URIs — never renumber. @impl/done
- ##SUM-EVERY-FACT-HAS-A-DESIGNATED-HOME Every fact has a designated home; when in doubt, it goes into the
  spec tree, where a fresh clone can find it. @impl/done
- ##SUM-HUMAN-BUFFER-IS-ENFORCED-BY-INVISIBILITY `.human/` is enforced by ignore-file invisibility, not by a rule
  the agent must read to obey. @impl/done
- ##SUM-NAMES-ARE-THE-URI-SEGMENTS Directory and file names *are* the URI segments — resolution must
  work with zero index. @impl/done
