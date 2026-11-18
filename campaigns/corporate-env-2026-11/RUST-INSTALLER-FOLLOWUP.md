# CORP-ENV follow-up: native Rust distribution installer

Owner mandate2026-11-18: after finishing APP-VERSIONS, return to CORP-ENV.
The corporate distribution currently boots through bash/cmd scripts; add a small
Rust installer executable inside the distribution so installation can start where
script execution is restricted. Then update the owner's existing `~/corp` instance.
This is queued work, not authorization to interrupt APP-VERSIONS Windows migration.

## Ownership and sequence

Architect owns existing-flow analysis, contract, brief and cross-cutting review.
Builder owns implementation, tests and local integration. First finish the active
APP-VERSIONS scope; then inspect the existing distribution generator, scripts,
site-profile/self-install owners and actual `~/corp` layout before choosing code
changes. Reuse existing installation semantics rather than inventing a parallel
configuration writer or resolver. The installer entry point must not merely invoke
bash/cmd scripts as its essential bootstrap path.

## Required outcome

- A directly launchable small Rust application is shipped inside the corporate
  distribution for the relevant supported targets, especially Windows.
- It performs the current intended bootstrap/install/configuration workflow through
  existing Rust owners, with clear diagnostics and the existing consent/offline,
  digest, source and PATH rules preserved.
- Script launchers may remain compatible entry points, but restricted script
  execution must not prevent the native executable's supported workflow.
- Verification exercises isolated installation paths and failures without altering
  normal host installations or silently granting elevated privileges.
- After reviewed implementation, refresh this owner's distribution in `~/corp`.
  Inspect first; preserve unrelated/custom content and existing artifacts unless
  their replacement is explicitly part of the update. Report exact changed files
  and validation. Do not wipe or recreate the whole directory by default.

## Authority boundary

The owner explicitly requested the local `~/corp` update as part of this task.
This does not authorize external release/publication, remote corporate rollout,
credential access, host security-policy bypass or signing/configuration changes.
A native program avoids a dependency on script execution; it does not promise to
bypass policies that also prohibit unapproved executables. Ask only for genuine
owner-only decisions discovered during the existing-flow analysis.
