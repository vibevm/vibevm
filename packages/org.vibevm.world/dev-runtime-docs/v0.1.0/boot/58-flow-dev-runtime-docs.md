# Flow: Load-bearing setup docs {#root}

A project's setup and runtime documentation is **load-bearing** — it is the file
someone reaches for when the build breaks, the environment is wrong, or a
prerequisite is missing.

## The rule

Every change that touches the **toolchain, prerequisites, environment variables,
paths, or bootstrap steps** updates the relevant setup/runtime doc **in the same
commit**. Never ship a setup change with the doc update deferred — deferral is
exactly where the drift these files exist to prevent lives.

Full protocol: [`spec/flows/dev-runtime-docs/DEV-RUNTIME-DOCS-PROTOCOL.md`](../flows/dev-runtime-docs/DEV-RUNTIME-DOCS-PROTOCOL.md).

## Never

- Never ship a dev-env or runtime-setup change with its doc update in a later commit.
- Never let the setup docs describe a toolchain the project no longer uses.
