# UP-C0-LINUX-PROOF-01 — cooperative namespace proof

Date: 2026-11-16. Bounded synthetic implementation/proof atom, not C1 activation.
Builder codes under existing relayed direct USER mandate; architect reviews.
Base `6a83be1dd35d7bec2de4911ee375c2be3cb8686a` plus exact committed brief delivery.
NO candidate overlays, root/epoch/artifact/privacy inputs, new dependency, toolchain,
remote, user-state mutation, cleanup or publication grant. Record actual task binding.

## Decision and exact perimeter

Choose a narrowly reviewed Linux renameat2(RENAME_NOREPLACE) bridge using existing
libc dependency, NOT overwrite rename/precheck or a guessed cap-std API. This is a
separate cooperative proof surface, never a fallback behind existing strong APIs.

Allowed files:
- crates/vibe-safefs/src/transaction.rs — private module registration only;
- crates/vibe-safefs/src/transaction/cooperative.rs — new private proof operations;
- crates/vibe-safefs/src/transaction/cooperative_linux.rs — new Linux bridge;
- crates/vibe-safefs/src/transaction/cooperative_tests.rs — new synthetic tests.

Keep surface crate-private/test-only until independently reviewed C1 caller contract.
No lib.rs export, lifecycle consumer wiring, Cargo change or old Windows API edit.
Private helper factoring in these files/purposes needs no per-function permission.

Crate deny(unsafe_code) stays. Permit ONE function-local allow(unsafe_code) in Linux
bridge solely for libc::renameat2 with borrowed live directory fds, validated single
component CStrings and libc::RENAME_NOREPLACE. No raw syscall-number assumptions,
FD ownership transfer, pointer persistence or custom FFI declarations. Document
borrow/pointer lifetimes and local spec deviation. All other code stays safe Rust.
If pinned dir's existing safe fd borrowing cannot express this, stop for exact owner
review. Unsupported target/libc symbol availability must compile/refuse explicitly,
not fallback to overwriting operation. Non-Linux backend is unsupported proof only,
not declaration that whole personal lifecycle will reject supported platforms.

## Cooperative posture and truthful outcomes

No safefs->lifecycle reverse dependency. A private proof session retains one admitted
root and ties descendant parent capabilities to it. Its explicit caller posture is
cooperative namespace exclusivity for the operation lifetime, to be enforced later
by actual LifecycleLease owner. A minted token alone does NOT prove a lease or OS
exclusion. Tests exercise posture; no production caller can activate this API yet.

- Create: exclusive component mkdir then no-follow admit; result is admitted child,
  not syscall-returned created-object proof. Creation followed by admission failure
  retains changed/uncertain evidence; never auto-delete an unverified entry.
- Move: retained source/destination parents, validated components, expected source
  observation, immediate precheck, atomic destination no-replace, closing source/
  destination observations. Existing occupant refuses untouched. Success needs exact
  expected observed identity at destination and expected source absence. Final-gap
  uncooperative source substitution is outside the cooperative guarantee: if observed
  afterward, return possibly-moved/uncertain, NOT untouched refusal/restored.
- Remove: explicitly operation-owned direct child only; directory empty-only. Retained
  parent and expected observation checked immediately before action, closing outcome
  measured. No recursive delete, pathname reopen of ancestors or compensation against
  foreign entries. Hostile final-gap removal cannot promise inode-CAS; state this.

Paths are diagnostics, not renewed authority. Parent/root movement never redirects
into replacement tree; selected-path drift prevents normal selected-path success.
Same bytes at foreign inode refuse. No blind retry on EINTR/ambiguous errors, no
automatic rollback/Drop cleanup. Preserve opaque owned recovery/effect evidence.
Directory durability uses existing truthful sync observations, not Windows-specific
JournalRecoverable wording as a Linux success claim. Syscall error alone does not
prove no namespace effects on every filesystem; reobserve and retain uncertainty.

## Mandatory proof gates

Synthetic isolated roots only. Test directory and regular-file move, occupied
regular/directory/symlink destination, exact-source mismatch/same-byte foreign inode,
ancestor/root replacement preserving replacement sentinels, invalid components before
syscall, observed unsupported/cross-filesystem error classification, create admission
failure, empty remove/nonempty refusal, and deterministic final-gap substitution
with honest uncertainty/no compensating foreign mutation. Reuse existing injected
hooks only if semantics fit; focused private test hook in these files is allowed.
No real settings, native plugins, providers or arbitrary user files.

Verify actual destination NOREPLACE on assigned Linux filesystem and report target/
filesystem assumptions. Mocked unsupported/crossfs tests are not real cross-volume
proof. Test uncertain error observations rather than asserting all errors mean no
mutation. Adjacent safefs tests/strict lint/fmt/Conform with nonzero selected counts;
Windows strong API source remains byte-identical. No broad suite by habit.

Primary source actually opened via web.run in authoring turn:
https://man7.org/linux/man-pages/man2/rename.2.html — signature uses dirfds/names;
NOREPLACE requires filesystem support and protects destination absence, not expected
source inode. This proves interface facts only, not the proposed implementation.

Return frozen exact manifest/diff, dependency/input census, safety/rights reasoning,
actual gate outputs and unresolved caller posture. Architectural clearance of proof
is required BEFORE public export or C1 slot/ignore/stash adaptation. Existing strong
create/remove/rename laws remain unchanged; no Windows parity claim or blanket
hostile-writer guarantee follows. Whole USER-PROJECT mandate remains unfinished.
