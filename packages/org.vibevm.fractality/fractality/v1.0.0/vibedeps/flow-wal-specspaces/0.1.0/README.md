# wal-specspaces

Non-central WALs for repositories that host more than one project.

`flow:org.vibevm.world/wal` gives a project session-durable state: a living
WAL checkpoint plus a cold-resume snapshot, wind-down and resume
phrases. This package extends that convention to **specspaces** —
sub-projects nested in a host repository but worked on as independent
projects. Each specspace carries its own boot contract, WAL, and
cold-resume file; a one-file registry (`SPECSPACES.md`) at the host
root names them; the session grammar gains an optional specspace name
(`RESUME SESSION <name>`, `END SESSION <name>`) that switches a
session into a specspace **without loading the host's full boot**.

A **bare** phrase (no name) never wanders into a specspace on its own:
it targets the `default` specspace declared in `SPECSPACES.md` if one
is set, and otherwise the host project itself.

What ships:

- `spec/boot/11-flow-wal-specspaces.md` — the boot snippet: how a
  session recognises specspace phrases, which project a bare phrase
  targets, and what it loads (and pointedly does not load) for a
  specspace session.
- `spec/flows/wal-specspaces/SPECSPACES-PROTOCOL.md` — the full
  protocol: the registry format (with the optional default), target
  resolution, the scoped grammar, the five laws (boot scoping, state
  locality, one focus, host rules survive, package state stays out),
  lifecycle, and a re-derive prompt.

Requires `flow:org.vibevm.world/wal` (=0.2.0): specspaces reuse its
two-file model rather than redefining it.

License: UPL-1.0.
