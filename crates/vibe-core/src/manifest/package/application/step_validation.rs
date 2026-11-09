//! Validation of features, steps, conditions, needs, templates and the
//! declared result of an installer option (`PROP-065`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#steps");

use std::collections::BTreeSet;
use std::path::Path;

use crate::manifest::{FieldError, FieldPath, declarant_path, is_portable_token};

use super::validation::{
    Site, validate_command_root, validate_sha256, validate_shell_command, validate_source,
    validate_sources,
};
use super::*;

impl InstallerOptionDecl {
    /// Every rule of `PROP-065` that can be checked without the host.
    pub(super) fn validate_steps(&self, at: &FieldPath, template: bool) -> Result<(), FieldError> {
        let option = &self.id;
        let names = self.validate_wizard(at)?;
        let option_table = format!("[[installer_options]] `{option}`");
        for (index, requirement) in self.requires.iter().enumerate() {
            requirement.validate(&option_table, &at.table("requires").index(index))?;
        }
        let steps = steps_of(self);
        let mut step_ids = BTreeSet::new();
        let mut authored = 0;
        for step in &steps {
            let table = format!("[[installer_options.steps]] `{}`", step.id);
            let site = self
                .step_site(step, &table, at, &mut authored)
                .templated(template);
            if !step_ids.insert(step.id.as_str()) {
                return Err(site.refuse(
                    "id",
                    format!("{table} is duplicated in installer option `{option}`"),
                ));
            }
            if !is_portable_token(&step.id) && step.origin == StepOrigin::Authored {
                return Err(site.refuse(
                    "id",
                    format!(
                        "[[installer_options.steps]].id `{}` must be a portable lowercase token",
                        step.id
                    ),
                ));
            }
            if step.title.trim().is_empty() {
                return Err(site.refuse("title", format!("{table} title must be nonempty")));
            }
            for (field, conditions) in [("when", &step.when), ("unless", &step.unless)] {
                for condition in conditions {
                    validate_condition(condition, &names, &table, &site.field(field))?;
                }
            }
            if step.phase == StepPhase::Finish
                && !matches!(
                    step.kind,
                    StepKind::Shell { .. }
                        | StepKind::Agent { .. }
                        | StepKind::Copy { .. }
                        | StepKind::Env { .. }
                )
            {
                return Err(site.refuse(
                    "phase",
                    format!(
                        "{table} of kind `{}` cannot run in the finish phase; only shell, agent, copy and env steps can",
                        step.kind.name()
                    ),
                ));
            }
            if step.optional && matches!(step.kind, StepKind::Agent { required: true, .. }) {
                return Err(site.refuse(
                    "optional",
                    format!(
                        "{table} cannot be both optional and required; an optional agent step is skipped without an agent, \
                         a required one refuses the installation (PROP-064#selection)"
                    ),
                ));
            }
            for (index, requirement) in step.requires.iter().enumerate() {
                requirement.validate(&table, &site.leaf(|at| at.table("requires").index(index)))?;
            }
            validate_kind(&step.kind, &names, &site)?;
        }
        match result_of(self) {
            ResultDecl::External(external) if self.installation.is_none() => {
                validate_command_root(
                    &external.external,
                    &Site::whole(
                        "[installer_options].result",
                        at.table("result").field("external"),
                    ),
                )?;
            }
            ResultDecl::Owned(_)
                if !steps
                    .iter()
                    .any(|step| step.phase == StepPhase::Prepare && step.kind.writes_out()) =>
            {
                return Err(FieldError::new(
                    at.clone(),
                    format!(
                        "[[installer_options]] `{option}` has an owned result but no archive, recipe or copy step that writes $VIBE_OUT; \
                         add one or declare result = {{ external = \"…\" }}"
                    ),
                ));
            }
            _ => {}
        }
        Ok(())
    }

    /// The address of one expanded step. A hook or the `installation`
    /// shorthand is a whole site at the field it came from; an authored
    /// step is the next element of `steps`, counted in `authored`.
    fn step_site<'a>(
        &self,
        step: &StepDecl,
        table: &'a str,
        at: &FieldPath,
        authored: &mut usize,
    ) -> Site<'a> {
        match step.origin {
            StepOrigin::PreInstall => Site::whole(table, at.field("pre_install")),
            StepOrigin::PostInstall => Site::whole(table, at.field("post_install")),
            StepOrigin::Authored if self.installation.is_some() => {
                Site::whole(table, at.field("installation"))
            }
            StepOrigin::Authored => {
                let site = Site::new(table, at.table("steps").index(*authored));
                *authored += 1;
                site
            }
        }
    }
}

