# UP-ROOT-CAPABILITY-01 — retained authority through lifecycle effects

Date: 2026-11-16. Architect implementation brief; not a worker packet.
Compatibility baseline `6a83be1dd35d7bec2de4911ee375c2be3cb8686a`.
Contract predecessor at authoring `aefc4302136b7d0b2b40685ac77bd5a7ee45fb4b`;
exact committed delivery hash is sent separately. Builder binds actual assigned
root/branch and each atom's exact admitted product inputs before dispatch.

This addresses the current routing prerequisite, not later personal-layer loading.
No public personal lifecycle is activated until the relevant effect consumers pass
integrated review. Existing internal empty/fresh refusals are unfinished guards,
not a permanent restriction or accepted implementation of supported modes.

## Mandate, authority and input gates

PROP-074 personal root admission, ordinary project/workspace behavior, lifecycle
rollback/cooperative stop and routing-preflight rules govern. Builder owns coding,
worker execution, integration and tests; architect owns contract and review. Current
local execution policy applies separately. No source commit, push/publication,
actual user-state mutation, credentials or toolchain changes are authorized here.

Previously reviewed root/epoch/receiver deltas remain uncommitted. Technical review
is not worker input admission. The one-atom nine-file source-owner exception does
NOT admit these lifecycle prerequisites. Before each coding dispatch deliver exact
committed base plus an authorized frozen prerequisite inventory; if this requires
another candidate-input exception, obtain the precise brief amendment first.
Independent read-only inspection/tests in the assigned tree retain normal authority.
No moving dirty snapshot, private runtime state or peer-tree writes.

## Invariants and minimal design

1. Personal execution retains its admitted filesystem-root Project capability and
   component identities through every effect. Paths are diagnostics, not a renewed
   authority grant. Opening Project::open(display_path) after a precheck is not a
   retained handoff. Member/slot/boot descendants are validated root-relative
   capabilities of that same authority, not arbitrary absolute paths.
2. Use existing Project/Pinned no-follow APIs, identity tokens, bounded stable reads,
   owned publication receipts, guarded rename/remove and existing transaction
   machinery. No new generic filesystem framework, permission broker, solver or
   public safefs API expansion. A missing essential primitive is an explicit
   blocker/perimeter request, not a custom ambient syscall workaround.
3. Legacy discovery and ordinary accepted behavior remain unchanged by default.
   Personal explicit-root mode must be typed and mandatory where selected; missing
   authority refuses, never falls back to ambient legacy. Preserve all supported
   materialization/representation/native modes; do not disable a mode to avoid work.
4. Distinguish source authority from destination authority. External registry/cache
   input uses its own admitted source owner; target writes and recovery use personal
   root authority. Cache hash metadata is not actual payload verification. This
   brief does not solve the independent retained-source/materializer contract.
5. Root movement observed at a boundary refuses normal execution and does not
   redirect into a replacement root. Retained capability may identify an old moved
   tree; that is not permission to report success at the selected path. Preserve
   uncertain effects/receipts and avoid speculative cleanup. Observations do not
   claim kernel CAS against arbitrary uncooperative writers.
6. One manifest undo owner and operation-owned epoch remain as reviewed. Slot/boot/
   lock/native participants preserve actual ordering, evidence and typed errors.
   Fallible finalization occurs before evidence disarm; postcommit failure does not
   undo durable state. Root authority alone does not establish whole-operation undo.

## Actual owner seams and bounded write perimeter

Implement meaningful atoms with exact per-atom file lists, not one giant rewrite:

### C1 — Slot, ignore and stash destination capability

Allowed workspace owners: `crates/vibe-workspace/src/vibedeps.rs`, focused modules
under `vibedeps/` including build_ignore; `install/checked_placement.rs` and focused
children; `install/rollback.rs` and focused children; install apply/composition only
for passing retained authority. Relevant existing install/orchestrator ports and
owner fields may carry the capability without changing source/solver semantics.

Map each mkdir/copy/rename/remove to source authority, retained target parent,
expected identity and failure/undo evidence. Use private same-filesystem stash
allocated relative to retained authority, with exclusive creation and retained
identity; no ambient tempfile path that can escape after root replacement. Guard
both source/destination names on rename and refuse cross-filesystem/unsupported
cases explicitly under existing laws. Do not recursively remove unverified trees.
A general arbitrary tree copier is not assumed available: compose existing checked
copy/identity mechanisms only after naming their actual safe seam. Return a precise
primitive gap before inventing a new one.

### C2 — Boot publication and inverse capability

Allowed workspace owners: `boot_artifacts/inverse.rs`, existing boot transaction/
publication/redirect owners and focused tests; `install/bootgen/` native_managed,
replay_prepare/replay_publish; `install/native.rs`; `boot_artifacts/native_managed/`
explicit/write/replay/control as needed for typed root propagation. Existing
system_prompt publication owners may receive retained descendant authority where
boot transaction already owns them; no layered personal boot/composition feature
is implemented in this atom.

