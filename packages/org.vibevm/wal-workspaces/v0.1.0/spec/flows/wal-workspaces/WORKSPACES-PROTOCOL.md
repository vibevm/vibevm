# The Workspaces Protocol {#root}

**Scope of this document.** This file defines *what* a workspace is,
the registry file that names them, the scoped session grammar, and
the five laws that keep a workspace's state from bleeding into its
host. The per-workspace WAL mechanics are not redefined here — each
workspace runs the two-file model owned by `flow:org.vibevm/wal`.

## What a workspace is {#what}

A workspace is a sub-project that lives inside a host repository but
is **worked on as an independent project**: its own boot contract,
its own WAL, its own cold-resume file, its own plans and specs, its
own definition of a green floor. The host carries it (one git
history, one set of repo-wide rules), but a session working on the
workspace neither loads nor scans the host's project context.

The problem this solves: a central WAL describes one project. The
moment a repository hosts a second, independently-evolving effort,
sessions face a bad choice — boot the whole host corpus to work on a
corner of it, or work blind. Workspaces make the second effort a
first-class project with first-class session continuity, at the cost
of one registry file and one grammar extension.

## The registry: `WORKSPACES.md` {#registry}

One file at the host root names every workspace. One table row per
workspace:

```markdown
| name | root | wal | continue | status |
|---|---|---|---|---|
| fractality | packages/org.vibevm.fractality/ | WAL.md | CONTINUE.md | 2026-07-09 — ignition PLANNED; next: Phase 0 |
```

- **name** — the word used in session phrases. Short, unique,
  lowercase.
- **root** — the workspace root, relative to the host root. The
  workspace's `CLAUDE.md` (or equivalent boot contract) lives here.
- **wal**, **continue** — paths relative to root. Defaults are
  `spec/WAL.md` and `CONTINUE.md` per the wal flow; a workspace
  without a spec tree at its root may keep both flat at the root, as
  the example does. The registry entry is the truth.
- **status** — one line, refreshed at every workspace wind-down:
  date, campaign/phase state, the next step. A pointer for the
  *host's* readers; never the workspace's canonical state.

## The session grammar {#grammar}

The wal flow's wind-down and resume phrases gain an optional
workspace name. Recognise the intent, not the exact wording:

| Intent | English | Russian |
|---|---|---|
| resume into workspace | `RESUME SESSION <name>`, `RESTORE CONTEXT <name>` | `ВОССТАНОВИ СЕССИЮ <name>`, `ПРОДОЛЖАЕМ <name>` |
| wind down workspace | `END SESSION <name>`, `WRAP UP <name>` | `ЗАВЕРШИ СЕССИЮ <name>`, `ФИКСИРУЕМ <name>` |

A phrase **without** a name keeps its host meaning unchanged. A name
that matches no registry row is surfaced, not guessed.

**Resume into a workspace** (report-then-wait, as in the wal flow):
read the host's repo-wide rules, the workspace boot contract, the
workspace WAL, the workspace cold-resume file; verify the tree
empirically (branch, sync, working tree, recent commits touching the
workspace); report; stop and wait for direction.

**Wind down a workspace:** rewrite the workspace WAL; overwrite the
workspace cold-resume file wholesale; refresh the workspace's status
line in `WORKSPACES.md`; commit per the host's commit rules; update
the host WAL **only** if host files changed too.

## The five laws {#laws}

1. **Boot scoping.** A workspace session loads the host's repo-wide
   rules and the workspace's own files — nothing else from the host.
   Crossing into host files mid-session is legal but announced.
2. **State locality.** A workspace's canonical state lives in the
   workspace (its WAL). The registry status line is a pointer, not
   state; the host WAL never carries workspace detail beyond "the
   workspace exists; see its WAL".
3. **One session, one focus.** A session serves the host or one
   workspace. Work for two projects in one session splits into
   commits per project and updates each project's WAL — and is the
   exception, said out loud, not the habit.
4. **Host rules survive.** Repo-wide non-negotiables (authorship,
   commit conventions, secrecy rules) bind workspace sessions in
   full. A workspace may add rules; it may not subtract the host's.
5. **Package state stays out.** WAL, cold-resume, and registry files
   are project state. No installable package may create, overwrite,
   or delete them on install or uninstall — the same law the wal
   flow states for its two files.

## Lifecycle {#lifecycle}

- **Register:** create the workspace root with its boot contract,
  WAL, and cold-resume file; add the registry row; mention the
  registry in the host boot contract so sessions recognise the
  grammar. First wind-down validates the loop.
- **Retire:** the workspace graduates to its own repository (its
  files move wholesale; its WAL goes with it) or closes (final
  status line says so; the row moves to a "retired" section — the
  name stays reserved so old phrases fail loudly, not silently).
- **Nesting:** one level. A workspace hosting its own workspaces is
  a sign it wants to be a repository.

## Re-derive for your project {#re-derive}

Run this prompt once to adapt the protocol to a concrete host:

```
Read WORKSPACES-PROTOCOL.md. Adapt it to this repository:
1. Create WORKSPACES.md at the host root (empty table if no
   workspaces exist yet).
2. Add a short "Workspaces" section to the host boot contract:
   the grammar, the boot-scoping law, and a pointer to the registry.
3. If a sub-project already behaves like a workspace, register it:
   boot contract, WAL, cold-resume file, registry row.
4. Record which host sections are "repo-wide rules" that workspace
   sessions must still load — name them explicitly.
Do not move any existing project state while adapting.
```

## Summary {#summary}

- A workspace is a sub-project with first-class session continuity:
  own boot contract, own WAL, own cold-resume file.
- `WORKSPACES.md` at the host root is the registry; its status
  column is a pointer, never canonical state.
- Session phrases gain an optional workspace name; resume stays
  report-then-wait; wind-down operates on the workspace's files.
- Five laws: boot scoping, state locality, one focus, host rules
  survive, package state stays out.
- One nesting level; a workspace that outgrows the host graduates
  to its own repository.
