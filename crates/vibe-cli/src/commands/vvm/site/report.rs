//! The `vibe self apply` report (`vibe-self-apply/1`) and its text form
//! (`PROP-019#site-profile`): what changed in which file, the effective
//! installation, where a person puts releases and payloads, the PATH step.
//!
//! The document itself is the registered wire type
//! `vibe_wire::generated::self_apply_report` (`cli-self-apply-report`); this
//! module owns the projection of the command's own observations into it and
//! the human rendering of the result.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile");

use vibe_wire::generated::self_apply_report::{
    ConsentSource, EnvBlockStatus, EnvEffectiveness, EnvObservationStatus,
    EnvPathEntryStatus, EnvProbeStatus, EnvTargetKind, EnvironmentObservation, LoginShell,
    PathFixOutcome, PathReport, PxStep, SelfApplyReport, SkipReason, WhereToPut,
};

use crate::commands::vvm::bundle::release_source::kind_label;
use crate::commands::vvm::env::{self, PathFixObservation};
use crate::output;

#[path = "report/effective.rs"]
pub(super) mod effective;

/// The JSON protocol name of the report.
pub(super) const PROTOCOL: &str = "vibe-self-apply/1";

/// The command name every form of the report carries.
pub(super) const COMMAND: &str = "self:apply";

/// Project a performed PATH step into the report's `path` member: the
/// observation's own members, with the planning ones absent.
pub(super) fn path_done(observation: PathFixObservation) -> PathReport {
    PathReport {
        shim_dir: observation.shim_dir,
        outcome: Some(match observation.outcome {
            env::PathFixOutcome::AlreadyOnPath => PathFixOutcome::AlreadyOnPath,
            env::PathFixOutcome::Applied => PathFixOutcome::Applied,
            env::PathFixOutcome::Skipped => PathFixOutcome::Skipped,
        }),
        was_on_durable_path: Some(observation.was_on_durable_path),
        on_durable_path: Some(observation.on_durable_path),
        planned: None,
        consent: observation.consent.map(consent_source),
        skip_reason: observation.skip_reason.map(skip_reason),
        changed: Some(observation.changed),
        activation_hint: observation.activation_hint,
        fix_hint: observation.fix_hint,
        durable_environment: Some(environment(observation.durable_environment)),
        target: None,
    }
}

/// The wire value of one consent source.
pub(super) fn consent_source(source: env::ConsentSource) -> ConsentSource {
    match source {
        env::ConsentSource::Flag => ConsentSource::Flag,
        env::ConsentSource::Profile => ConsentSource::Profile,
        env::ConsentSource::Interactive => ConsentSource::Interactive,
    }
}

/// The wire value of one skip reason.
pub(super) fn skip_reason(reason: env::SkipReason) -> SkipReason {
    match reason {
        env::SkipReason::FlagDeclined => SkipReason::FlagDeclined,
        env::SkipReason::ProfileNever => SkipReason::ProfileNever,
        env::SkipReason::NotInteractive => SkipReason::NotInteractive,
        env::SkipReason::Declined => SkipReason::Declined,
    }
}

/// The wire value of one performed PATH outcome, spelled for the receipt
/// and for a human.
pub(super) fn outcome_label(outcome: &PathFixOutcome) -> &'static str {
    match outcome {
        PathFixOutcome::AlreadyOnPath => "already_on_path",
        PathFixOutcome::Applied => "applied",
        PathFixOutcome::Skipped => "skipped",
    }
}

