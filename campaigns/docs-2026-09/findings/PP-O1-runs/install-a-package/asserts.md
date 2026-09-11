# asserts — install-a-package

Executed verbatim from the page's `<assert>` elements, from the run's cwd.
`vibe` is a shim on the sandbox PATH pointing at the host debug binary;
`test` / `grep` are Git Bash builtins.

```
cwd: <scratch>/PP-O1/install-a-package/work/hello-vibe
===== ASSERT: grep -q "org.vibevm.world/wal" vibe.lock
----- exit: 0
===== ASSERT: vibe check --quiet
vibe check: 0 errors, 0 warnings, 0 info
----- exit: 0
```
