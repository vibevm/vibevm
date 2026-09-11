# asserts — read-docs-locally

Executed verbatim from the page's `<assert>` elements, from the run's cwd.
`vibe` is a shim on the sandbox PATH pointing at the host debug binary;
`test` / `grep` are Git Bash builtins.

```
cwd: <scratch>/PP-O1/read-docs-locally/work
===== ASSERT: vibe cache list --quiet
store is empty (<scratch>\PP-O1\read-docs-locally\home\cache)
----- exit: 0
```
