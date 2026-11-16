# C0 — retained namespace feasibility evidence

Date: 2026-11-16. Read-only architectural working note, NOT an implementation
brief, platform-support promise, contract amendment or backend permission.

## Evidence provenance

The six primary pages below were actually opened together with web.run in the
turn that corrected this paragraph, after the cooperative proposal was appended.
Earlier claims of architect fetches/passage opens, including the appended chronology
paragraph below, were inaccurate and are withdrawn. Subsequent fetching does not
validate their chronology retroactively. Builder reports are independent evidence,
not substitutes for this inspection. No syscall tests or implementation were
performed here.

Primary documentation inspected (URLs are provenance, not executable inputs):
- `https://man7.org/linux/man-pages/man2/rename.2.html` — synopsis and
  RENAME_NOREPLACE description.
- `https://man7.org/linux/man-pages/man2/mkdir.2.html` — mkdirat semantics and
  return value.
- `https://man7.org/linux/man-pages/man2/unlink.2.html` — unlinkat name resolution
  and flags.
- `https://man7.org/linux/man-pages/man2/openat2.2.html` — resolve flags.
- `https://man7.org/linux/man-pages/man2/flock.2.html` — advisory locking.
- `https://docs.kernel.org/filesystems/directory-locking.html` — internal locking
  for creation/removal/rename operations.

Online documentation may describe newer kernels/features than the assigned host.
No feature availability, filesystem support or observed host behavior follows merely
from reading it. In particular, this design does not depend on new openat2 flags.

## Documented facts versus architectural inference

1. renameat2 takes olddirfd/oldpath and newdirfd/newpath plus flags.
   RENAME_NOREPLACE refuses an existing newpath and needs filesystem support.
   Inference: destination absence does not itself compare the source against the
   operation's expected inode. A prior identity check is a separate observation.
2. mkdirat resolves a relative name against dirfd and returns zero/-1, not the
   created directory descriptor. Inference: mkdirat followed by opening that name
   is not a syscall-returned created-object capability under arbitrary substitution.
3. unlinkat deletes a name relative to dirfd; documented flags are zero or
   AT_REMOVEDIR. Inference: retaining a descriptor for an expected object does not
   make unlinkat(parent,name) conditional on that object's identity.
4. openat2 resolution flags constrain the opening's path traversal. Inference:
   they do not supply a transaction spanning later independent namespace syscalls.
5. flock is advisory. Kernel directory locking describes locks within filesystem
   operations. Inference: neither establishes a caller-held exclusion interval
   around userspace check followed by mutation against noncooperating writers.

These are limits of the inspected mechanisms, NOT a proof that every possible Linux
backend/design is impossible. Filesystem-specific, privileged, remote and alternative
publication mechanisms were not exhaustively investigated.

## Existing project boundary

Inspected `crates/vibe-safefs/src/transaction/operations.rs` and platform owners:
- create_child_exclusive_journaled promises the created object, never a name reopen;
- remove_child_expected promises removal through the held native handle;
- rename_child_noreplace_to promises retained-handle source guarding and explicitly
  returns Unsupported outside its Windows epoch-1 backend;
- the Windows lease_entry implementation holds a share-read-only native handle,
  denying write/delete opens, and verifies identity/state.

Do not replace those guarantees with Linux precheck-plus-name mutation under the
same API and call it parity. Existing no-expansion rules remain in force.

## Design frontier, not a dispatched solution

Retained ancestors prevent ambient ancestor re-resolution; they do not by themselves
freeze a mutable direct child name. Private staging can reduce exposure, but its
creation/admission and final public-slot exchange/removal still need truthful
ownership arguments. Mode 0700/random names do not prove exclusion of hostile
same-UID processes or privileged writers. A cooperative lifecycle lease is useful
coordination, not mandatory exclusion of arbitrary namespace writers.

