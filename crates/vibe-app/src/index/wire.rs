//! Conversion between the index domain model and the generated
//! `vibe-user-applications/8` wire form. `requiredBy` is written from the
//! forward requirement edges at every save and never read back, so it
//! cannot drift from them (`PROP-065#uninstall-tree`) — a weak edge is
//! mirrored there like any other, because the member names who requires
//! this installation and a recommender does (`PROP-077`
//! `#APP-RECOMMENDS-MECHANICS`).

use std::path::PathBuf;

use vibe_core::manifest::{PublicCommandDecl, RequirementScope, ShellInterpreter, StepPhase};
use vibe_wire::generated::application::e7::index as wire;

use super::launcher_wire::{icon_from_wire, icon_to_wire, launcher_from_wire, launcher_to_wire};
use super::{
    AcceptedLicense, ApplicationMenu, AuthoredCommands, EnvEntry, EnvOp, Index, Installation,
    InstalledTemplate, Lifecycle, Location, Mark, Method, MigrationEntryRecord,
    MigrationOutcomeRecord, MigrationResult, PackageCoordinate, Requirement, StepOutcome,
    StepRecord, ValueKind,
};

pub(super) fn installation_to_wire(
    index: &Index,
    key: &str,
    value: &Installation,
) -> wire::Installation {
    installation_with_reverse(
        value,
        index
            .required_by(key)
            .into_iter()
            .map(|edge| edge.owner.to_owned())
            .collect(),
    )
}

pub(super) fn identity_record(value: &Installation) -> wire::Installation {
    installation_with_reverse(value, Vec::new())
}