/// The wire form of the durable-environment observation the PATH step read.
fn environment(observation: env::EnvironmentObservation) -> EnvironmentObservation {
    EnvironmentObservation {
        target_kind: match observation.target_kind {
            env::TargetKind::RcFile => EnvTargetKind::RcFile,
            env::TargetKind::WindowsRegistry => EnvTargetKind::WindowsRegistry,
            env::TargetKind::Unavailable => EnvTargetKind::Unavailable,
        },
        target: observation.target,
        // `and_then`, not `map`: cmd is not a login shell with an rc file
        // this step could have read, so there is nothing to report for it.
        shell: observation.shell.and_then(|shell| match shell {
            env::Shell::Bash => Some(LoginShell::Bash),
            env::Shell::Zsh => Some(LoginShell::Zsh),
            env::Shell::Fish => Some(LoginShell::Fish),
            env::Shell::Pwsh => Some(LoginShell::Pwsh),
            env::Shell::Posix => Some(LoginShell::Posix),
            env::Shell::Cmd => None,
        }),
        observation: match observation.observation {
            env::ObservationStatus::Present => EnvObservationStatus::Present,
            env::ObservationStatus::Missing => EnvObservationStatus::Missing,
            env::ObservationStatus::Unavailable => EnvObservationStatus::Unavailable,
            env::ObservationStatus::Malformed => EnvObservationStatus::Malformed,
        },
        managed_block: match observation.managed_block {
            env::BlockStatus::Present => EnvBlockStatus::Present,
            env::BlockStatus::Absent => EnvBlockStatus::Absent,
            env::BlockStatus::Malformed => EnvBlockStatus::Malformed,
            env::BlockStatus::NotApplicable => EnvBlockStatus::NotApplicable,
            env::BlockStatus::Unknown => EnvBlockStatus::Unknown,
        },
        path_entry: match observation.path_entry {
            env::PathEntryStatus::Correct => EnvPathEntryStatus::Correct,
            env::PathEntryStatus::Missing => EnvPathEntryStatus::Missing,
            env::PathEntryStatus::Different => EnvPathEntryStatus::Different,
            env::PathEntryStatus::Ambiguous => EnvPathEntryStatus::Ambiguous,
            env::PathEntryStatus::Unknown => EnvPathEntryStatus::Unknown,
        },
        login_probe: match observation.login_probe {
            env::ProbeStatus::NotRun => EnvProbeStatus::NotRun,
            env::ProbeStatus::Ran => EnvProbeStatus::Ran,
            env::ProbeStatus::Failed => EnvProbeStatus::Failed,
            env::ProbeStatus::TimedOut => EnvProbeStatus::TimedOut,
            env::ProbeStatus::Unsupported => EnvProbeStatus::Unsupported,
        },
        login_effectiveness: match observation.login_effectiveness {
            env::Effectiveness::Unknown => EnvEffectiveness::Unknown,
            env::Effectiveness::Effective => EnvEffectiveness::Effective,
            env::Effectiveness::NotEffective => EnvEffectiveness::NotEffective,
        },
        error: observation.error,
    }
}

/// Emit the report: the document itself under `--json`, its text form and
/// the diagnostics otherwise.
pub(super) fn emit(report: &SelfApplyReport, ctx: &output::Context) -> anyhow::Result<()> {
    if ctx.is_json() {
        return ctx.emit_json(report);
    }
    for warning in &report.warnings {
        ctx.diagnostic(warning);
    }
    ctx.summary(&text(report));
    Ok(())
}

/// The wire value of one action, spelled for a human and for the receipt.
fn action_label(report: &SelfApplyReport) -> &'static str {
    use vibe_wire::generated::self_apply_report::ApplyAction;
    match report.action {
        ApplyAction::Apply => "apply",
        ApplyAction::DryRun => "dry-run",
        ApplyAction::Remove => "remove",
        ApplyAction::Show => "show",
    }
}

fn text(report: &SelfApplyReport) -> String {
    let mut out = String::new();
    let line = |out: &mut String, text: &str| {
        out.push_str(text);
        out.push('\n');
    };
    let action = action_label(report);
    match (&report.profile, action) {
        (Some(profile), action) => {
            let name = profile.name.as_deref().unwrap_or("(unnamed)");
            let verb = match (action, report.changed) {
                ("dry-run", _) => "would be applied",
                ("apply", true) => "applied",
                ("apply", false) => "already applied — nothing changed",
                ("remove", _) => "removed",
                _ => "in effect",
            };
            line(
                &mut out,
                &format!(
                    "site profile `{name}` {verb} (from {}, sha256 {})",
                    profile.source,
                    short(&profile.sha256)
                ),
            );
        }
        (None, "remove") if !report.changed => line(&mut out, "no site profile is applied"),
        (None, "show") => line(&mut out, "no site profile is applied"),
        (None, _) => {}
    }
    for file in &report.files {
        line(
            &mut out,
            &format!("  {:<9} {}", change_label(file), file.path),
        );
        if let Some(diff) = &file.diff {
            for row in diff.lines() {
                line(&mut out, &format!("    {row}"));
            }
        }
    }
    if let Some(imported) = &report.imported {
        let verb = if imported.planned {
            "would import"
        } else {
            "imported"
        };
        line(
            &mut out,
            &format!(
                "{verb} the running vibe `{}` as {}",
                imported.executable, imported.selector
            ),
        );
    }
    if let Some(px) = &report.px {
        px_text(px, &mut out);
    }
    if let Some(effective) = &report.effective {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&self::effective::sections(
            effective,
            crate::output::language::viewport(),
            crate::output::language::rule_width(),
        ));
        out.push_str("\n\n");
    }
    if let Some(where_to_put) = &report.where_ {
        where_text(where_to_put, &mut out);
    }
    if let Some(path) = &report.path {
        path_text(path, &mut out);
    }
    for step in &report.next_steps {
        line(&mut out, &format!("next: {step}"));
    }
    out.trim_end().to_owned()
}

