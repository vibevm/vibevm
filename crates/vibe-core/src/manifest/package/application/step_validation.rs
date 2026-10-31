//! Validation of features, steps, conditions, needs, templates and the
//! declared result of an installer option (`PROP-065`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#steps");

use std::collections::BTreeSet;
use std::path::Path;

use crate::manifest::{declarant_path, is_portable_token};

use super::validation::{
    validate_command_root, validate_sha256, validate_shell_command, validate_source,
    validate_sources,
};
use super::*;

impl InstallerOptionDecl {
    /// Every rule of `PROP-065` that can be checked without the host.
    pub(super) fn validate_steps(&self) -> Result<(), String> {
        let option = &self.id;
        let mut feature_ids = BTreeSet::new();
        for feature in &self.features {
            if !is_portable_token(&feature.id) {
                return Err(format!(
                    "[[installer_options.features]].id `{}` must be a portable lowercase token",
                    feature.id
                ));
            }
            if !feature_ids.insert(feature.id.as_str()) {
                return Err(format!(
                    "[[installer_options.features]].id `{}` is duplicated in installer option `{option}`",
                    feature.id
                ));
            }
            if feature.name.trim().is_empty() {
                return Err(format!(
                    "[[installer_options.features]] `{}` name must be nonempty",
                    feature.id
                ));
            }
        }
        let steps = steps_of(self);
        let mut step_ids = BTreeSet::new();
        for step in &steps {
            let table = format!("[[installer_options.steps]] `{}`", step.id);
            if !step_ids.insert(step.id.as_str()) {
                return Err(format!(
                    "{table} is duplicated in installer option `{option}`"
                ));
            }
            if !is_portable_token(&step.id) && step.origin == StepOrigin::Authored {
                return Err(format!(
                    "[[installer_options.steps]].id `{}` must be a portable lowercase token",
                    step.id
                ));
            }
            if step.title.trim().is_empty() {
                return Err(format!("{table} title must be nonempty"));
            }
            for condition in step.when.iter().chain(&step.unless) {
                validate_condition(condition, &feature_ids, &table)?;
            }
            if step.phase == StepPhase::Finish
                && !matches!(
                    step.kind,
                    StepKind::Shell { .. } | StepKind::Agent { .. } | StepKind::Copy { .. }
                )
            {
                return Err(format!(
                    "{table} of kind `{}` cannot run in the finish phase; only shell, agent and copy steps can",
                    step.kind.name()
                ));
            }
            validate_kind(&step.kind, &feature_ids, &table)?;
        }
        match result_of(self) {
            ResultDecl::External(external) if self.installation.is_none() => {
                validate_command_root(&external.external, "[installer_options].result")?;
            }
            ResultDecl::Owned(_)
                if !steps
                    .iter()
                    .any(|step| step.phase == StepPhase::Prepare && step.kind.writes_out()) =>
            {
                return Err(format!(
                    "[[installer_options]] `{option}` has an owned result but no archive, recipe or copy step that writes $VIBE_OUT; \
                     add one or declare result = {{ external = \"…\" }}"
                ));
            }
            _ => {}
        }
        Ok(())
    }
}

fn validate_condition(
    condition: &Condition,
    features: &BTreeSet<&str>,
    table: &str,
) -> Result<(), String> {
    let word = |value: &str| !value.is_empty() && !value.chars().any(|ch| ch.is_whitespace());
    match condition {
        Condition::Feature(id) if !features.contains(id.as_str()) => Err(format!(
            "{table} condition `{condition}` names an undeclared feature"
        )),
        Condition::CommandPresent(name) | Condition::EnvPresent(name) if !word(name) => Err(
            format!("{table} condition `{condition}` needs a nonempty name without whitespace"),
        ),
        Condition::FilePresent(path) => validate_template(path, features, table, "file_present"),
        _ => Ok(()),
    }
}

/// Syntax of one template field: known variables and declared features.
fn validate_template(
    value: &str,
    features: &BTreeSet<&str>,
    table: &str,
    field: &str,
) -> Result<(), String> {
    if value.chars().any(char::is_control) {
        return Err(format!(
            "{table}.{field} must not contain control characters"
        ));
    }
    expand_step_template(value, &mut |var| match var {
        StepVar::Feature(id) if !features.contains(id) => {
            Err(format!("`$feature:{id}` names an undeclared feature"))
        }
        _ => Ok("x".into()),
    })
    .map(|_| ())
    .map_err(|reason| format!("{table}.{field}: {reason}"))
}

fn validate_kind(kind: &StepKind, features: &BTreeSet<&str>, table: &str) -> Result<(), String> {
    match kind {
        StepKind::Sources { sources } => validate_sources(sources, table),
        StepKind::Archive {
            source,
            sha256,
            payload_root,
            ..
        } => {
            validate_source(source, table)?;
            validate_sha256(sha256, table)?;
            if let Some(root) = payload_root {
                declarant_path(root)
                    .map_err(|fault| format!("{table}.payload_root: {}", fault.reason()))?;
            }
            Ok(())
        }
        StepKind::Recipe { script, .. } => declarant_path(script).map(|_| ()).map_err(|fault| {
            format!(
                "{table}.script must be a package-relative file: {}",
                fault.reason()
            )
        }),
        StepKind::Shell {
            command,
            force_install,
            update,
            uninstall,
            force_uninstall,
            ..
        } => {
            validate_shell_command(command, table, "command")?;
            for (field, value) in [
                ("force_install", force_install),
                ("update", update),
                ("uninstall", uninstall),
                ("force_uninstall", force_uninstall),
            ] {
                if let Some(value) = value {
                    validate_shell_command(value, table, field)?;
                }
            }
            Ok(())
        }
        StepKind::Native {
            source,
            sha256,
            arguments,
            force_arguments,
            uninstall,
            force_uninstall,
            ..
        } => {
            validate_source(source, table)?;
            validate_sha256(sha256, table)?;
            let control =
                |values: &[String]| values.iter().any(|arg| arg.chars().any(char::is_control));
            if control(arguments) || force_arguments.as_deref().is_some_and(control) {
                return Err(format!(
                    "{table} arguments must not contain control characters"
                ));
            }
            for (field, value) in [
                ("uninstall", uninstall),
                ("force_uninstall", force_uninstall),
            ] {
                if let Some(value) = value {
                    validate_shell_command(value, table, field)?;
                }
            }
            Ok(())
        }
        StepKind::Copy { from, to } => {
            declarant_path(from).map_err(|fault| {
                format!(
                    "{table}.from must be a package-relative path: {}",
                    fault.reason()
                )
            })?;
            validate_template(to, features, table, "to")?;
            if Path::new(to).is_absolute() || to.split(['/', '\\']).any(|part| part == "..") {
                return Err(format!(
                    "{table}.to must be relative to the payload without `..`"
                ));
            }
            Ok(())
        }
        StepKind::Agent {
            prompt,
            prompt_file,
            ..
        } => match (prompt, prompt_file) {
            (Some(prompt), None) if !prompt.trim().is_empty() => {
                validate_template(prompt, features, table, "prompt")
            }
            (None, Some(file)) => declarant_path(file).map(|_| ()).map_err(|fault| {
                format!(
                    "{table}.prompt_file must be a package-relative file: {}",
                    fault.reason()
                )
            }),
            _ => Err(format!(
                "{table} must declare exactly one nonempty prompt or prompt_file"
            )),
        },
    }
}