fn installation_with_reverse(value: &Installation, required_by: Vec<String>) -> wire::Installation {
    let (library, directory, app_root) = match &value.location {
        Location::Owned { library, directory } => {
            (Some(library.clone()), Some(directory.clone()), None)
        }
        Location::External { app_root } => (None, None, Some(path_text(app_root))),
    };
    let (icon, icon_set) = icon_to_wire(value.icon.as_ref());
    wire::Installation {
        source_evidence: value.source_evidence.clone(),
        secret_origins: value.secret_origins.clone(),
        configuration: value.configuration.clone(),
        pending: value.pending.clone(),
        slot: value.slot.clone(),
        package: wire::Package {
            group: value.package.group.clone(),
            name: value.package.name.clone(),
            version: value.package.version.clone(),
        },
        option: value.option.clone(),
        platform: value.platform.clone(),
        arch: value.arch.clone(),
        method: match value.method {
            Method::Shell => wire::Method::Shell,
            Method::Zip => wire::Method::Zip,
            Method::TarGz => wire::Method::TarGz,
            Method::Exe => wire::Method::Exe,
            Method::Msi => wire::Method::Msi,
            Method::SourceBuild => wire::Method::SourceBuild,
            Method::Steps => wire::Method::Steps,
        },
        library,
        directory,
        app_root,
        portable: value.portable,
        launchers: value.launchers.iter().map(launcher_to_wire).collect(),
        public_commands: value
            .public_commands
            .iter()
            .map(|command| wire::PublicCommand {
                name: command.name.clone(),
                launcher: command.launcher.clone(),
            })
            .collect(),
        path_dirs: value.path_dirs.clone(),
        template: value
            .template
            .as_ref()
            .map(|template| wire::InstallationTemplate {
                revision: template.revision.clone(),
                produced: template.produced.clone(),
                rendered_sha256: template.rendered_sha256.clone(),
                digest_source_sha256: template.digest_source_sha256.clone(),
                source_url: template.source_url.clone(),
                digest_url: template.digest_url.clone(),
            }),
        migrations: value
            .migrations
            .iter()
            .map(|entry| wire::MigrationEntry {
                id: entry.id.clone(),
                name: entry.name.clone(),
                description: entry.description.clone(),
                default: entry.default,
            })
            .collect(),
        migrated: value
            .migrated
            .iter()
            .map(|outcome| wire::MigrationOutcome {
                id: outcome.id.clone(),
                from: outcome.from.clone(),
                to: outcome.to.clone(),
                result: result_to_wire(outcome.result),
                step: outcome.step.clone(),
                work: outcome.work.clone(),
                log: outcome.log.clone(),
                at: outcome.at.clone(),
            })
            .collect(),
        commands: wire::AuthoredCommands {
            uninstall: value.commands.uninstall.clone(),
            force_uninstall: value.commands.force_uninstall.clone(),
            update: value.commands.update.clone(),
            interpreter: value.commands.interpreter.map(interpreter_to_wire),
        },
        features: value.features.clone(),
        parameters: value.parameters.clone(),
        license: value.license.as_ref().map(|license| wire::AcceptedLicense {
            name: license.name.clone(),
            accepted_at: license.accepted_at.clone(),
        }),
        icon,
        icon_set,
        menu_group: value.menu.menu_group.clone(),
        categories: value.menu.categories.clone(),
        keywords: value.menu.keywords.clone(),
        steps: value
            .steps
            .iter()
            .map(|step| wire::StepRecord {
                id: step.id.clone(),
                kind: step.kind.clone(),
                phase: (step.phase == StepPhase::Finish).then_some(wire::Phase::Finish),
                outcome: match step.outcome {
                    StepOutcome::Done => wire::Outcome::Done,
                    StepOutcome::Skipped => wire::Outcome::Skipped,
                    StepOutcome::Failed => wire::Outcome::Failed,
                    StepOutcome::Delegated => wire::Outcome::Delegated,
                    StepOutcome::Interrupted => wire::Outcome::Interrupted,
                },
                uninstall: step.uninstall.uninstall.clone(),
                force_uninstall: step.uninstall.force_uninstall.clone(),
                interpreter: step.uninstall.interpreter.map(interpreter_to_wire),
            })
            .collect(),
        hidden: value.lifecycle.hidden,
        mark: (value.lifecycle.mark == Mark::Auto).then_some(wire::Mark::Auto),
        requires: value
            .lifecycle
            .requires
            .iter()
            .map(|requirement| wire::Requirement {
                package: requirement.package.clone(),
                key: requirement.key.clone(),
                scope: scope_to_wire(requirement.scope),
                step: requirement.step.clone(),
                weak: requirement.weak,
            })
            .collect(),
        required_by,
        env: value.lifecycle.env.iter().map(env_to_wire).collect(),
        held: value.lifecycle.held,
        replaced: value.lifecycle.replaced.clone(),
        superseded_by: value.lifecycle.superseded_by.clone(),
    }
}

fn result_to_wire(result: MigrationResult) -> wire::MigrationResult {
    match result {
        MigrationResult::Done => wire::MigrationResult::Done,
        MigrationResult::ExportFailed => wire::MigrationResult::ExportFailed,
        MigrationResult::ImportFailed => wire::MigrationResult::ImportFailed,
        MigrationResult::Pending => wire::MigrationResult::Pending,
    }
}

fn result_from_wire(result: wire::MigrationResult) -> MigrationResult {
    match result {
        wire::MigrationResult::Done => MigrationResult::Done,
        wire::MigrationResult::ExportFailed => MigrationResult::ExportFailed,
        wire::MigrationResult::ImportFailed => MigrationResult::ImportFailed,
        wire::MigrationResult::Pending => MigrationResult::Pending,
    }
}

fn scope_to_wire(scope: RequirementScope) -> wire::Scope {
    match scope {
        RequirementScope::Build => wire::Scope::Build,
        RequirementScope::Runtime => wire::Scope::Runtime,
    }
}

