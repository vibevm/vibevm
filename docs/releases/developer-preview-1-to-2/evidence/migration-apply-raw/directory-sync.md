# Capability directory synchronization fix

Writer stopped. Only crates/vibe-safefs/src/transaction/operations.rs and transaction/tests/external.rs changed. Existing test macro body preserved; two new bodies formatted independently to avoid prior unrelated macro reindent. Governing SEC-NO-FOLLOW and portable APPLY tier semantics retained.

The before-code tempdir probe observed held descriptor fdinfo flags012600000 and sync_all Err(Os{code9,kindUncategorized,message Bad file descriptor}). A readable Dir.open(".") handle through the same capability had matching physical identity and sync_all Ok. This is an observed EBADF/O_PATH problem, not an inferred filesystem refusal. Installed cap-primitives oflags.rs explicitly adds target_o_path for directory-only opens without readdir_required; cap-std Dir.open delegates read(true) without that directory-only request.

Fix on Unix only: derive held expected identity, reopen fixed self operand "." readably through the retained Dir, verify directory kind and reopened/held identities before sync, call real sync_all, and verify both identities afterward. No ambient pathname, raw syscall, unsafe, dependency or component-policy relaxation. The helper never creates entries. Identity mismatch is Failed(InvalidData); underlying I/O error kinds are preserved. Existing Unsupported handling remains truthful, and Windows continues its existing clone/sync and journal_recoverable classification. Genuine sync failures are not converted to success. Linux native evidence does not claim macOS/Windows native proof.

Bound tests --locked --offline -p vibe-safefs --lib, jobs4/sharedtarget, each -- --exact:

- transaction::tests::directory_flush_reopens_readable_capability_without_namespace_creation
- transaction::tests::directory_flush_refuses_a_foreign_expected_identity

Before-code probe initially EXIT101 due my unqualified private identity function in the test macro (probe.log), corrected qualified reference; actual probe EXIT0 1passed/129filtered (probe-2.log). After primitive change first test EXIT0 1passed/130filtered (readable-flush-tests.log), including actual new root.sync_directory()==Synced and namespace/byte snapshot equality. Second test EXIT0 1passed/130filtered (identity-refusal-tests.log): different real tempdir expected identity refuses and snapshot stays identical. No test-only global hook was added. Existing safefs warnings retained. No old5repeat/wholecrate/formatall/runtime rerun/Git/public/user-path writes. Root reruns basicApply and dependent affected runtime cases.

Raw logs under /tmp/vibevm-next-M13-D1-directory-sync/ retain complete stdout/stderr/errno observations.

Installed sources inspected (all under /home/olegchir/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/): cap-std-4.0.3/src/fs/dir.rs; cap-fs-ext-4.0.3/src/dir_ext.rs; cap-primitives-4.0.3/src/fs/{open_dir.rs,open_options.rs,mod.rs,dir_options.rs}; cap-primitives-4.0.3/src/rustix/linux/fs/open_impl.rs; cap-primitives-4.0.3/src/rustix/fs/{open_options_ext.rs,mod.rs,open_unchecked.rs,dir_options_ext.rs,oflags.rs}. Exact source owners project/pinned.rs and transaction/operations.rs provided existing handle/identity policy; Cargo.lock confirms4.0.3. No network or package changes.
