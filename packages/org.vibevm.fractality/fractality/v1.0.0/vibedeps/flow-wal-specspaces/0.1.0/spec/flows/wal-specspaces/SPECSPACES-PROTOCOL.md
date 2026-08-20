# The Specspaces Protocol {#root}

**Scope of this document.** This file defines *what* a specspace is,
the registry file that names them, how a session phrase resolves to a
target, the scoped session grammar, and the five laws that keep a
specspace's state from bleeding into its host. The per-specspace WAL
mechanics are not redefined here — each specspace runs the two-file
model owned by `flow:org.vibevm.world/wal`.

## What a specspace is {#what}

A specspace is a sub-project that lives inside a host repository but
is **worked on as an independent project**: its own boot contract,
its own WAL, its own cold-resume file, its own plans and specs, its
own definition of a green floor. The host carries it (one git
history, one set of repo-wide rules), but a session working on the
specspace neither loads nor scans the host's project context.

A specspace is its own installable, publishable thing in the general
case; that a host repository happens to author one under `packages/`
(as vibevm does with `fractality`) is incidental. People install
specspaces as ordinary packages, and two of them coinciding inside
one host means nothing in general.

The problem this solves: a central WAL describes one project. The
moment a repository hosts a second, independently-evolving effort,
sessions face a bad choice — boot the whole host corpus to work on a
corner of it, or work blind. Specspaces make the second effort a
first-class project with first-class session continuity, at the cost
of one registry file and one grammar extension.

## The registry: `SPECSPACES.md` {#registry}

One file at the host root names every specspace. An optional
`default:` line may precede the table; then one table row per
specspace:

```markdown
default: host

| name | root | wal | continue | status |
|---|---|---|---|---|
| fractality | packages/org.vibevm.fractality/ | WAL.md | CONTINUE.md | 2026-07-09 — ignition PLANNED; next: Phase 0 |
```

