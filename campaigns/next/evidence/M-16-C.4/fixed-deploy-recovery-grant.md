# Fixed Deploy durable pointer and recovery-only reattach

M-16-C.4 now has a trusted library boundary for one authored fixed Deploy
profile and target. The host issues one durable pointer under the incumbent
lifecycle lease and C1 namespace. The pointer records physical-root correlation,
scope, manifest witness, selected generation identity and policy generation;
it never stores a callback, approval or deployment result. An active reattach
requires matching current manifest, generation and policy. Drift returns a
distinct inspection-only host for old keys; malformed or foreign pointer
evidence refuses without another namespace. Active submissions recheck policy
and manifest under the lease before C1 admission.

## Exact checks

- `cargo test -q -p vibe-orchestrator --lib machine_submission::deploy_grant::tests::`: exit 0, 8 passed, 0 failed, 530 filtered out, on installed nightly and the private worktree target. Cases cover matching active reattach, manifest/profile/policy/generation drift, a real completed deployment, old-key partial effects, malformed pointer/hash and foreign settings root. The resumed coordinator added the profile and generation drift cases after reviewing the retained WIP; the eight-case rerun passed.
- File-scoped `rustfmt --edition 2024 --config skip_children=true --check` and `git diff --check`: exit 0. The `skip_children` setting confines the check to changed files; a recursive check also reported unrelated existing formatting in `machine_submission/lifecycle_adapter/live_owner/control.rs`.

The service is currently a library seam. CLI machine-frame and registered MCP
process startup do not yet issue or reattach this Deploy pointer, so this atom
does not claim cross-process Deploy equivalence. Native Windows/macOS behavior
is unrun. No full gate, routine mutation panel or public VibeVM 1.1.0 release ran.
M-16-C.4 remains active.
