# UP-C2-SAFEFS-01 — narrowly authorized boot primitives

Date: 2026-11-16. Amendment to C2 brief67d118a79cc4a4f02f32fad1dede35d3ba045eeb.
Exact78 input21547 unchanged, Linux-scoped development under relayed direct USER
mandate. No new dependencies/unsafe, real user state, cleanup, source commit,
publication or public lifecycle activation. Existing strong APIs remain unchanged.

## Exact added output perimeter

Existing crates/vibe-safefs/src/:
- lib.rs (safe exports only);
- project.rs (parent-lock methods/module registration only);
- transaction.rs (module registration only);
- transaction/retained_cooperative.rs (observed-file-removal forwarding only).
New files under that same src/:
- transaction/boot_stage.rs;
- transaction/boot_stage_tests.rs;
- project/parent_lock_tests.rs;
- transaction/retained_file_remove.rs;
- transaction/retained_file_remove_tests.rs.

Ordinary validators/file.rs, old Windows platform operations, Cargo and core role
algebra are not changed. If existing private visibility prevents safe reuse, request
the exact extra owner; do not duplicate a weaker low-level validator.

## Boot stage domain choice

Choose trusted journal-owner handoff, NOT moving StageRole into core or duplicating
closed role list in safefs. BootStageName::from_authorized_parts validates only
lexical domain: incumbent generation6..64 ASCII alphanumeric, nonempty lowercase
ASCII role words separated by single hyphens, exact incumbent full filename
`.vibe-boot-txn-{generation}-{role}.stage`. Bound lengths to incumbent grammar;
reject separators, reserved aliases, malformed/double-hyphen role and NUL.

Workspace MUST first validate its existing closed StageRole and generation through
incumbent journal laws. An arbitrary safe caller can mint a lexically valid unknown
role: safefs token is explicitly NOT unforgeable role authorization. No generic
allowed_roles argument or ordinary reserved-name bypass. Unknown role refusal is a
workspace journal integration gate, not a safefs closed-role claim.

Specific retained-parent read/inspect/create methods accept only BootStageName.
Stable read remains no-follow regular single-link and bounded. Exclusive creation
retains actual file handle/parent/name and expected evidence BEFORE writes, records
write/sync/closing name identity/parent-sync results, returns typed private receipt
or failure retaining partial effects. No automatic stage deletion on error. No
same-byte foreign inode adoption. Ordinary name API behavior unchanged.

Stage cleanup is REQUIRED for usable C2, not an unspecified future remover: provide
specific receipt-based observed removal with actual held identity/expected state,
and separate fresh-recovery entry accepting validated name+actual observed state
under incumbent admitted journal intent. The latter does not reconstruct an old
live receipt from digest. Private recovery retains all uncertainty; neither method
can clean a stage based on filename alone. Share the observed removal engine where
possible, with explicit domain token instead of arbitrary raw reserved string.

## Parent lock

Pinned::try_lock_file and blocking lock_file sibling preserve the actual boot lock
posture (blocking incumbent), root lifecycle lease remains nonblocking unchanged.
Use retained parent's safe no-follow open/create, existing regular/single-link
checks, real OS lock and held-vs-current-name identity verification. Guard owns
parent/file/name and supports capability-relative assert_current; never reopens
Pinned.path. No relocation to .vibe, arbitrary lock-name bypass or reentry claim.
Preserve existing root Project lock behavior. Acquire/drop semantics must match
existing OS guard; no unsafe destructor or speculative removal of lock names.

## Cooperative file removal

Separate from strong remove_child_expected. Session::remove_file_observed validates
same-session parent/root, ordinary name and expected regular single-link state,
opens/retains actual file, compares held and named state BEFORE unlink through
retained parent. Closing observation/root-parent validation determines outcome;
error alone is not no-effects proof. Retain expected/observed states, held file,
parent/root/private name, phase and durability facts in failure. Same-byte foreign
inode, symlink, directory or unsupported type refuses before action.

Final-gap noncooperating replacement is outside cooperative guarantee. Do not claim
inode-CAS or untouched foreign object in that excluded gap. No compensation,
recursive deletion, retry of uncertain mutation or ambient ancestor reopen. Boot
stage-specific cleanup uses this semantic law through its explicit domain method,
not ordinary ensure_safe_component weakening. Failed sync after successful unlink
must preserve removed/effect evidence, not be mislabeled unchanged/restored.

## Gates and handoff

Synthetic Linux roots only: unchanged boot grammar, exclusive collision, live stage
identity/bytes/mode, partial write/admission/sync error, stage cleanup and malformed
lexical names; unknown role tested at actual workspace journal boundary. Parent
lock contention/drop/current-name replacement/ancestor replacement, blocking versus
try behavior without test deadlock. Ordinary and bootstage removal success, foreign
same-byte inode/nonregular refusal, deterministic final-gap/postremove error,
truthful sync/uncertainty and no foreign cleanup. Test guard against ordinary API
acceptance of reserved names.

Existing safefs adjacent tests/lint/fmt/Conform and C2 journal integration gates on
coherent bytes, nonzero counts; Windows runtime unrun, no new parity claim. Return
frozen exact diff/manifest, safety and domain-boundary mapping. This permits code
within the listed scope, not primitive or C2 clearance. C1 pending panel, C3 native
and C4 route obligations remain separate. Ordinary in-scope helper factoring needs
no per-function permission; new owners/semantic changes require precise review.
