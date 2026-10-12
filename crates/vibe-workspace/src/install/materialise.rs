//! Dependency slot materialisation with selected representation.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#install");

use super::*;

/// Materialise a resolution and run `pre-install` after a nonempty payload
/// diff (PROP-009 §2.7, PROP-020 §2.1). Injectable seams cover hook execution
/// and the PROP-011 integrity check. Identity-current slots may be skipped;
/// verification divergence reconciles and warns. Reconciliation reporting is
/// independent: identity-only work is materialised but runs no hook. A failed
/// `pre-install` removes the offending slot and aborts (PROP-020 §2.5).
pub(super) fn materialise_resolution_with_spec_format(
    workspace_root: &Path,
    resolution: &[ResolvedDep],
    options: MaterialiseOptions<'_>,
) -> Result<Materialised, WorkspaceError> {
    let MaterialiseOptions {
        slot_integrity,
        spec_format,
        slot_verifier,
        lifecycle,
        progress,
    } = options;
    let mut materialised = Vec::new();
    let mut skipped = Vec::new();
    let mut integrity_warnings = Vec::new();
    let mut conversion_counts = AppliedConversionCounts::default();
    let mut conversion_failures = Vec::new();
    let mut post_install_deps = Vec::new();
    let mut hook_reports = Vec::new();
    let mut pre_install = PreInstallPlan::new(&lifecycle, workspace_root);
    let task = progress.task("Materialising dependency slots");
    task.set_progress(0, Some(resolution.len() as u64), "packages");
    for dep in resolution {
        let package_task = task.progress().task(format!(
            "Materialising {}/{}@{}",
            dep.group, dep.name, dep.version
        ));
        // PROP-022 §2.4 — an in-place package is a project-local git working
        // tree in an unversioned slot. Move the fetched clone (with its
        // `.git`) into the slot instead of the per-file `copy`, and
        // `.gitignore` it (not vendored, §2.7).
        if is_in_place(dep) {
            if spec_format.is_transformed() {
                return Err(WorkspaceError::SpecMaterialization {
                    path: dep.content_dir.clone(),
                    reason: "transformed spec formats are unavailable for in-place packages"
                        .to_string(),
                });
            }
            let rel = vibedeps::in_place_slot_rel_path(&dep.group, &dep.name);
            let slot_abs = vibedeps::in_place_slot_abs_path(workspace_root, &dep.group, &dep.name);
            // The install layer may have already placed the slot directly — an
            // incremental in-place update (PROP-022 §2.4), `git fetch`-ed onto
            // the existing `.git` rather than re-cloned — signalled by the
            // dep's `content_dir` BEING the slot. Then there is no clone to
            // move; the slot is already current and we only run the hook.
            let already_placed = dep.content_dir == slot_abs;
            if !already_placed
                && vibedeps::is_in_place_slot(workspace_root, &dep.group, &dep.name)
                && slot_integrity == SlotIntegrity::TrustPresence
            {
                skipped.push(rel);
                package_task.skip("slot already current");
                task.set_progress(
                    (materialised.len() + skipped.len()) as u64,
                    Some(resolution.len() as u64),
                    "packages",
                );
                continue;
            }
            if !already_placed {
                required_source_hash(dep)?;
                vibedeps::materialise_in_place(
                    workspace_root,
                    &dep.group,
                    &dep.name,
                    &dep.content_dir,
                )?;
                vibedeps::ensure_gitignored(workspace_root, &rel)?;
            }
            let changed = if already_placed {
                dep.in_place_changed.unwrap_or(true)
            } else {
                true
            };
            if !changed {
                skipped.push(rel);
                package_task.skip("in-place source unchanged");
                task.set_progress(
                    (materialised.len() + skipped.len()) as u64,
                    Some(resolution.len() as u64),
                    "packages",
                );
                continue;
            }
            // In-place keeps its git-native reset/eligibility semantics.
            post_install_deps.push(dep.clone());
            pre_install.run_or_defer(dep, &mut hook_reports)?;
            materialised.push(rel);
            package_task.finish();
            task.set_progress(
                (materialised.len() + skipped.len()) as u64,
                Some(resolution.len() as u64),
                "packages",
            );
            continue;
        }
        let slot = vibedeps::slot_rel_path(&dep.group, &dep.name, &dep.version);
        let present =
            vibedeps::is_materialised(workspace_root, &dep.group, &dep.name, &dep.version);
        let slot_abs = vibedeps::slot_abs_path(workspace_root, &dep.group, &dep.name, &dep.version);
        // PROP-045: representation freshness precedes every presence-based
        // trust decision. A changed project/user setting always re-materialises.
        let format_current = present && vibedeps::format_is_current(&slot_abs, spec_format);
        // Mutable local `file://` sources (PROP-011 §2.6) cannot use version
        // presence as identity: the author may edit the source in place. The
        // fresh LocalRegistry fetch already put the current tree hash on the
        // resolution, so a valid record carrying that same hash earns
        // eligibility without another tree walk here. Missing, malformed, or
        // mismatched records fall through to reconciliation; its strict record
        // reader turns malformed state into a hard error instead of wiping it.
        let source_identity_current = present
            && (!dep.source_mutable
                || (format_current
                    && vibedeps::read_slot_record(&slot_abs).is_ok_and(|record| {
                        required_source_hash(dep).is_ok_and(|hash| record.source_hash == *hash)
                    })));

        // Every identity-current slot follows the configured integrity
        // strategy (§2.3/§5.2): `trust-presence` accepts it outright; `verify`
        // still spot-checks payload through the caller's seam, so local drift
        // heals even when the mutable source itself is unchanged. No verifier
        // (or an unverifiable slot) keeps the shipped rematerialise discipline.
        let trusted = source_identity_current
            && match slot_integrity {
                SlotIntegrity::TrustPresence => format_current,
                SlotIntegrity::Verify => match slot_verifier {
                    None => false,
                    Some(verifier) => {
                        match verifier.verify_slot_for_format(dep, &slot_abs, spec_format) {
                            SlotCheck::Verified => true,
                            SlotCheck::Diverged { expected, actual } => {
                                integrity_warnings.push(format!(
                                    "{}/{}@{}: vibedeps slot hashes {actual}, locked hash is \
                                     {expected} — re-materialising",
                                    dep.group, dep.name, dep.version
                                ));
                                false
                            }
                            SlotCheck::DivergedDetail { reason } => {
                                integrity_warnings.push(format!(
                                    "{}/{}@{}: {reason} — re-materialising",
                                    dep.group, dep.name, dep.version
                                ));
                                false
                            }
                            SlotCheck::Unverifiable => false,
                        }
                    }
                },
            };
        if trusted {
            skipped.push(slot);
            package_task.skip("slot already current");
            task.set_progress(
                (materialised.len() + skipped.len()) as u64,
                Some(resolution.len() as u64),
                "packages",
            );
            continue;
        }
        let source_hash = required_source_hash(dep)?;
        let materialise_report = if let Some(prepared) =
            slot_verifier.and_then(|verifier| verifier.prepared_transformed_slot(dep))
        {
            let slot_abs =
                vibedeps::slot_abs_path(workspace_root, &dep.group, &dep.name, &dep.version);
            if !prepared.matches_selection(&slot_abs, &dep.content_dir, spec_format, source_hash) {
                return Err(WorkspaceError::SpecMaterialization {
                    path: slot_abs,
                    reason: "prepared conversion differs from selected resolution".into(),
                });
            }
            prepared.publish_report()?
        } else {
            vibedeps::materialise_with_spec_format_report(
                workspace_root,
                &dep.group,
                &dep.name,
                &dep.version,
                &dep.content_dir,
                copy_mode_for(&dep.manifest),
                spec_format,
                source_hash,
            )?
        };
        conversion_counts.converted += materialise_report.converted;
        conversion_counts.copied_unconverted += materialise_report.copied_unconverted.len();
        conversion_failures.extend(materialise_report.copied_unconverted.iter().map(
            |(source, reason)| AppliedConversionFailure {
                slot: slot.clone(),
                source: source.clone(),
                reason: reason.clone(),
            },
        ));
        if materialise_report.payload_changed() {
            post_install_deps.push(dep.clone());
            pre_install.run_or_defer(dep, &mut hook_reports)?;
        }
        materialised.push(slot);
        package_task.finish();
        task.set_progress(
            (materialised.len() + skipped.len()) as u64,
            Some(resolution.len() as u64),
            "packages",
        );
    }
    pre_install.dispatch(&materialised, &skipped)?;
    task.finish();
    Ok(Materialised {
        materialised,
        skipped,
        integrity_warnings,
        conversion_counts,
        conversion_failures,
        post_install_deps,
        hook_reports,
    })
}
