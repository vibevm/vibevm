# wal-specspaces {#root}

<status stage="doc" state="done" audience="user"/>

@fact:NON-CENTRAL-WALS-FOR-MULTI-PROJECT-REPOSITORIES Non-central WALs for repositories that host more than one project. @status:impl/done

@fact:THE-WAL-FLOW-GIVES-A-PROJECT-SESSION-DURABLE-STATE `flow:org.vibevm.world/wal` gives a project session-durable state: a living
WAL checkpoint plus a cold-resume snapshot, wind-down and resume
phrases. @status:impl/done

@fact:THIS-PACKAGE-EXTENDS-THAT-CONVENTION-TO-SPECSPACES This package extends that convention to **specspaces** —
sub-projects nested in a host repository but worked on as independent
projects. @status:impl/done

@fact:EACH-SPECSPACE-CARRIES-ITS-OWN-STATE-REGISTRY-AND-GRAMMAR Each specspace carries its own boot contract, WAL, and
cold-resume file; a one-file registry (`SPECSPACES.md`) at the host
root names them; the session grammar gains an optional specspace name
(`RESUME SESSION <name>`, `END SESSION <name>`) that switches a
session into a specspace **without loading the host's full boot**. @status:impl/done

@fact:A-BARE-PHRASE-NEVER-WANDERS-INTO-A-SPECSPACE A **bare** phrase (no name) never wanders into a specspace on its own:
it targets the `default` specspace declared in `SPECSPACES.md` if one
is set, and otherwise the host project itself. @status:impl/done

@fact:package-contents-lead What ships: @status:impl/done

- @fact:CONTENT-THE-BOOT-SNIPPET `spec/boot/11-flow-wal-specspaces.xml` — the boot snippet: how a
  session recognises specspace phrases, which project a bare phrase
  targets, and what it loads (and pointedly does not load) for a
  specspace session. @status:impl/done
- @fact:CONTENT-THE-FULL-PROTOCOL `spec/flows/wal-specspaces/SPECSPACES-PROTOCOL.xml` — the full
  protocol: the registry format (with the optional default), target
  resolution, the scoped grammar, the five laws (boot scoping, state
  locality, one focus, host rules survive, package state stays out),
  lifecycle, and a re-derive prompt. @status:impl/done

@fact:REQUIRES-THE-WAL-FLOW-AT-AN-EXACT-PIN Requires `flow:org.vibevm.world/wal` (=1.0.0): specspaces reuse its
two-file model rather than redefining it. @status:impl/done

@fact:license-line License: UPL-1.0. @status:impl/done

