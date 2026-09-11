# asserts — ship-tools

Executed verbatim from the page's `<assert>` elements, from the run's cwd.
`vibe` is a shim on the sandbox PATH pointing at the host debug binary;
`test` / `grep` are Git Bash builtins.

```
cwd: <scratch>/PP-O1/ship-tools/work
===== ASSERT: vibe bin list --path packages/notes-tools
error: unexpected argument '--path' found

Usage: vibe.exe bin list [OPTIONS]

For more information, try '--help'.
----- exit: 2
===== ASSERT: vibe bin exec notes-check --path packages/notes-tools -- --help
error: violates spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-025#dispatch: no installed package declares a binary `notes-check` (declared: []); fix surface: `vibe bin list` shows the full table
----- exit: 1
```
