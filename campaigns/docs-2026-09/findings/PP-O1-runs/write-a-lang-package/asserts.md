# asserts — write-a-lang-package

Executed verbatim from the page's `<assert>` elements, from the run's cwd.
`vibe` is a shim on the sandbox PATH pointing at the host debug binary;
`test` / `grep` are Git Bash builtins.

```
cwd: <scratch>/PP-O1/write-a-lang-package/work
===== ASSERT: vibe check --path vibevm/vibepacks/org.acme/sql-style/v0.1.0 --quiet
vibe check: 0 errors, 0 warnings, 0 info
----- exit: 0
===== ASSERT: grep -q "kind = \"lang\"" vibevm/vibepacks/org.acme/sql-style/v0.1.0/vibe.toml
----- exit: 0
```
