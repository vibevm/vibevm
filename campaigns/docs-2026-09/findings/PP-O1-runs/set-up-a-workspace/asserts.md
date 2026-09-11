# asserts — set-up-a-workspace

Executed verbatim from the page's `<assert>` elements, from the run's cwd.
`vibe` is a shim on the sandbox PATH pointing at the host debug binary;
`test` / `grep` are Git Bash builtins.

```
cwd: <scratch>/PP-O1/set-up-a-workspace/work/hello-vibe
===== ASSERT: test -f packages/notes-flow/vibe.toml
----- exit: 0
===== ASSERT: test -f packages/notes-docs/vibe.toml
----- exit: 0
===== ASSERT: test ! -e packages/notes-flow/vibe.lock
----- exit: 1
===== ASSERT: vibe check --quiet
vibe check: 0 errors, 0 warnings, 0 info
----- exit: 0
```
