# wal-workspaces

Non-central WALs for repositories that host more than one project.

`flow:org.vibevm.world/wal` gives a project session-durable state: a living
WAL checkpoint plus a cold-resume snapshot, wind-down and resume
phrases. This package extends that convention to **workspaces** —
sub-projects nested in a host repository but worked on as independent
projects. Each workspace carries its own boot contract, WAL, and
cold-resume file; a one-file registry (`WORKSPACES.md`) at the host
root names them; the session grammar gains an optional workspace name
(`RESUME SESSION <name>`, `END SESSION <name>`) that switches a
session into a workspace **without loading the host's full boot**.

What ships:

- `spec/boot/11-flow-wal-workspaces.md` — the boot snippet: how a
  session recognises workspace phrases and what it loads (and
  pointedly does not load) for a workspace session.
- `spec/flows/wal-workspaces/WORKSPACES-PROTOCOL.md` — the full
  protocol: the registry format, the scoped grammar, the five laws
  (boot scoping, state locality, one focus, host rules survive,
  package state stays out), lifecycle, and a re-derive prompt.

Requires `flow:org.vibevm.world/wal` (=0.2.0): workspaces reuse its
two-file model rather than redefining it.

License: UPL-1.0.