pub(super) fn env_to_wire(entry: &EnvEntry) -> wire::EnvEntry {
    wire::EnvEntry {
        step: entry.step.clone(),
        op: match entry.op {
            EnvOp::Set => wire::EnvOp::Set,
            EnvOp::Unset => wire::EnvOp::Unset,
            EnvOp::PathPrepend => wire::EnvOp::PathPrepend,
        },
        name: entry.name.clone(),
        value: entry.value.clone(),
        previous: entry.previous.clone(),
        previous_kind: entry.previous_kind.map(|kind| match kind {
            ValueKind::String => wire::ValueKind::String,
            ValueKind::ExpandString => wire::ValueKind::ExpandString,
        }),
    }
}

pub(super) fn env_from_wire(entry: wire::EnvEntry) -> EnvEntry {
    EnvEntry {
        step: entry.step,
        op: match entry.op {
            wire::EnvOp::Set => EnvOp::Set,
            wire::EnvOp::Unset => EnvOp::Unset,
            wire::EnvOp::PathPrepend => EnvOp::PathPrepend,
        },
        name: entry.name,
        value: entry.value,
        previous: entry.previous,
        previous_kind: entry.previous_kind.map(|kind| match kind {
            wire::ValueKind::String => ValueKind::String,
            wire::ValueKind::ExpandString => ValueKind::ExpandString,
        }),
    }
}

fn interpreter_to_wire(value: ShellInterpreter) -> wire::Interpreter {
    match value {
        ShellInterpreter::PosixSh => wire::Interpreter::PosixSh,
        ShellInterpreter::WindowsCmd => wire::Interpreter::WindowsCmd,
        ShellInterpreter::WindowsPowershell => wire::Interpreter::WindowsPowershell,
    }
}

fn interpreter_from_wire(value: wire::Interpreter) -> ShellInterpreter {
    match value {
        wire::Interpreter::PosixSh => ShellInterpreter::PosixSh,
        wire::Interpreter::WindowsCmd => ShellInterpreter::WindowsCmd,
        wire::Interpreter::WindowsPowershell => ShellInterpreter::WindowsPowershell,
    }
}

