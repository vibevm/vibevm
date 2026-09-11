# asserts — first-project

Executed verbatim from the page's `<assert>` elements, from the run's cwd.
`vibe` is a shim on the sandbox PATH pointing at the host debug binary;
`test` / `grep` are Git Bash builtins.

```
cwd: <scratch>/PP-O1/first-project/work
===== ASSERT: test -f hello-vibe/vibe.toml
----- exit: 0
===== ASSERT: test -f hello-vibe/vibe.lock
----- exit: 0
===== ASSERT: grep -q "org.vibevm.world/wal" hello-vibe/vibe.lock
----- exit: 0
===== ASSERT: vibe check --path hello-vibe --quiet
vibe check: 0 errors, 0 warnings, 0 info
----- exit: 0
```
