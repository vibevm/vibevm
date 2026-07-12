# Spec tree layout {#root}

**Scope of this document.** The reference directory layout for an
addressable spec tree, the PROP / FEAT distinction, the decision
table for where each kind of fact lives, and the `.human/` private
buffer with its physical-invisibility rule. The addressing scheme the
layout serves is defined in
[`ADDRESSABLE-SPECS-PROTOCOL.md`](ADDRESSABLE-SPECS-PROTOCOL.md).

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

`spec/` or `specs/` — either; pick one and never mix. The tree
divides into three zones with different loss semantics: **shared
state** (`spec/` — losing it is a catastrophe, it is the only channel
between the two processes), **artifacts** (`src/`, `tests/` — losing
a file is an inconvenience; it can be regenerated from the spec), and
**private buffers** (`.human/` — one process's memory, invisible to
the other).

Harness entry files (`CLAUDE.md`, `AGENTS.md`, and whatever the next
tool demands) should be thin redirects into `spec/BOOT.md`. One boot
sequence, many doors — the alternative is N slowly diverging copies.

## PROP vs FEAT {#prop-vs-feat}

|          | PROP | FEAT |
|----------|------|------|
| Holds    | standing decisions, contracts, protocol rules | one feature slice: scope, plan, acceptance criteria |
| Lifetime | in force until superseded — never deleted | completes and freezes when the slice ships |
| Changes  | rarely; every semantic change is a changelog line | freely while active; frozen afterwards |
| Cited by | code markers, commits, other specs — long-lived anchors | the WAL and the commits of its own campaign |

PROP is the law; FEAT is a project under that law. When a FEAT
uncovers a lasting decision, the decision moves *up* into a PROP and
the FEAT cites it — the slice document never becomes the permanent
home of a permanent rule.

Number per module (`PROP-001`, `FEAT-003`, …) and never renumber:
the number is part of the URI, and URIs are forever
([authoring rules §anchor-stability](authoring-rules.md#anchor-stability)).

## What goes where {#what-goes-where}

| Fact | Home |
|------|------|
| Architecture, stack, cross-module decision | `spec/common/PROP-*` |
| One module's contract or invariant | `spec/modules/<m>/PROP-*` |
| A feature's scope and acceptance criteria | `spec/modules/<m>/FEAT-*` |
| What the next session must do first | `spec/WAL.md` |
| How specs are updated, conflict rules | protocol docs at the spec root |
| The session-boot minimum | boot entry file (≤ 500 tokens) |
| Implementation detail (*how*) | code and doc comments — never the spec |
| Copy-paste snippets, human-only reminders | `.human/` |

When a fact has no obvious home, default it into the spec tree
rather than a wiki, a gist, or a head: a teammate — human or agent —
who clones the repository must receive everything the project knows.

## The `.human/` private buffer {#human-buffer}

`.human/` holds text that must never enter the agent's context:
copy-paste correction snippets ("you are drifting, re-read the
spec"), negotiation notes, half-formed doubts. Two reasons to keep
it out:

- **Token cost.** Every file the agent reads is context spent; a
  shortcuts file is pure overhead for any task.
- **Reaction risk.** An agent that reads "you are drifting" mid-task
  may *respond* to it — reflecting on drift it has not committed,
  polluting the session with a correction nobody issued.

The enforcement rule: **physical invisibility beats logical
prohibition.** Do not write "never read `.human/`" in the boot file
— that line itself costs tokens forever and invites the very
attention it forbids. Instead, list `.human/` in the agent's ignore
mechanism (`.claudeignore`, `.aiexclude`, `.cursorignore` — whatever
the harness supports), so the directory does not exist as far as the
agent can see. A firewall beats a "no entry" sign.

If the harness supports no ignore file, keep the buffer outside the
repository working tree entirely. The principle survives the
mechanism.

## Naming maps to addressing {#naming}

The layout is the URI scheme made physical:

- the directory name under `spec/modules/` is the `<module>` segment;
- the file name minus `.md` is the `<doc>` segment;
- the `{#anchor}` in the file is the fragment.

`spec://com.example.shop/PROP-001#verification.timeout` resolves with
zero index: `spec/modules/com.example.shop/PROP-001.md`, then find
`{#verification.timeout}`. Keep the mapping one-to-one — the moment
resolution needs a lookup table, every citation costs a search, and
the twenty-token correction stops being twenty tokens.

Use reverse-DNS module directory names when the specs could ever be
shared beyond this repository; short local names are fine when they
provably cannot.

## Migrating an existing project {#migrating}

Most projects arrive with decisions scattered across READMEs, wikis,
docstrings, and heads. Delegate the inventory:

```
Inventory every Markdown file in this repository that states a
decision, requirement, or plan. For each: current path, what kind
of fact it holds (per the what-goes-where table in
spec/flows/addressable-specs/spec-tree-layout.md), its proposed
home in the spec tree, and which headings need {#anchor}s.
Output a migration table. Move nothing yet.
```

Review the table, then migrate in small, reviewable steps — the tree
is load-bearing, so it deserves the same care as a schema migration.

## Summary {#summary}

- Three zones: shared state (`spec/`), artifacts (`src/`, `tests/`),
  private buffers (`.human/`). Their loss semantics differ; treat
  them accordingly.
- PROP is standing law, FEAT is a slice under it; lasting decisions
  migrate up. Numbers are part of URIs — never renumber.
- Every fact has a designated home; when in doubt, it goes into the
  spec tree, where a fresh clone can find it.
- `.human/` is enforced by ignore-file invisibility, not by a rule
  the agent must read to obey.
- Directory and file names *are* the URI segments — resolution must
  work with zero index.
