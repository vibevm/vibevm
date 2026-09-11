# asserts — scrape

Executed verbatim from the page's `<assert>` elements, from the run's cwd.
`vibe` is a shim on the sandbox PATH pointing at the host debug binary;
`test` / `grep` are Git Bash builtins.

```
cwd: <scratch>/PP-O1/scrape/work/hello-vibe
===== ASSERT: test ! -e ../product-clean/vibe.toml
----- exit: 0
===== ASSERT: test ! -e ../product-clean/vibevm
----- exit: 0
```
