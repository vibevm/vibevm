# asserts — translate-documentation

Executed verbatim from the page's `<assert>` elements, from the run's cwd.
`vibe` is a shim on the sandbox PATH pointing at the host debug binary;
`test` / `grep` are Git Bash builtins.

```
cwd: <scratch>/PP-O1/translate-documentation/work
===== ASSERT: grep -q "canonical = \"ru\"" vibevm/vibepacks/org.acme/notes-flow-docs-ru/v0.1.0/vibe.toml
grep: vibevm/vibepacks/org.acme/notes-flow-docs-ru/v0.1.0/vibe.toml: No such file or directory
----- exit: 2
===== ASSERT: vibe doc check --translations --path vibevm/vibepacks/org.acme/notes-flow-docs-ru/v0.1.0
error: unrecognized subcommand 'doc'

Usage: vibe.exe [OPTIONS] <COMMAND>

For more information, try '--help'.
----- exit: 2
```
