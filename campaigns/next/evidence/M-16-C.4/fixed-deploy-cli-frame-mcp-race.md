# M-16-C.4 fixed Deploy CLI frame and MCP process race

An isolated Linux fixture starts the framed CLI session and a separate
registered MCP server against one explicitly issued fixed Deploy host. The
MCP process first inspects the absent key, then both processes submit the
same key from a barrier. A cold MCP reattach resolves one completed run. The
C1 index retains one namespace and one key pointing to that run; one host
grant and the deployed launcher exist.

Either racing submit may return the completed owner reply. The losing process
may instead receive `submission_refused` for `Lease(Busy)` with an unavailable
effect observation and no known run ID. That refusal is not a no-effects
claim; cold reattach supplies the retained terminal identity. The test
rejects other refusal codes and observations.

Exact checks on 2026-10-05: the two `frame_cross_surface::` cases passed 2/2;
the new race case passed five additional isolated runs 1/1 each. File-scoped
`rustfmt --check` and `git diff --check` exited 0. The fixture redirects
`HOME` and `VIBE_SETTINGS` to temporary directories and uses no public host.
