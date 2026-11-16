# UP-C1-LEASE-01 — exact retained-root lease handoff

Date: 2026-11-16. First narrow C1 prerequisite, not slot/ignore/stash completion.
Builder owns coding/tests under relayed direct USER mandate; architect owns review.

## Exact inputs and outputs

Base `6a83be1dd35d7bec2de4911ee375c2be3cb8686a` plus exact committed delivery of
this brief. Fresh assigned isolated source, no candidate overlays or new dependency.
Read-only LEASE-PROPOSAL.md and NEGATIVE-GATES.md under
`/fast/tmp/up-c1-input-census-ei2_l7_p` are design evidence, not source overlays.
The fifteen-file census manifest ebb99cd1 is NOT input admission. Do not copy dirty
lease/root/epoch/metadata/artifact/privacy sources. Existing b444 lease architectural
intent remains compatible; this brief chooses a base-authored minimal implementation.

Allowed writes ONLY:
- `crates/vibe-lifecycle/src/lease.rs`;
- `crates/vibe-lifecycle/src/lease/retained_tests.rs` (new focused tests).

No Cargo, public CLI/wire/schema, workspace/orchestrator, safefs API or other owner
changes. No source commit, integration, publication, credentials, host mutation,
provider call, cleanup or Git metadata repair authorized.

## Implementation contract

1. Change private LifecycleLease.project storage from Project to Arc<Project>.
   Existing lease stays non-Clone/non-Copy with one actual LockGuard; Arc over the
   whole lease shares that acquisition, never a clone/reacquisition of OS guard.
2. Existing acquire(root:&Path) preserves its current opening and typed error
   mappings, then delegates to acquire_retained(Arc::new(opened_project)).
3. Add acquire_retained(project:Arc<Project>) using the EXACT supplied capability
   for incumbent try_lock(LOCK_NAME). No Project::open(display_path), root discovery,
   cwd fallback, canonicalize or ambient path-based lock. Preserve current lock
   filename/location, nonblocking behavior, Busy/Directory mappings and drop law.
4. Add read-only retained_project(&self)->&Arc<Project>. Existing crate-private
   project()->&Project remains behaviorally unchanged via deref. Accessor proves
   shared capability identity, NOT lease ownership for holders of a Project alone.
   Root paths remain diagnostics, not proof that selected pathname still names it.
5. Moving selected root may leave held capability at the moved tree. This atom must
   not lock a replacement tree; it also must NOT claim selected-path health/success
   admission. Caller later performs original component/epoch revalidation around
   acquisition. No generic root-only bypass of that future caller gate.

## Dependency-correct later C1 authority boundary

vibe-lifecycle depends on vibe-workspace; workspace must not depend back on lifecycle.
Later workspace defines a trusted authority port and upper layer holds real lease,
original admission revalidator and exact shared root. Arc equality binds its supplied
root, while erased owner retains lease through inverse/recovery. This is a trusted
host injection boundary, not unforgeable OS proof against arbitrary implementers.
No such port/adapter or production cooperative API is implemented in this atom.

## Focused and adjacent gates

- Arc::ptr_eq accessor/supplied Project, without re-opening or changing identity.
- Legacy acquire still locks, contends and releases with existing error mapping.
- Retained and legacy same-root acquisitions contend; no nested lock reentry.
- Move selected root, place replacement and sentinel: retained acquisition must not
  create/lock replacement .vibe; inspect original retained capability's lock facts.
  Do not call that selected-path admission success.
- Dropping original Project Arc does not release lease. Sharing Arc<LifecycleLease>
  keeps contention until last owner drops, then reacquisition works.
- Absent/unsafe legacy root behavior unchanged. No mutating acquisition on preview/
  read introduced; no public command wiring in this atom.

Use isolated synthetic directories only. Run focused tests with nonzero counts,
adjacent existing lease tests, strict lifecycle lint/fmt/Conform. Report failures,
platform assumptions and unrun Windows separately. No broad suite by habit.
Return immutable exact two-file manifest/diff, base/binding, actual commands/counts.
Architect review required before declaring prerequisite cleared or selecting it as
later input. This is not complete C1 or whole USER acceptance. Next substantive C1
atom must bind production authority/typed consuming inverse and source-copy/stash/
ignore owners together; do not quietly import dirty root/epoch or overwrite already
reviewed artifact recovery carriers. C2 boot, C3 native effects, C4 public routes
remain subsequent campaign obligations.