- **`default:`** (optional, above the table) — which target a **bare**
  session phrase resolves to (see [target resolution](#resolve)).
  `default: host` (or omitting the line entirely) makes a bare phrase
  target the host project; `default: <name>` makes a bare phrase
  target that specspace. Omit it unless the repository's primary work
  really lives in one specspace.
- **name** — the word used in session phrases. Short, unique,
  lowercase.
- **root** — the specspace root, relative to the host root. The
  specspace's `CLAUDE.md` (or equivalent boot contract) lives here.
- **wal**, **continue** — paths relative to root. Defaults are
  `spec/WAL.md` and `CONTINUE.md` per the wal flow; a specspace
  without a spec tree at its root may keep both flat at the root, as
  the example does. The registry entry is the truth.
- **status** — one line, refreshed at every specspace wind-down:
  date, campaign/phase state, the next step. A pointer for the
  *host's* readers; never the specspace's canonical state.

## Target resolution {#resolve}

A resume or wind-down phrase resolves to exactly **one** target — the
host project or a single specspace — by this order:

1. **Explicit target wins, always.** A phrase that names a specspace
   (`RESUME SESSION <name>`), or that names an explicit directory,
   targets that specspace or directory — **regardless of any declared
   default or prior context**. This is the user's always-available
   escape hatch: an explicit command forces restoration from an
   arbitrary specspace or directory. A name matching no registry row
   is surfaced, not guessed.
2. **Declared default.** A **bare** phrase (no name) uses the
   specspace named by the `default:` line of `SPECSPACES.md`, when one
   is declared.
3. **Host fallback.** With no name and no declared default, a bare
   phrase targets the **host project** — restore the host root's own
   WAL and cold-resume file, per the host contract's session-command
   sections.

A bare phrase therefore **never silently selects a specspace**. At the
host root, a bare `RESUME SESSION` / `ВОССТАНОВИ СЕССИЮ` restores the
**host** WAL — not a registered specspace such as `fractality`. This
is the whole point of rule 3: a registry with rows in it must not
tempt a session into resuming the wrong project.

## The session grammar {#grammar}

The wal flow's wind-down and resume phrases gain an optional
specspace name. Recognise the intent, not the exact wording:

| Intent | English | Russian |
|---|---|---|
| resume into specspace | `RESUME SESSION <name>`, `RESTORE CONTEXT <name>` | `ВОССТАНОВИ СЕССИЮ <name>`, `ПРОДОЛЖАЕМ <name>` |
| wind down specspace | `END SESSION <name>`, `WRAP UP <name>` | `ЗАВЕРШИ СЕССИЮ <name>`, `ФИКСИРУЕМ <name>` |

The name is optional; [target resolution](#resolve) decides what a
phrase with no name means.

**Resume into a specspace** (report-then-wait, as in the wal flow):
read the host's repo-wide rules, the specspace boot contract, the
specspace WAL, the specspace cold-resume file; verify the tree
empirically (branch, sync, working tree, recent commits touching the
specspace); report; stop and wait for direction.

**Wind down a specspace:** rewrite the specspace WAL; overwrite the
specspace cold-resume file wholesale; refresh the specspace's status
line in `SPECSPACES.md`; commit per the host's commit rules; update
the host WAL **only** if host files changed too.

## The five laws {#laws}

1. **Boot scoping.** A specspace session loads the host's repo-wide
   rules and the specspace's own files — nothing else from the host.
   Crossing into host files mid-session is legal but announced.
2. **State locality.** A specspace's canonical state lives in the
   specspace (its WAL). The registry status line is a pointer, not
   state; the host WAL never carries specspace detail beyond "the
   specspace exists; see its WAL".
3. **One session, one focus.** A session serves the host or one
   specspace. Work for two projects in one session splits into
   commits per project and updates each project's WAL — and is the
   exception, said out loud, not the habit.
4. **Host rules survive.** Repo-wide non-negotiables (authorship,
   commit conventions, secrecy rules) bind specspace sessions in
   full. A specspace may add rules; it may not subtract the host's.
5. **Package state stays out.** WAL, cold-resume, and registry files
   are project state. No installable package may create, overwrite,
   or delete them on install or uninstall — the same law the wal
   flow states for its two files.

## Lifecycle {#lifecycle}

- **Register:** create the specspace root with its boot contract,
  WAL, and cold-resume file; add the registry row; mention the
  registry in the host boot contract so sessions recognise the
  grammar. First wind-down validates the loop.
- **Retire:** the specspace graduates to its own repository (its
  files move wholesale; its WAL goes with it) or closes (final
  status line says so; the row moves to a "retired" section — the
  name stays reserved so old phrases fail loudly, not silently).
- **Nesting:** one level. A specspace hosting its own specspaces is
  a sign it wants to be a repository.

## Re-derive for your project {#re-derive}

Run this prompt once to adapt the protocol to a concrete host:

```
Read SPECSPACES-PROTOCOL.md. Adapt it to this repository:
1. Create SPECSPACES.md at the host root (empty table if no
   specspaces exist yet). Add a `default:` line only if a bare
   session phrase should target a specspace instead of the host.
2. Add a short "Specspaces" section to the host boot contract:
   the grammar, target resolution (bare → default else host), the
   boot-scoping law, and a pointer to the registry.
3. If a sub-project already behaves like a specspace, register it:
   boot contract, WAL, cold-resume file, registry row.
4. Record which host sections are "repo-wide rules" that specspace
   sessions must still load — name them explicitly.
Do not move any existing project state while adapting.
```

## Summary {#summary}

- A specspace is a sub-project with first-class session continuity:
  own boot contract, own WAL, own cold-resume file.
- `SPECSPACES.md` at the host root is the registry; an optional
  `default:` line sets what a bare phrase targets; its status column
  is a pointer, never canonical state.
- Target resolution: an explicit name/directory always wins; a bare
  phrase takes the declared default, else the host — never a
  specspace by accident.
- Session phrases gain an optional specspace name; resume stays
  report-then-wait; wind-down operates on the specspace's files.
- Five laws: boot scoping, state locality, one focus, host rules
  survive, package state stays out.
- One nesting level; a specspace that outgrows the host graduates
  to its own repository.
