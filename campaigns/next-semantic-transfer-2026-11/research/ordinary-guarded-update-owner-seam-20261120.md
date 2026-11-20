# Guarded-leaf update — pre-writer ownership boundary

Contract NST-ORDINARY-GUARDED-UPDATE-FIRST-01.md (a50810b1a61caf631c6735f8c439ab995eb1de60, integrated e84fc7f91). Source review only; no update/rollback writer behavior changed. Accepted dormant install qualification9e3d75cf2 remains independent.

## Existing complete-graph/publication owners

`update/scoped.rs` solves the actual Roots::All graph and compares selected package membership to old_lock. Its existing membership refusal is the first public barrier for the qualified optional-leaf removal. `stage::prepare_records` clones the old lock and replaces/adds prospective rows; it does not delete the unreachable row. `stage::preflight_world` feeds retained rows and selected replacements to provisional_world, so merely removing the guard would retain the old leaf in the boot/publication world. A supported reconciliation must carry one exact admitted removal through graph, record and full-world preparation, not silently filter unrelated history.

`prune_superseded` only visits re-resolved packages with changed versions. It has no unreachable-leaf set, and is not a generic orphan-removal owner. Whole-install prune_stale_slots walks every unkept directory, so invoking it here would widen deletion beyond the selected proved leaf and is not an acceptable shortcut.

## Missing checked removal/inverse

`InstallRollback::remove` calls move_aside with an ambient pathname. That path has no caller-bound ownership witness or complete expected tree validation. Errors from symlink_metadata are collapsed into a Created step. Existing `Step::MovedAside` inverse calls remove_any(destination) unconditionally before renaming the stash back. A new foreign replacement at the slot name could therefore be erased during rollback. This legacy primitive cannot establish the new guarded-update contract's foreign/modified-slot preservation.

`CheckedPlacement` has an after-tree comparison and conflict refusal, but no checked removal counterpart. Its internal Tree is a complete cooperative observation, not a public deletion authorization. `vibedeps::verify_recorded_files` explicitly ignores unrecorded paths; record source_hash and named file hashes do not authorize discarding a foreign extra file. A whole-slot removal needs a complete admitted census including only proved payload/owned metadata, plus stable directory/member identity and no-follow checks.

A narrow existing-owner extension is needed before writer changes: capture one admitted local-copy slot against exact retained source/record and complete allowed tree; move only that checked slot into the existing same-volume stash; restore it only while the destination is absent, otherwise preserve the foreign occupant and report retained recovery backup. Existing restore already calls stash.keep() on inverse errors, which can retain recovery evidence. Do not retrofit permissive removal of unrelated slots or claim external hook side effects are reversible.

## Linux no-replace capability boundary

The existing safefs `Pinned::rename_child_noreplace_to` is explicitly Windows-only. Its non-Windows platform fallback returns Unsupported; there is no existing Linux renameat2 owner in the crates. The new checked-removal contract therefore cannot be satisfied on Linux by merely composing the exposed move API. An existence check followed by ordinary rename is not a no-replace claim. Before implementing the Linux writer, a scoped safefs platform extension and its exact cooperative-versus-hostile mutation semantics must be agreed. Retained parent handles plus Linux RENAME_NOREPLACE can protect the destination, but ordinary pathname-based source rename does not magically become source-handle-atomic; that limit must not be hidden.

Architect clarification for the expectation census: begin with untransformed plain copy only, no overlays/converter/in-place. Reproduce through the trusted materializer against the validated old lock source hash. Admit exactly resulting file paths/bytes plus verified canonical slot record, and only parent directory components of those paths. Unknown files and empty directories refuse; boot/metadata are not whitelisted by directory name. No writer change is authorized by a self-snapshot alone.

## Required integration after the primitive is agreed

Keep generic membership refusal for unsupported changes. Classify only one guarded optional leaf with no outgoing dependencies, complete old/new reachable graphs and unchanged admitted root/source meaning. Another root retaining the leaf must prevent removal. Newly reachable leaf uses current resolver/fetch, not old cache inference. Existing default-review meaning and exact update constraints remain. Thread the selected removal through planned lock/configurations/app requirements/full_world and measured lifecycle; perform it only inside the same rollback region as placement/boot/record publication.

Required public tests remain active/off/on, retained-vs-removed answers, no-op/cold replay, retaining root, edited/foreign slot refusal, staged/publication failures with restored lock/slot/boot and ordinary version movement. Exact hook history is measured separately; irreversible hook effects are not promised rollback. This review is not implementation acceptance or a new prerequisite campaign. Architectural scope decision requested; no generic guard or writer was changed.
