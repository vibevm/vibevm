# Retained-image blocker closure

The specific retained-before-image blocker is closed by the reviewed source and three retained exact-case results. This is read-only closure evidence, not runtime/task/platform acceptance. No code, Cargo/test, Git or user-path operation ran here.

ExternalStore::sync_stable_file_bounded delegates to the new external_sync owner. It opens the selected existing file without create/truncate, verifies regular single-link shape and held identity, compares an exact stable snapshot, then calls the actual held file.sync_all. It rechecks the held identity and expected snapshot afterward and returns the separately observed parent-directory tier. Windows requests a write-capable handle for FlushFileBuffers, without performing a content write; native macOS/Windows proof remains unrun.

Matching orphan retain_image now reaches confirm_image before prepared intent. Before every NEW target publication/restore, recovery.target_write rereads before.bin and matches validated before bytes/raw hash/length, then requires confirm_image before target snapshot recheck and mutation. Failure returns SynchronizationFailed and prevents the target write; it does not treat byte equality as a prior sync receipt. Lock/cache/store policy checks precede the sync and target primitive calls.

Inspect and exact terminal no-new-write replay bypass this synchronization path. The owner test hook is after native prechecks immediately before actual file.sync_all: the failing hook prevents publication; the successful hook observes the real sync boundary. The two regression cases compare exact target snapshot/identity and retained image, confirm no target effect on refusal, and verify zero sync calls for inspect/replay.

Actual logs0/1/2 each show1passed/0failed/569filtered: matching orphan confirmation/failure; loaded restore confirmation with inspect/replay exclusion; prior checkpoint and sticky unknown durability. A confirmed image sync does not upgrade unknown older journal/namespace durability, and the aggregate remains unconfirmed where required. Namespace weaker tiers remain distinct and no unrestricted content-CAS/OS exclusion is introduced.

The new before/after retained-image sync process boundaries are source-bound; root's two restart reruns remain outside this closure's measured evidence. No additional gap in this single prerequisite was found. Root retains prefix proof, runtime commit, semantic acceptance and native platform judging. Writer stopped.
