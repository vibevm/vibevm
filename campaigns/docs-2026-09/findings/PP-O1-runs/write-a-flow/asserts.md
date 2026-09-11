# asserts — write-a-flow

Executed verbatim from the page's `<assert>` elements, from the run's cwd.
`vibe` is a shim on the sandbox PATH pointing at the host debug binary;
`test` / `grep` are Git Bash builtins.

```
cwd: <scratch>/PP-O1/write-a-flow/work
===== ASSERT: vibe check --path packages/review-notes --quiet
vibe check: 0 errors, 0 warnings, 0 info
----- exit: 0
===== ASSERT: test -f packages/review-notes/vibevm/vibespecs/boot/review-notes.xml
----- exit: 0
```