pub(super) fn installation_from_wire(record: wire::Installation) -> Result<Installation, String> {
    crate::configuration::record::validate_origins(&record.secret_origins)?;
    if record.configuration.is_some() && !record.secret_origins.is_empty() {
        return Err("common configuration and legacy secret origin records cannot both own parameter metadata".into());
    }
    if let Some(configuration) = &record.configuration {
        crate::configuration::record::validate(configuration)?;
    }
    let method = match record.method {
        wire::Method::Shell => Method::Shell,
        wire::Method::Zip => Method::Zip,
        wire::Method::TarGz => Method::TarGz,
        wire::Method::Exe => Method::Exe,
        wire::Method::Msi => Method::Msi,
        wire::Method::SourceBuild => Method::SourceBuild,
        wire::Method::Steps => Method::Steps,
    };
    let owned = match method {
        Method::Zip | Method::TarGz | Method::SourceBuild => Some(true),
        Method::Shell | Method::Exe | Method::Msi => Some(false),
        Method::Steps => None,
    };
    let location = match (owned, record.library, record.directory, record.app_root) {
        (Some(true) | None, Some(library), Some(directory), None) => {
            if directory
                .split('/')
                .any(|part| part.is_empty() || part == "..")
            {
                return Err(format!("directory `{directory}` is not library-relative"));
            }
            Location::Owned { library, directory }
        }
        (Some(false) | None, None, None, Some(app_root)) => {
            let app_root = PathBuf::from(app_root);
            if !app_root.is_absolute() {
                return Err("external appRoot is not absolute".into());
            }
            Location::External { app_root }
        }
        _ => {
            return Err(
                "owned methods need library and directory; external methods need only appRoot"
                    .into(),
            );
        }
    };
    Ok(Installation {
        source_evidence: record.source_evidence,
        secret_origins: record.secret_origins,
        configuration: record.configuration,
        pending: record.pending,
        slot: record.slot,
        package: PackageCoordinate {
            group: record.package.group,
            name: record.package.name,
            version: record.package.version,
        },
        option: record.option,
        platform: record.platform,
        arch: record.arch,
        method,
        location,
        portable: record.portable,
        launchers: record
            .launchers
            .into_iter()
            .map(launcher_from_wire)
            .collect(),
        public_commands: record
            .public_commands
            .into_iter()
            .map(|command| PublicCommandDecl {
                name: command.name,
                launcher: command.launcher,
            })
            .collect(),
        path_dirs: record.path_dirs,
        // Not part of the index wire: the sidecar beside it carries the
        // declared version files, and `Index::load` fills them in
        // (`PROP-075#VERSION-FILE-FORMATS`).
        version_file: Vec::new(),
        template: record.template.map(|template| InstalledTemplate {
            revision: template.revision,
            produced: template.produced,
            rendered_sha256: template.rendered_sha256,
            digest_source_sha256: template.digest_source_sha256,
            source_url: template.source_url,
            digest_url: template.digest_url,
        }),
        migrations: record
            .migrations
            .into_iter()
            .map(|entry| MigrationEntryRecord {
                id: entry.id,
                name: entry.name,
                description: entry.description,
                default: entry.default,
            })
            .collect(),
        migrated: record
            .migrated
            .into_iter()
            .map(|outcome| MigrationOutcomeRecord {
                id: outcome.id,
                from: outcome.from,
                to: outcome.to,
                result: result_from_wire(outcome.result),
                step: outcome.step,
                work: outcome.work,
                log: outcome.log,
                at: outcome.at,
            })
            .collect(),
        commands: AuthoredCommands {
            uninstall: record.commands.uninstall,
            force_uninstall: record.commands.force_uninstall,
            update: record.commands.update,
            interpreter: record.commands.interpreter.map(interpreter_from_wire),
        },
        features: record.features,
        parameters: record.parameters,
        license: record.license.map(|license| AcceptedLicense {
            name: license.name,
            accepted_at: license.accepted_at,
        }),
        icon: icon_from_wire(record.icon, record.icon_set),
        menu: ApplicationMenu {
            menu_group: record.menu_group,
            categories: record.categories,
            keywords: record.keywords,
        },
        steps: record
            .steps
            .into_iter()
            .map(|step| StepRecord {
                id: step.id,
                kind: step.kind,
                phase: match step.phase {
                    Some(wire::Phase::Finish) => StepPhase::Finish,
                    _ => StepPhase::Prepare,
                },
                outcome: match step.outcome {
                    wire::Outcome::Done => StepOutcome::Done,
                    wire::Outcome::Skipped => StepOutcome::Skipped,
                    wire::Outcome::Failed => StepOutcome::Failed,
                    wire::Outcome::Delegated => StepOutcome::Delegated,
                    wire::Outcome::Interrupted => StepOutcome::Interrupted,
                },
                uninstall: AuthoredCommands {
                    uninstall: step.uninstall,
                    force_uninstall: step.force_uninstall,
                    update: None,
                    interpreter: step.interpreter.map(interpreter_from_wire),
                },
            })
            .collect(),
        lifecycle: Lifecycle {
            hidden: record.hidden,
            mark: match record.mark {
                Some(wire::Mark::Auto) => Mark::Auto,
                _ => Mark::Manual,
            },
            requires: record
                .requires
                .into_iter()
                .map(|requirement| Requirement {
                    package: requirement.package,
                    key: requirement.key,
                    scope: match requirement.scope {
                        wire::Scope::Build => RequirementScope::Build,
                        wire::Scope::Runtime => RequirementScope::Runtime,
                    },
                    step: requirement.step,
                    weak: requirement.weak,
                })
                .collect(),
            env: record.env.into_iter().map(env_from_wire).collect(),
            held: record.held,
            replaced: record.replaced,
            superseded_by: record.superseded_by,
        },
    })
}

fn path_text(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}
