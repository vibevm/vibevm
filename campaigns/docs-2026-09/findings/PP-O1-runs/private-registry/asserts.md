# asserts — private-registry

Executed verbatim from the page's `<assert>` elements, from the run's cwd.
`vibe` is a shim on the sandbox PATH pointing at the host debug binary;
`test` / `grep` are Git Bash builtins.

```
cwd: <scratch>/PP-O1/private-registry/work/hello-vibe
===== ASSERT: vibe registry list --quiet
  1. acme (primary)
     url:     git@github.com:acme-specs
     org:     acme-specs
     host:    github.com (adapter: github)
     naming:  fqdn
     ref:     main
     mirrors: (none)
  2. vibespecs
     url:     https://github.com/vibespecs
     org:     vibespecs
     host:    github.com (adapter: github)
     naming:  fqdn
     ref:     main
     mirrors: (none)

vibe registry list: 2 registries, 0 mirrors, 0 overrides.
----- exit: 0
===== ASSERT: vibe registry test
Registry test
  acme       git@github.com:acme-specs     → reachable  (auth=none)
  vibespecs  https://github.com/vibespecs  → reachable  (auth=none)
vibe registry test: 2/2 reachable
----- exit: 0
```