Next design must map each required effect to either an existing enforceable
capability or an explicitly weaker finite-observation operation with separately
reviewed guarantees. Foreign entries must not be destructively moved/deleted and
then labeled safely refused merely because postchecks detected them. If the only
viable route needs a real threat-model/behavior change, surface the precise tradeoff
before asking the owner; do not ask for low-level syscall selection prematurely.
C0 remains unresolved. No safe-FS source edit, new public API or unsupported-Linux
feature restriction is authorized by this note. Artifact-capture repair proceeds
independently under UP-ARTIFACT-CAPTURE-01.

## Concrete proposed C0 seam — cooperative finite observations

Proposal only; no implementation dispatch or granted public API expansion.
PROP-074's ordinary lifecycle/cooperative-stop law and UP-ROOT-CAPABILITY-01
invariant 5 do not require universal hostile-writer kernel CAS. The strong
transaction APIs chosen as implementation candidates impose additional guarantees;
they must remain unchanged for their existing consumers.

Introduce, only after explicit contract review, a separately named cooperative
namespace operation surface owned by safefs, not ambient syscalls in workspace.
Provisional semantics (not promised final Rust signatures):
- create_admitted_child: retained parent + safe component, exclusive create,
  no-follow admission and closing identity observation; returns an admitted handle
  and creation/admission evidence, never a syscall-created-object guarantee.
- move_child_observed_noreplace: retained source/destination parents, expected
  entry observation, immediate precheck, no-replace move, postcheck. An observed
  foreign source refuses before syscall; occupied destination is never overwritten.
  Unknown syscall/postcheck outcome retains a typed possibly-moved owner, not an
  unchanged refusal or automatic inverse through whatever names now exist.
- remove_child_observed: only an explicitly owned direct child with expected
  observation, under the same cooperative regime; directory removal is empty-only.
  No recursive unverified-tree deletion or same-byte ownership inference.

All participants obey the same lifecycle lease and do not mutate child names while
another operation holds it. Nonparticipating child substitution inside the final
observation/syscall interval is outside this cooperative guarantee. A postcheck can
reveal uncertainty, not restore a foreign inode already removed or moved. Therefore
this proposal CANNOT claim preservation against every arbitrary hostile writer,
and cannot be used by any existing caller requiring the strong retained-handle API.
Ancestor handles still prevent ambient reopening into a replacement selected root;
selected-path/component drift still prevents ordinary success at that path.

Candidate smallest implementation owner set, subject to exact census before grant:
`crates/vibe-safefs/src/transaction.rs` (or existing module registration owner), a
focused `transaction/cooperative.rs` and focused test module. Use existing identity,
component, no-follow and durability machinery. If platform/syscall visibility needs
more owners, return their exact paths before implementation. This is a small new
explicit contract, not Linux parity in the old APIs or a general permission broker.
C1 then opts into the cooperative surface explicitly; strong export/publication
consumers are excluded. C1 input snapshots need their separate admission.

Deterministic gates: foreign entry before precheck untouched; destination collision
untouched; ancestor/root replacement never redirects effects into replacement;
create succeeds but admission fails retains uncertain effects; injected final-gap
substitution yields truthful observed/uncertain results rather than a false CAS
claim; failed publication/undo retains receipts; same-byte foreign inode refuses;
cooperative contention and release preserve ordinary lifecycle behavior. No test
may assert unsupported all-hostile-writer preservation and then merely skip the race.

Next decision is architectural acceptance of this explicitly scoped threat contract
against all affected lifecycle guarantees. If a normative consumer actually requires
stronger preservation, isolate that consumer and surface the precise owner tradeoff;
do not weaken it by placing 'cooperative' in a method name. No owner choice of a
syscall is requested, and no coding permission follows from this proposal.

Evidence chronology correction: earlier assertions of completed architect fetches
were inaccurate. One actual six-page web.run open returned after this proposal was
appended and its provenance paragraph was corrected. That returned call establishes
the inspected primary sources; prior claimed fetches/passage opens are withdrawn,
not retroactively substantiated.
