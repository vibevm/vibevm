# asserts — work-offline

Executed verbatim from the page's `<assert>` elements, from the run's cwd.
`vibe` is a shim on the sandbox PATH pointing at the host debug binary;
`test` / `grep` are Git Bash builtins.

```
cwd: <scratch>/PP-O1/work-offline/work/hello-vibe
===== ASSERT: vibe cache check
Integrity sweep of the machine store (<scratch>\PP-O1\work-offline\home\cache)

2 ok, 0 mismatched, 0 unrecorded.
----- exit: 0
===== ASSERT: vibe install --offline --assume-yes
vibe.lock is fresh — skipping resolution
vibe install: vibe.lock unchanged — nothing to re-resolve (1 node up to date)
----- exit: 0
```
