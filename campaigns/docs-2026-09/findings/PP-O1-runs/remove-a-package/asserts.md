# asserts — remove-a-package

Executed verbatim from the page's `<assert>` elements, from the run's cwd.
`vibe` is a shim on the sandbox PATH pointing at the host debug binary;
`test` / `grep` are Git Bash builtins.

```
cwd: <scratch>/PP-O1/remove-a-package/work/hello-vibe
===== ASSERT: test ! -e vibevm/vibedeps/org.vibevm.world.wal
----- exit: 1
===== ASSERT: vibe check --quiet
vibe check: 0 errors, 0 warnings, 0 info
----- exit: 0
```
