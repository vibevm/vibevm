# Flow: Load-bearing setup docs {#root}

<status stage="impl" state="done"/>

@fact:SETUP-AND-RUNTIME-DOCUMENTATION-IS-LOAD-BEARING A project's setup and runtime documentation is **load-bearing** — it is the file
someone reaches for when the build breaks, the environment is wrong, or a
prerequisite is missing. @status:impl/done

## The rule {#rule}

@fact:EVERY-SETUP-TOUCHING-CHANGE-UPDATES-THE-DOC-IN-THE-SAME-COMMIT Every change that touches the **toolchain, prerequisites, environment variables,
paths, or bootstrap steps** updates the relevant setup/runtime doc **in the same
commit**. @status:impl/done

@fact:NEVER-SHIP-A-SETUP-CHANGE-WITH-THE-DOC-UPDATE-DEFERRED Never ship a setup change with the doc update deferred — deferral is
exactly where the drift these files exist to prevent lives. @status:impl/done

@fact:sibling-document-pointers Full protocol: @spec://org.vibevm.world/dev-runtime-docs/flows/dev-runtime-docs/DEV-RUNTIME-DOCS-PROTOCOL#root. @status:impl/done

## Never {#never}

- @fact:NEVER-DEFER-THE-DOC-UPDATE-TO-A-LATER-COMMIT Never ship a dev-env or runtime-setup change with its doc update in a later commit. @status:impl/done
- @fact:NEVER-LET-THE-DOCS-DESCRIBE-AN-ABANDONED-TOOLCHAIN Never let the setup docs describe a toolchain the project no longer uses. @status:impl/done
