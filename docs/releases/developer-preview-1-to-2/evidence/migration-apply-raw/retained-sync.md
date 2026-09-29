# Retained before-image data synchronization

Writer stopped. Added ExternalStore::sync_stable_file_bounded(relative:&str,cap:usize,expected:&StableFileSnapshot)->Result<DirectoryDurability>. An Ok return confirms actual readable held-file sync_all; returned enum is the actual pinned-parent directory flush tier, including failures/reduced support. The runtime must assess that tier before target mutation. Byte observation alone is not treated as durable retention.

It reuses project_view and existing holder_of no-follow parent walk, opens the existing regular single-link readable file without creation/truncation, matches held physical identity and exact full snapshot before sync through the existing bounded reader, syncs the held file, rechecks regular/single-link held identity and exact named snapshot afterward, then invokes the corrected parent directory sync owner. Windows opens read+write for FlushFileBuffers without a content-write operation. Missing/changed/replaced/link/over-cap/error cases refuse; syscall failures propagate. No handle or raw OS identity escapes. An outside participating-client barrier remains required; this is not unrestricted CAS against uncooperating writers.

Root-approved perimeter extensions kept every owner≤600: external.rs588lines delegates to included new transaction/external_sync.rs70lines. Existing transaction/tests/external.rs remains501lines with formatting retained. Two new tests would format that owner to606, so approved new tests/external_sync.rs109lines+one include/macro registration in transaction/tests.rs were used. file/project_methods.rs changed ONLY fn holder_of→pub(crate) fn holder_of for existing capability-law reuse, preserving other formatting. lib.rs adds only the existing-test/inject-failures gated hook export. No dependency, unsafe, syscall, codec, second reader/hash or generic framework added.

Hook arm_before_retained_file_sync(Option<Box<dyn FnMut()->bool>>) exists only for test/inject-failures. It observes the actual boundary after admission checks immediately before file.sync_all; false proceeds, true refuses before the syscall, None clears. It reads no environment and adds no production global state. Runtime writer received compiled stable API/hook to prove matching orphan/loaded-image invocation and refusal before target writes.

Exact selected tests (--locked --offline -p vibe-safefs --lib, jobs4/sharedtarget, -- --exact):

- transaction::tests::retained_image_sync_confirms_data_without_rewrite_and_refuses_injected_failure
- transaction::tests::retained_image_sync_refuses_replacement_changed_bytes_missing_and_links

Both EXIT0, each1passed/0failed/132filtered. Matching case observed real file-sync success and parent Synced; observer invoked once. Injection and all refusals compare tempdir names/types/bytes unchanged. Same-byte replacement retains old inode under saved.bin to avoid reuse; changed-byte, missing, hardlink and symlink paths refuse. Link subcases on Windows depend on symlink privilege; this Linux proof does not assert other native-platform proof.

Raw logs /tmp/vibevm-next-M13-D1-retained-sync/{matching-tests.log,refusals-tests.log}; existing safefs warnings retained. Targeted formatting touched only new/delegating owners; no old2+5 cases rerun, broad gate, Git/public/user-path mutation or runtime edits. Source APPLY durable before-state prerequisite and SEC-NO-FOLLOW remain governing. Root retains runtime integration/review/acceptance.
