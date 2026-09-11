# asserts — publish-a-package

Executed verbatim from the page's `<assert>` elements, from the run's cwd.
`vibe` is a shim on the sandbox PATH pointing at the host debug binary;
`test` / `grep` are Git Bash builtins.

```
cwd: <scratch>/PP-O1/publish-a-package/work
===== ASSERT: vibe registry publish packages/notes --dry-run
Publishing <scratch>\PP-O1\publish-a-package\work\packages\notes → registry `local` (`<scratch>\PP-O1\publish-a-package\work\registry`) [dry-run]
error: loading publish token: publish refused: no token available for host `C`. Set `VIBEVM_PUBLISH_TOKEN` or write a token to `~/.vibe/git.publish.token`. (violates spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#publish; fix: export `VIBEVM_PUBLISH_TOKEN` or write `~/.vibe/<host-prefix>.publish.token`)
----- exit: 1
```
