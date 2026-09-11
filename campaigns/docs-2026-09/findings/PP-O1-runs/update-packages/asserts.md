# asserts — update-packages

Executed verbatim from the page's `<assert>` elements, from the run's cwd.
`vibe` is a shim on the sandbox PATH pointing at the host debug binary;
`test` / `grep` are Git Bash builtins.

```
cwd: <scratch>/PP-O1/update-packages/work/hello-vibe
===== ASSERT: vibe check --quiet
vibe check: 0 errors, 0 warnings, 0 info
----- exit: 0
===== ASSERT: vibe list --quiet
flow:wal@1.0.0
----- exit: 0
```
