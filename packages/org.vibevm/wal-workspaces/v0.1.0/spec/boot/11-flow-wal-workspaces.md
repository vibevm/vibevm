# Flow: WAL Workspaces {#root}

This project may host **workspaces** — sub-projects nested in the
repository that are worked on as independent projects, each with its
own boot contract, WAL, and cold-resume file. The registry is
`WORKSPACES.md` at the host root; if that file is absent, no
workspaces exist and this snippet is inert.

## Recognising a workspace session {#recognise}

1. A resume or wind-down phrase carrying a workspace name —
   `RESUME SESSION <name>`, `END SESSION <name>`, or the project's
   language twins — targets that workspace, not the host project.
2. A session whose task clearly lives inside a registered workspace
   root follows that workspace's boot contract, even without the
   phrase. When in doubt, ask which project the session is for.

## The boot-scoping law {#scoping}

A workspace session reads, in order:

1. the host's repo-wide non-negotiable rules (the section the host
   contract marks as binding for every commit),
2. the workspace's own boot contract (`CLAUDE.md` at the workspace
   root, or the file the registry names),
3. the workspace WAL,
4. the workspace cold-resume file (the WAL wins where they diverge).

It does **not** load the host's full boot sequence, the host WAL, or
host specs — unless the task explicitly crosses into the host
project, and then the session says so before touching host files.

## Session commands, scoped {#commands}

Wind-down and resume phrases carrying a workspace name operate on
that workspace's WAL and cold-resume file, and refresh the
workspace's one-line status in `WORKSPACES.md`. The host WAL is
updated only when host files actually changed in the session.
Resume remains report-then-wait: restore, verify state empirically,
report, stop.

Full protocol:
[`spec/flows/wal-workspaces/WORKSPACES-PROTOCOL.md`](../flows/wal-workspaces/WORKSPACES-PROTOCOL.md).
