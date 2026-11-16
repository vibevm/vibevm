# UP-WINDOWS-CAPTURE-A1 — readonly publication and private allocation

Date: 2026-11-16. Concrete architectural contract/design gate, NOT an implementation
packet or authorization of new unsafe bindings/dependencies. Artifact candidate
28c38fba remains unchanged; A1 blocks universal integration. Linux evidence and
pending coherent test results remain separate. Builder owns eventual implementation
and tests; architect owns contract/perimeter review.

## Decision and two independent capabilities

Do not extend generic publish_new_in_with_mode by calling set_permissions and
ignoring readonly staging cleanup. Do not infer private Windows ACLs from tempfile
or unix_mode=None. Keep existing strong namespace APIs and existing mode publication
behavior unchanged. Additive permissions publication and private allocation must
have independent receipts/errors/tests and join only at captured backing consumer.

### A1-P — stage-owned permissions publication

Provisional surface: Project::publish_new_in_with_permissions(directory, name,
bytes, permissions) -> Result<(), PublishError>. This is file permissions/readonly
preservation, NOT ACL copying from input or permission-policy bypass.

Owner: existing file/create_new.rs, with a focused private platform helper if
required. Until exact helper paths/bindings are admitted, only the existing owner
is identified, not a blanket file/platform subtree grant.

Required sequence and invariant:
1. Exclusive stage creation relative to retained parent returns owned held file.
   Preserve that actual identity through bytes/permissions/sync and publication.
2. Apply permissions on the held stage before target visibility. Re-observe held
   permissions and bytes; Windows readonly and Unix mode are distinct observations.
3. Destination no-replace publication remains authoritative; foreign occupants are
   untouched. Held-stage cleanup must remove ONLY the operation's staging entry,
   preserving the destination link and readonly state. A Windows implementation
   may investigate handle-based extended disposition that ignores readonly, but
   must prove which link is removed, access/share rights and filesystem support.
4. Unsupported cleanup capability is detected before target publication where
   possible. If discovered after an owned stage exists, retain its exact recovery
   evidence. BeforePublication describes the unchanged DESTINATION and may coexist
   with explicitly reported staging residue, as publish.rs already specifies.
   Track residue/ownership separately; never claim no effects or promote an
   untouched target to PossiblyPublished merely because a stage remains. Use an
   additive typed private residue carrier if PublishError alone is insufficient;
   preserve existing PublishStage meaning. No deletion/chmod of an ambient name
   to manufacture success.
5. After publication/cleanup verify visible identity, bytes, expected permissions
   and single-link count. A foreign replacement is not ownership even with equal
   bytes. Do not clear readonly on a shared inode to delete stage: that changes
   destination attributes too. No fallback to a weaker path-based remover.

This contract does not mandate a particular Windows syscall sequence. If existing
stage handle lacks DELETE access or safe link-specific deletion cannot be proved,
return that exact blocker before API implementation; do not retrofit unsafe FFI
inside generic create_new.rs without separate perimeter approval.

Required gates: writable and readonly success; link collision; prelink write/attr/
sync failure; stage cleanup failure; postlink failure; same-byte foreign replacement;
readonly source remains unchanged; true residue classification/no speculative undo;
legacy mode API nonregression. Windows execution is required for Windows success
claims, not Linux-only compilation or mocked disposition success.

### A1-D — private fresh Windows directory admission

Provisional surface: a retained parent's exclusive private-child allocator returning
an owned admitted directory plus private security/identity evidence, with separate
NotCreated and CreatedButUnsealed outcomes. Use the existing Windows native owner
transaction/platform/operations.rs and windows_abi.rs only as the starting census.
No automatic implementation/API perimeter grant is made by naming them.

Required policy: explicit protected DACL at creation, no inherited broad grants;
allow the effective process token's current user and LocalSystem only, with explicit
child inheritance to protect payload/recovery descendants. Do not grant Everyone,
Authenticated Users or Users; no explicit Administrators grant required. This is
ordinary discretionary protection, NOT exclusion of privileged administrators,
backup operators or hostile same-user processes. Impersonation/thread-token mode is
unsupported until separately specified; do not accidentally build ACL for a user
other than the actual admitted caller. A privileged process must not silently
substitute an unrelated desktop session's identity.

Required procedure:
1. Select/admit scratch parent through existing runtime authority and retained
   no-follow capability; refuse unsupported filesystems/security semantics.
2. Build a valid descriptor for the actual caller with protected DACL and necessary
   inheritable child ACEs before creation. Null/empty DACL are not synonyms for this
   policy. No temporary permissive directory followed by ACL tightening.
3. Create exclusively relative to retained parent, passing the descriptor at creation;
   retain returned native handle rather than reopening a raced name as created proof.
4. Query owner/security control/DACL and file identity through the held handle before
   writing payload/recovery data. Validate the exact allowed policy and no reparse
   object. Preserve descriptor buffers/SIDs and handles with explicit RAII lifetime.
5. Any uncertain creation/security verification result retains opaque recovery owner
   and identity where known; never recursively removes unverified content. Refuse
   fallback to inherited ACLs, ambient CreateDirectory path, world-readable scratch
   or an undocumented chmod equivalent. Captured backing consumes this receipt.

No new dependency or unsafe code is authorized yet. Next implementation proposal
must enumerate minimal Windows bindings/functions, struct layouts/link libraries,
rights/share flags, error mapping and exact focused helper/test paths. Prefer an
already approved Windows binding owner if sufficient; otherwise return one bounded
binding decision rather than duplicate ad-hoc ACL code across consumers.

Required gates on Windows: broad-inheritance parent does not leak child access;
process-user/System access, foreign-user denial using a synthetic token/test account
where available; descriptor/owner mismatch; symlink/reparse/foreign same-name refusal;
creation succeeds/security query fails; inherited payload/recovery policy; readonly
publication under allocated root; no private path/security descriptor in public
errors. Report unavailable second-user/privileged cases unrun, never simulate their
acceptance from descriptor formatting alone.

## Primary evidence actually retrieved for this proposal

Architect web.run open returned these four Microsoft primary pages after the
initial draft was written and before committed delivery in this turn:
- https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-fscc/2e860264-018a-47b3-8555-565a13b35a45
  FileDispositionInformationEx: ignore-readonly flag, DELETE access requirement,
  unsupported/error outcomes. This is not proof of a working stage-link algorithm.
- https://learn.microsoft.com/windows/win32/api/winternl/nf-winternl-ntcreatefile
  native create returns a handle and accepts ObjectAttributes.
- https://learn.microsoft.com/en-us/windows/win32/api/ntdef/ns-ntdef-_object_attributes
  creation SecurityDescriptor member.
- https://learn.microsoft.com/en-us/windows/win32/secauthz/security-descriptors-for-new-objects
  explicit creation security and protected DACL inheritance controls.

These were returned primary pages, not executed Windows tests or a complete
ABI/security audit. Earlier C0 fetch chronology errors do not supply A1
evidence. Existing safefs source/create_new and native owners were read locally.

## Next action and acceptance boundary

Builder may produce the exact read-only binding/sequence proposal while continuing
already admitted artifact tests/recovery work. Actual edits to safe-FS wait for an
explicit follow-up perimeter covering the verified sequence. No owner needs to
choose low-level APIs now; if real compatibility/security tradeoff remains after
this design, surface that bounded choice. Do not ship a Linux-only bypass in the
ordinary cached route as universal support. No source commit/publication, cleanup,
credential/runtime changes or whole USER-PROJECT acceptance follows.