/// The px companion step (`PROP-019#companion-tools`): one line, and for
/// a px the profile needs and the machine has not got, the repair — the
/// line the operator missed when the only trace was a grey `skipped`.
fn px_text(px: &PxStep, out: &mut String) {
    use vibe_wire::generated::self_apply_report::PxOutcome;
    let line = match (&px.outcome, px.planned) {
        (PxOutcome::NotNeeded, _) => return,
        (PxOutcome::AlreadyInstalled, _) => px
            .program
            .as_deref()
            .map(|program| format!("px: already installed (`{program}`)")),
        (PxOutcome::Installed, false) => px
            .program
            .as_deref()
            .map(|program| format!("px: installed `{program}`")),
        (PxOutcome::Installed, true) => Some(format!(
            "px: would be installed into `{}`",
            px.store.clone()
        )),
        (PxOutcome::Missing | PxOutcome::NoTarget, planned) => Some(format!(
            "px: NOT INSTALLED{} — {}",
            if planned { " (and would not be)" } else { "" },
            px.fix.as_deref().unwrap_or("px is needed and is not there")
        )),
    };
    if let Some(line) = line {
        out.push_str(&line);
        out.push('\n');
    }
}

/// The wire value of one file change, spelled for a human.
fn change_label(file: &vibe_wire::generated::self_apply_report::FileChange) -> &'static str {
    use vibe_wire::generated::self_apply_report::FileChangeKind;
    match file.change {
        FileChangeKind::Create => "create",
        FileChangeKind::Update => "update",
        FileChangeKind::Unchanged => "unchanged",
        FileChangeKind::Delete => "delete",
    }
}

fn short(sha: &str) -> &str {
    sha.get(..12).unwrap_or(sha)
}

fn where_text(where_to_put: &WhereToPut, out: &mut String) {
    if let Some(first) = where_to_put.releases.first() {
        out.push_str(&format!(
            "put vibe releases into `{}` ({}), one directory per release, holding exactly the release page's files:\n",
            first.location,
            kind_label(first.source.clone())
        ));
        for versioned in &where_to_put.releases {
            if let Some(dir) = &versioned.example_dir {
                out.push_str(&format!("  {dir}/\n"));
                for file in &versioned.files {
                    out.push_str(&format!("    {file}\n"));
                }
            }
        }
        out.push_str(
            "  (a bare `vibevm-<version>-<target>.zip` at the top is accepted too; \
             `vibe self releases` lists what is there)\n",
        );
    }
    if let Some(packages) = &where_to_put.packages {
        out.push_str(&format!(
            "put package payloads into `{}`; an input `F` (sha256 `D`) of package `G/N@V` is looked for at:\n",
            packages.dir
        ));
        for rung in &packages.layout {
            out.push_str(&format!("  {rung}\n"));
        }
        if let Some(example) = packages.example.first() {
            out.push_str(&format!("  e.g. {example}\n"));
        }
        out.push_str(
            "  (`vibe artifacts where <package>` names the exact files a package needs)\n",
        );
    }
}

/// The PATH step: a performed step carries `outcome`, a planned one
/// `planned`; a document with neither says nothing and prints nothing.
fn path_text(path: &PathReport, out: &mut String) {
    if let Some(outcome) = &path.outcome {
        let what = match outcome {
            PathFixOutcome::AlreadyOnPath => {
                format!("PATH: `{}` is already on it", path.shim_dir)
            }
            PathFixOutcome::Applied => format!("PATH: put `{}` on it", path.shim_dir),
            PathFixOutcome::Skipped => {
                format!("PATH: left as it is (`{}` not on it)", path.shim_dir)
            }
        };
        out.push_str(&what);
        out.push('\n');
        for hint in [&path.activation_hint, &path.fix_hint]
            .into_iter()
            .flatten()
        {
            out.push_str(&format!("  {hint}\n"));
        }
        return;
    }
    if let Some(planned) = &path.planned {
        out.push_str(&format!(
            "PATH: {} `{}`{}\n",
            plan_label(planned).replace('_', " "),
            path.shim_dir,
            path.target
                .as_deref()
                .map(|target| format!(" ({target})"))
                .unwrap_or_default()
        ));
    }
}

/// The wire value of one planned PATH step, spelled for a human.
fn plan_label(planned: &vibe_wire::generated::self_apply_report::PathPlan) -> &'static str {
    use vibe_wire::generated::self_apply_report::PathPlan;
    match planned {
        PathPlan::AlreadyOnPath => "already_on_path",
        PathPlan::WouldApply => "would_apply",
        PathPlan::WouldAsk => "would_ask",
        PathPlan::WouldSkip => "would_skip",
    }
}
