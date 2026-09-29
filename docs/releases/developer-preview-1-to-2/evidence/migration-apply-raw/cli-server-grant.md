# D1 CLI server grant candidate

Writer stopped with Cargo held until root confirms runtime compile green. No Cargo/test/server/provider invocation, Git, global configuration/environment mutation or user-path operation occurred.

Exact candidate paths:

- crates/vibe-cli/src/cli/mcp.rs (229lines): McpServeArgs adds migration_recovery_store:Option<PathBuf> and migration_file:Vec<String>. --migration-file is repeatable and requires migration_recovery_store through the actual clap definition. Omission leaves None/empty, with no inferred HOME/store. Root-file default admission remains ServerContext's existing owner.
- crates/vibe-cli/src/commands/mcp/mod.rs (528lines): run_serve retains its existing required canonical absolute project resolution, user configuration and lifecycle-policy composition; only when the explicit store is Some does it call with_manifest_migration_authority(store, files). Failure wraps the fixed source-cited admission diagnosis before Server construction. No store flag preserves the current context behavior/None. Provider and lifecycle policy are unchanged.
- crates/vibe-cli/tests/mcp_migration_authority_args.rs (78lines): three proposed real-parser cases import the actual owned cli/mcp.rs argument definitions and flatten McpArgs into a test clap Parser. No alternate flags/command grammar, help parser, server, filesystem or environment operation is reconstructed.

Source citations on the argument owner, composer and cases bind PROP044#MANIFEST-MIGRATION-SURFACES reqr1. Scoped rustfmt skip_children completed. Exact McpServeArgs struct-literal search across crates/xtask/installed owned Rust1.1 found only the defining struct, no outside literal consumers needing edits.

Selected target/three exact names, BEFORE any compile:

cargo test --locked --offline -p vibe-cli --test mcp_migration_authority_args <case> -- --exact

1. absent_migration_flags_preserve_compatibility — actual default path, absent store, empty members.
2. explicit_store_preserves_repeatable_member_selection — exact supplied path/store and ordered two-member list; store-only leaves default-file selection to the existing context owner.
3. migration_files_require_store_and_unknown_grammar_is_refused — files-without-store usage refusal, unknown --schema argument and store flag without a value through actual clap errors.

Use shared root target/jobs4 only after root runtime-green authorization. Intended nonzero cases3, but currently zero executed: no check, compile, test listing or whole CLI suite ran. Startup authority behavior remains pending actual owning integration proof; this atom's tests only bind syntax/compatibility. Product1.1/public hold and operation schema/epoch stay unchanged. Root reviews, verifies and commits; no acceptance claim.
