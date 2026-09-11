# asserts — build-package-deploy

Executed verbatim from the page's `<assert>` elements, from the run's cwd.
`vibe` is a shim on the sandbox PATH pointing at the host debug binary;
`test` / `grep` are Git Bash builtins.

```
cwd: <scratch>/PP-O1/build-package-deploy/work/hello-vibe
===== ASSERT: vibe deploy --plan --profile local
error: `--profile local` was requested, but this project declares no deploy profiles (violates spec://org.vibevm.core/vibevm/common/PROP-054#OPEN-DEPLOY-TARGETS; fix: declare `[deploy.profiles.local]` with its `targets`, or drop the flag)
----- exit: 1
===== ASSERT: vibe deployments --json
{
  "command": "deployments",
  "ok": true,
  "count": 0,
  "deployments": []
}
----- exit: 0
```
