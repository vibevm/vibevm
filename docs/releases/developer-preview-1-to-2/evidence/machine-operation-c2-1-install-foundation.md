# M-16-C.2.1 protected empty-world Install, still open

The internal `Phase::Install` path is restricted to one root project with no
package/git requirements, provider declarations, hooks, build/deploy targets,
registry overrides or LLM configuration; no prior lifecycle state, trace,
force or install arguments are admitted. Its registry environment is an
in-memory default, with no global seeding. The caller selects a pure run ID
and exact pre-install plan under one lease, reserves it through C.1, writes
and rereads the matching lifecycle header, and marks journal-ready. It then
rechecks the selected manifest/lock bytes and publishes an immutable
`install-entered` record before the ordinary install engine can regenerate
boot artifacts. The final generated report and per-run marker are retained
and reread before the response.

At `baa48dae6`, focused adapter tests passed 11/11, install-stage tests 4/4,
and the affected ordinary package-free install case 1/1. In the effectful
case, the first protected call wrote `vibevm/vibespecs/boot/INDEX.md`; after a
test replaced it with a sentinel, the same-key retry returned the exact
retained result and left the sentinel untouched. Removing the marker made
the retry return recovery-required, again without a second boot write.
Changed manifest/plan, foreign header, declared package and nonempty lock
dependencies were refused. No public machine endpoint was enabled.

This does not accept M-16-C.2.1. General package installation may write during
planning, cache fetch, slot materialization, hooks and lockfile publication;
those effects need their own owner journal and partial recovery before they
can be admitted. A later command that legitimately rewrites its manifest or
lock also needs retained pre-effect plan lookup for same-key replay rather
than recomputing a different plan from its own output. Member runs and
handler execution remain unavailable on this adapter. The stage says an
effect may have happened; it does not claim physical CAS or detailed
partial-install truth.
