# Flow: WAL Specspaces {#root}

This project may host **specspaces** — sub-projects nested in the
repository that are worked on as independent projects, each with its
own boot contract, WAL, and cold-resume file. The registry is
`SPECSPACES.md` at the host root; if that file is absent, no
specspaces exist and this snippet is inert.

## Recognising a specspace session {#recognise}

1. A resume or wind-down phrase carrying a specspace name —
   `RESUME SESSION <name>`, `END SESSION <name>`, or the project's
   language twins — targets that specspace, not the host project.
2. A session whose task clearly lives inside a registered specspace
   root follows that specspace's boot contract, even without the
   phrase. When in doubt, ask which project the session is for.

## Which project a bare phrase targets {#default}

A session phrase resolves to exactly **one** target — the host
project or a single specspace — by this order:

1. **Explicit target wins, always.** A phrase that names a specspace
   (`RESUME SESSION <name>`), or that names an explicit directory,
   targets that specspace or directory — regardless of any declared
   default. This is how the user forces restoration from an arbitrary
   specspace or directory. A name matching no registry row is
   surfaced, not guessed.
2. **Declared default.** A **bare** phrase (no name) uses the
   specspace named by the `default:` line of `SPECSPACES.md`, if one
   is declared.
3. **Host fallback.** With no name and no declared default, a bare
   phrase targets the **host project** — restore the host root's own
   WAL and cold-resume file, per the host contract's session-command
   sections.

A bare phrase therefore **never silently selects a specspace**. At the
host root, a bare `восстанови сессию` / `RESUME SESSION` restores the
**host** WAL — not a registered specspace such as `fractality`.
Targeting a specspace requires naming it (or declaring it the default).

## The boot-scoping law {#scoping}

A specspace session reads, in order:

1. the host's repo-wide non-negotiable rules (the section the host
   contract marks as binding for every commit),
2. the specspace's own boot contract (`CLAUDE.md` at the specspace
   root, or the file the registry names),
3. the specspace WAL,
4. the specspace cold-resume file (the WAL wins where they diverge),
5. any active plan the specspace WAL names.

It does **not** load the host's full boot sequence, the host WAL, or
host specs — unless the task explicitly crosses into the host
project, and then the session says so before touching host files.

## Session commands, scoped {#commands}

Wind-down and resume phrases carrying a specspace name operate on
that specspace's WAL and cold-resume file, and refresh the
specspace's one-line status in `SPECSPACES.md`. The host WAL is
updated only when host files actually changed in the session.
Resume remains report-then-wait: restore, verify state empirically,
report, stop.

Full protocol:
[`spec/flows/wal-specspaces/SPECSPACES-PROTOCOL.md`](../flows/wal-specspaces/SPECSPACES-PROTOCOL.md).
