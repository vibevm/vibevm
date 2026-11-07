# Bound boot-check adapter — independent read-only review

Verdict: no concrete correctness blocker found in the reviewed source. Qualified support for this bounded consumer; root retains acceptance. No product edits, Cargo, network, Git operation or subagent launch performed for this review.

## Source-established behavior

- verify_boot_graph_with_spec_format reads the incumbent durable resolution, retains prepare_with_nodes with the caller-selected format, and supplies that exact context to build_unit_table_bound. It lowers the existing owner runtime set, uses plan_digest_frames_for(selected format), and delegates pending/fingerprint/stale propagation and deterministic ordering to verify_fingerprints. No alternate slot parser, format inference, generated_inputs::admit, compilation or regeneration was added.
- The legacy wrapper passes None, retains declaration-format refusal during binding preparation, and preserves its legacy table and Mixed plan-frame route. Both verifier entrypoints are exported by the installed library surface.
- CLI uses symlink_metadata on the durable lock; only NotFound skips the adapter. A dangling symlink or nonregular present lock is still applicable and reaches the strict durable reader. Present read errors are not changed into an empty graph. Workspace load, selected-format configuration, binding and verifier failures all become BootGraphIntegrity Error findings. Ordinary report emission precedes exit1; stale units stay Warning and warning-only exit0. No new report member or CheckId appears.
- The actual format selection calls the existing reinstall::effective_rederive_format owner (UserConfig plus install resolver), rather than reading representation intent back from the slot record or STATIC suffix. A no-lock project returns before this config load.
- The existing fingerprint reader's absent/unreadable/unparseable unit INDEX maps to stale, not silently clean; static dependency propagation and sorted output remain unchanged. The checker deliberately does not authenticate STATIC-body bytes.

## Observed evidence, not rerun here

Reviewed terminal log content: real CLI6 pass; bound library2 pass; legacy stale/activated-transform2 pass; corrected native freshness7 pass. Workspace and CLI strict Clippy logs end successfully. The earlier native filter ran0 tests and supplies no coverage; it is retained explicitly as such. I did not infer terminal Conform success from an in-progress log.

The CLI tests perform actual local-registry installs for declared arbitrary-suffix parent plus static child in Mixed and Xml. Fresh state is clean; corrupt and missing INDEX report the exact parent warning and success exit, with restored-positive control. Format drift, malformed lock, missing slot, nonregular lock, malformed manifest and malformed operator config yield one BootGraphIntegrity error, JSON ok=false, exit1. No-lock malformed config stays outside this adapter. Operator default after removing the project format override and valid empty lock are explicit cases. The command helper compares project and operator directory/file-byte snapshots around every check.

The bound library tests prove legacy declared refusal, selected/wrong format behavior, missing INDEX stale result and lock-selected hash disagreement refusal. Source inspection verifies the selected-format plan-frame call; these new fixtures do not independently discriminate an XML-specific activated-plan frame against a Mixed-specific activated-plan frame. Existing activated-plan/native tests provide neighboring compatibility evidence, not that additional exact runtime matrix. No such extra claim is made.

## Limits

Snapshot tests cover file bytes and directory membership in the named project/settings scopes, not all machine state or filesystem metadata. This is the incumbent finite fingerprint/pending verifier and error-reporting adapter, not new source authentication, global CAS/transaction, STATIC-body integrity or complete host check/docs/skills support. No full A4/campaign completion claim.
