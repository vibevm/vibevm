# asserts — write-a-feat-or-stack

Executed verbatim from the page's `<assert>` elements, from the run's cwd.
`vibe` is a shim on the sandbox PATH pointing at the host debug binary;
`test` / `grep` are Git Bash builtins.

```
cwd: <scratch>/PP-O1/write-a-feat-or-stack/work
===== ASSERT: vibe check --path vibevm/vibepacks/org.acme/welcome-page/v0.1.0 --quiet
vibe check: 0 errors, 0 warnings, 0 info
----- exit: 0
===== ASSERT: vibe check --path vibevm/vibepacks/org.acme/static-site/v0.1.0 --quiet
vibe check: 0 errors, 0 warnings, 0 info
----- exit: 0
```