/// The feature and parameter ids conditions and templates may name.
pub(super) struct Names<'a> {
    pub option: &'a InstallerOptionDecl,
    pub features: BTreeSet<&'a str>,
    pub params: BTreeSet<&'a str>,
}

/// `at` is the step's `when` or `unless` field: a condition is one string
/// of it, and the field may hold one string or an array of them.
fn validate_condition(
    condition: &Condition,
    names: &Names<'_>,
    table: &str,
    at: &FieldPath,
) -> Result<(), FieldError> {
    let refuse = |message: String| Err(FieldError::new(at.clone(), message));
    let word = |value: &str| !value.is_empty() && !value.chars().any(|ch| ch.is_whitespace());
    match condition {
        Condition::Feature(id) if !names.features.contains(id.as_str()) => refuse(format!(
            "{table} condition `{condition}` names an undeclared feature"
        )),
        Condition::Param(id) | Condition::ParamEquals(ParamEquals { id, .. })
            if !names.params.contains(id.as_str()) =>
        {
            refuse(format!(
                "{table} condition `{condition}` names an undeclared parameter"
            ))
        }
        Condition::CommandPresent(name) | Condition::EnvPresent(name) if !word(name) => refuse(
            format!("{table} condition `{condition}` needs a nonempty name without whitespace"),
        ),
        Condition::FilePresent(path) => {
            validate_template(path, names, &Site::whole(table, at.clone()), "file_present")
        }
        _ => Ok(()),
    }
}

/// Syntax of one template field: known variables and declared features.
fn validate_template(
    value: &str,
    names: &Names<'_>,
    site: &Site<'_>,
    field: &str,
) -> Result<(), FieldError> {
    if value.chars().any(char::is_control) {
        return Err(site.refuse(
            field,
            format!("{}.{field} must not contain control characters", site.prose),
        ));
    }
    expand_names(value, names, site, field)
}

/// An inline agent prompt is prose: line breaks and tabs are allowed, other
/// control characters are not (`PROP-064#envelope`).
fn validate_prompt(value: &str, names: &Names<'_>, site: &Site<'_>) -> Result<(), FieldError> {
    names
        .option
        .validate_prompt_template(value)
        .map_err(|reason| site.refuse("prompt", format!("{}.prompt: {reason}", site.prose)))
}

/// The variables a template names exist: features and parameters declared.
fn expand_names(
    value: &str,
    names: &Names<'_>,
    site: &Site<'_>,
    field: &str,
) -> Result<(), FieldError> {
    names
        .option
        .validate_step_template(value)
        .map_err(|reason| site.refuse(field, format!("{}.{field}: {reason}", site.prose)))
}

/// An environment variable name an env step may set or unset: a portable
/// identifier, never `PATH` (`PROP-065#env-step`: PATH only through
/// `path_prepend`). `at` is where the name is written.
fn validate_env_name(
    name: &str,
    table: &str,
    field: &str,
    at: FieldPath,
) -> Result<(), FieldError> {
    let mut chars = name.chars();
    let portable = chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_');
    if !portable {
        return Err(FieldError::new(
            at,
            format!(
                "{table}.{field} name `{name}` must be a letter or `_` followed by letters, digits or `_`"
            ),
        ));
    }
    if name.eq_ignore_ascii_case("PATH") {
        return Err(FieldError::new(
            at,
            format!("{table}.{field} cannot name PATH; PATH is only edited through path_prepend"),
        ));
    }
    Ok(())
}

