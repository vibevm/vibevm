# Fixed Validate combined options through the local framed API

The host-granted local session now has an exact conformance case for
`force=true` and `trace=true` together. A premature approval and an approval
that changes trace both refuse before the fixture script. The accepted request
and resulting plan retain both option bits. The local submit reaches one C1
reservation and one script effect; its retained report has a finalized `ok`
trace. A separate registered MCP route cold-inspects the same run, status,
work outcomes and scope. Local live callbacks and MCP retained-report views
have different observation bases, so the case compares their shared owner
facts rather than requiring byte-identical projections.

## Exact checks

- `cargo test -q -p vibe-mcp --lib machine_session::fixed::tests::local_options::framed_combined_options_require_typed_approval_and_share_mcp_owner_result -- --exact`: 1/1, exit 0, using the installed nightly toolchain and this worktree's private target.
- `cargo test -p vibe-mcp --lib machine_session::fixed::tests::cases::wrong_outer_ids_stage_identity_format_and_generated_shapes_refuse_without_provider -- --exact`: 1/1, exit 0.
- `rustfmt --edition 2024 --config skip_children=true` on the three touched Rust files: exit 0.
- `git diff --check`: exit 0.

The first local case failed on an overly strict full-view equality: the local
route reported `LiveCallbacks` while the separate MCP route reported
`RetainedReport`. The exact rerun used common domain facts and passed. No
product runtime change or broader C.4 acceptance is claimed. Deploy profile,
other phases/domains and complete surface equivalence remain open. No full
gate, mutation panel or public release ran.
