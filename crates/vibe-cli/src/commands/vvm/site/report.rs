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
    ConsentSource, Effective, EnvBlockStatus, EnvEffectiveness, EnvObservationStatus,
    EnvPathEntryStatus, EnvProbeStatus, EnvTargetKind, EnvironmentObservation, LoginShell,
    PathFixOutcome, PathReport, PxStep, SelfApplyReport, SkipReason, WhereToPut,
};

use crate::commands::vvm::bundle::release_source::kind_label;
use crate::commands::vvm::env::{self, PathFixObservation};
use crate::output;

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
        effective_text(effective, &mut out);
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

/// The wire value of one registry scope, spelled for a human.
fn scope_label(scope: &vibe_wire::generated::self_apply_report::RegistryScope) -> &'static str {
    use vibe_wire::generated::self_apply_report::RegistryScope;
    match scope {
        RegistryScope::User => "user",
        RegistryScope::Site => "site",
        RegistryScope::Default => "default",
    }
}

fn effective_text(effective: &Effective, out: &mut String) {
    if let Some(root) = &effective.install_root {
        out.push_str(&format!(
            "install root: {} ({})\n  shims on PATH: {}\n  data: {}\n",
            root.path, root.from, root.shim_dir, root.data_dir
        ));
        // Applications follow the same root as vibe (PROP-019
        // `#install-root`); with nothing declared they stay under the
        // settings root, which is this default unless VIBE_SETTINGS moves it.
        if root.from == super::layers::InstallBaseOrigin::BuiltIn.describe() {
            out.push_str(
                "  applications: commands and payloads under the settings root \
                 (`~/.vibe`, or `$VIBE_SETTINGS`)\n",
            );
        } else {
            out.push_str(&format!(
                "  applications: commands in {}, payloads in {}\n",
                root.shim_dir,
                std::path::Path::new(&root.path)
                    .join("opt")
                    .join("apps")
                    .display()
            ));
        }
    }
    out.push_str("registries, highest first:\n");
    if effective.registries.is_empty() {
        out.push_str("  (none)\n");
    }
    for row in &effective.registries {
        out.push_str(&format!(
            "  {:<16} {:<9} {:<8} {}\n",
            row.name,
            registry_type_label(&row.type_),
            scope_label(&row.scope),
            row.url
        ));
        // The exclusion is said beside the registry it switches off, so the
        // `never contacted:` line below is never read against an unmarked
        // row. The row stays declared — this is not a removal hint.
        if let Some(selector) = &row.disabled_by {
            out.push_str(&format!(
                "  {:<16} {} [{selector}]\n",
                "",
                vibe_registry::multi_registry_resolver::DISABLED_BY_POLICY_MARK,
            ));
        }
    }
    // Which registry resolution actually asks first: declared precedence is
    // unchanged, but a registry the policy disables is never in play
    // (PROP-002 `#registry-layers`, PROP-019 `#site-profile`).
    if effective
        .registries
        .iter()
        .any(|row| row.disabled_by.is_some())
    {
        out.push_str(&format!(
            "effective primary: {}\n",
            effective
                .registries
                .iter()
                .find(|row| row.disabled_by.is_none())
                .map_or(
                    "(none — the source policy disables every configured registry)",
                    |row| row.name.as_str()
                )
        ));
    }
    match (&effective.release, &effective.release_error) {
        (Some(release), _) => out.push_str(&format!(
            "release source: {} {} ({})\n",
            kind_label(release.source.clone()),
            release.location,
            release.from
        )),
        (None, Some(error)) => out.push_str(&format!("release source: unusable — {error}\n")),
        (None, None) => {}
    }
    if !effective.exclude_hosting.is_empty() {
        out.push_str(&format!(
            "never contacted: {}\n",
            effective.exclude_hosting.join(", ")
        ));
    }
    if let Some(prefer) = &effective.prefer_hosting {
        out.push_str(&format!("preferred hosting: {}\n", prefer.join(", ")));
    }
    let proxy = &effective.proxy;
    out.push_str(&format!(
        "proxy mode: {} (from {})\n",
        proxy.mode.as_deref().unwrap_or("none"),
        proxy.mode_origin
    ));
    let pac = proxy.pac.as_ref().map(|pac| {
        format!(
            "{pac} ({}, from {}){}",
            proxy.pac_kind.as_deref().unwrap_or("?"),
            proxy.pac_origin,
            // `system` resolved here: the URL px will be given and the
            // Windows setting that answered (`PROP-068#proxy-px`).
            proxy
                .pac_resolved
                .as_deref()
                .map(|resolved| format!(" · {resolved}"))
                .unwrap_or_default()
        )
    });
    let port = proxy.px_port.map(|port| port.to_string());
    // Said only when a layer asked for it, so the default report reads
    // exactly as it did before the key existed.
    let revocation = (proxy.revocation != "check")
        .then(|| format!("{} (from {})", proxy.revocation, proxy.revocation_origin));
    for (key, value) in [
        ("https", &proxy.https),
        ("http", &proxy.http),
        ("no_proxy", &proxy.no_proxy),
        ("pac", &pac),
        ("upstream", &proxy.upstream),
        ("px_port", &port),
        ("px_path", &proxy.px_path),
        ("ca_file", &proxy.ca_file),
        ("revocation", &revocation),
    ] {
        if let Some(value) = value {
            out.push_str(&format!("proxy {key}: {value}\n"));
        }
    }
    // PROP-068 `#proxy`: the whole trust set, not only the older key —
    // what a reader has to see to know which certificates are in force.
    if !proxy.extra_ca.is_empty() {
        out.push_str(&format!(
            "extra certificates (added to the operating system's own): {}\n",
            proxy.extra_ca.join(", ")
        ));
    }
    if effective.offline {
        out.push_str("offline by default ([net] offline)\n");
    }
    if effective.site_remap_rows > 0 {
        out.push_str(&format!(
            "remap rows from the site profile: {} (`vibe remap list` shows them)\n",
            effective.site_remap_rows
        ));
    }
    // Always said, like `[[registry]]` and the release source, because a
    // closed network's whole install behaviour turns on it and «the key is
    // absent» is itself the answer an operator needs confirmed
    // (PROP-075 `#VERSIONS-SELECT`).
    out.push_str(&format!(
        "version selection: {} (from {})\n",
        effective.versions_select.select, effective.versions_select.from
    ));
}

/// The wire value of one registry type, spelled for a human — the same
/// strings `[[registry]].type` is written with.
fn registry_type_label(
    kind: &vibe_wire::generated::self_apply_report::RegistryType,
) -> &'static str {
    use vibe_wire::generated::self_apply_report::RegistryType;
    match kind {
        RegistryType::Github => vibe_core::manifest::RegistryType::Github.as_str(),
        RegistryType::Gitverse => vibe_core::manifest::RegistryType::Gitverse.as_str(),
        RegistryType::Git => vibe_core::manifest::RegistryType::Git.as_str(),
        RegistryType::Monorepo => vibe_core::manifest::RegistryType::Monorepo.as_str(),
        RegistryType::Localdir => vibe_core::manifest::RegistryType::Localdir.as_str(),
    }
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