fn validate_env(
    set: &std::collections::BTreeMap<String, String>,
    unset: &[String],
    path_prepend: &[String],
    names: &Names<'_>,
    site: &Site<'_>,
) -> Result<(), FieldError> {
    let table = site.prose;
    if set.is_empty() && unset.is_empty() && path_prepend.is_empty() {
        return Err(FieldError::new(
            site.at.clone(),
            format!("{table} of kind `env` must declare set, unset or path_prepend"),
        ));
    }
    for (name, value) in set {
        let entry = site.leaf(|at| at.table("set").field(name));
        validate_env_name(name, table, "set", entry.clone())?;
        validate_template(value, names, &Site::whole(table, entry), "set")?;
    }
    for (index, name) in unset.iter().enumerate() {
        let entry = site.leaf(|at| at.table("unset").index(index));
        validate_env_name(name, table, "unset", entry.clone())?;
        if set.contains_key(name) {
            return Err(FieldError::new(
                entry,
                format!("{table} both sets and unsets `{name}`"),
            ));
        }
    }
    for (index, value) in path_prepend.iter().enumerate() {
        let entry = Site::whole(table, site.leaf(|at| at.table("path_prepend").index(index)));
        if value.trim().is_empty() {
            return Err(entry.refuse(
                "path_prepend",
                format!("{table}.path_prepend entries must be nonempty"),
            ));
        }
        validate_template(value, names, &entry, "path_prepend")?;
    }
    Ok(())
}

fn validate_kind(kind: &StepKind, names: &Names<'_>, site: &Site<'_>) -> Result<(), FieldError> {
    let table = site.prose;
    match kind {
        StepKind::Env {
            set,
            unset,
            path_prepend,
        } => validate_env(set, unset, path_prepend, names, site),
        StepKind::Sources { sources } => validate_sources(sources, site),
        StepKind::Archive {
            source,
            sha256,
            payload_root,
            ..
        } => {
            validate_source(source, site)?;
            validate_sha256(sha256, site)?;
            if let Some(root) = payload_root {
                declarant_path(root).map_err(|fault| {
                    site.refuse(
                        "payload_root",
                        format!("{table}.payload_root: {}", fault.reason()),
                    )
                })?;
            }
            Ok(())
        }
        StepKind::Recipe { script, .. } => declarant_path(script).map(|_| ()).map_err(|fault| {
            site.refuse(
                "script",
                format!(
                    "{table}.script must be a package-relative file: {}",
                    fault.reason()
                ),
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
            validate_shell_command(command, site, "command")?;
            for (field, value) in [
                ("force_install", force_install),
                ("update", update),
                ("uninstall", uninstall),
                ("force_uninstall", force_uninstall),
            ] {
                if let Some(value) = value {
                    validate_shell_command(value, site, field)?;
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
            validate_source(source, site)?;
            validate_sha256(sha256, site)?;
            let control =
                |values: &[String]| values.iter().any(|arg| arg.chars().any(char::is_control));
            if control(arguments) || force_arguments.as_deref().is_some_and(control) {
                let field = if control(arguments) {
                    "arguments"
                } else {
                    "force_arguments"
                };
                return Err(site.refuse(
                    field,
                    format!("{table} arguments must not contain control characters"),
                ));
            }
            for (field, value) in [
                ("uninstall", uninstall),
                ("force_uninstall", force_uninstall),
            ] {
                if let Some(value) = value {
                    validate_shell_command(value, site, field)?;
                }
            }
            Ok(())
        }
        StepKind::Copy { from, to } => {
            declarant_path(from).map_err(|fault| {
                site.refuse(
                    "from",
                    format!(
                        "{table}.from must be a package-relative path: {}",
                        fault.reason()
                    ),
                )
            })?;
            validate_template(to, names, site, "to")?;
            if Path::new(to).is_absolute() || to.split(['/', '\\']).any(|part| part == "..") {
                return Err(site.refuse(
                    "to",
                    format!("{table}.to must be relative to the payload without `..`"),
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
                validate_prompt(prompt, names, site)
            }
            (None, Some(file)) => declarant_path(file).map(|_| ()).map_err(|fault| {
                site.refuse(
                    "prompt_file",
                    format!(
                        "{table}.prompt_file must be a package-relative file: {}",
                        fault.reason()
                    ),
                )
            }),
            // An empty prompt, both fields or neither: `prompt` is the field
            // an editor shows for each.
            _ => Err(site.refuse(
                "prompt",
                format!("{table} must declare exactly one nonempty prompt or prompt_file"),
            )),
        },
    }
}