BootImages must capture/settle/restore through the same admitted descendants and
actual before/after evidence, including node entrypoints, static/index, existing
prefix artifacts and unit boots. Preserve foreign edits and pending transaction
journals. Identical bytes at a foreign replacement inode are not generic ownership
proof. Existing managed-block rules still apply; do not overwrite authored text
outside the owned block. Retain the boot inverse until all fallible metadata
finalization succeeds, then dispose of it through existing explicit commit behavior.
Typed boot-restoration failure must survive error wrapping and block restored status.

### C3 — Native load-image and helper effect boundary

Additional narrow lifecycle owners now admitted: `crates/vibe-lifecycle/src/native/`
`path.rs`, `compiler/admission.rs`, focused compiler/manager/runtime/mechanism call
sites and focused tests ONLY to propagate retained root/load-image authority and
report existing native effects correctly. Existing `native/helper.rs` cancellation
marking may be adjusted only if necessary to preserve its current unowned-effect
law; no helper protocol, permission, executable selection or compiler-policy redesign.
List exact necessary files/signatures before implementation, rather than treating
this paragraph as authorization to edit all native code.

Actual inventory: Published artifact access calls publish_load_image; that path
canonicalizes selected root and creates `.vibe/native-load/...`. Collect policy can
invoke an available compiler; it only treats certain unavailable builds as pending.
Helper marks unowned effects before plugin exchange. Therefore BootImages does not
cover all native generation effects. Use retained root/descendant capability for
load-image publication, preserve digest/identity/mode and existing immutable-load
semantics, and define whether each infrastructure artifact is an owned undoable
write or a retained durable cache effect. Do not relabel cached/plugin effects as
rolled back merely because boot/manifest restored. Do not invent undo of arbitrary
plugin work; preserve cancellation native/unowned facts and honest recovery status.
Native pending/replay continuation has an explicit owner and retention obligation.
No real plugin/provider execution is required for synthetic gates.

### C4 — Connect supported lifecycle branches

Only after C1-C3 relevant paths are cleared, use existing orchestrator/install
owners to connect Ready and empty/fresh/source/prerequisite/resume paths. Preserve
workspace members and deterministic explicit selected personal root. Do not start
a second manifest epoch, double-undo a participant or lose typed recovery through
parking/error rendering. Read/preview must not acquire mutating lease or create
outputs. Existing preview source/cache laws remain separate; no implicit init.
Public CLI wiring is still the later complete routing acceptance boundary.

## Verification before any public activation

Use isolated synthetic roots and deterministic interleavings, with no real user
settings, credentials, publication or production process changes.

- Root/ancestor replacement and symlink/reparse substitution at each admission,
  mkdir/copy, rename, prepublication, settle and recovery seam. Replacement-tree
  sentinels remain unchanged. Detect selected-path movement without following it.
- Same-byte foreign inode, hardlink, unexpected entry and unsupported type cases;
  protect external source/destination sentinels. Copy verifies actual prepared bytes
  against retained source recipe, not only caller-supplied source_hash metadata.
- Existing member workspace under a synthetic enclosing workspace, shared unit
  boots and independent node paths. No enclosing discovery or cwd selection.
- Partial slot placement, stash creation, boot publication and failed undo; preserve
  receipts/backups and distinguish known changes/uncertainty. No speculative cleanup.
- Native available compiler under collect mode, load image before helper exchange,
  helper unowned effects, pending/replay continuation and refused setup. Synthetic
  helpers assert actual path/argv/effect markers, not model self-report.
- Cancellation before/after effects, finalization failure, boot/slot/manifest undo
  conflict and postcommit failure. Never claim restored while native/unowned or
  infrastructure effects remain unclassified. Actual wait/stop semantics unchanged.
- Legacy adjacent tests for every modified owner and existing valid modes. Focused
  tests must select nonzero counts; report platforms/types unrun honestly.

Each substantial atom returns exact frozen manifest/diff, input/base, all affected
consumers and actual gates. Architect reviews safety and cross-cutting consequences;
worker/test success alone is not acceptance. Reuse unchanged valid evidence.

## Safe stop and remaining campaign

Stop with protected candidate and precise primitive/ownership blocker if existing
APIs cannot enforce a required boundary. Do not broaden mounts, follow a symlink,
weaken path policy, repair/delete user state or reject supported modes as a shortcut.
This prerequisite does not complete retained-source checked materialization,
application/remap routing, public CLI, late user boot, privacy exporters, overrides,
memory, checkers, tree or final whole-campaign proof. BOOT-PRIVACY-DESIGN.md is a
separate unimplemented proposal, not permission to bypass this prerequisite.
