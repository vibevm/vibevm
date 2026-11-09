//! Validation of package-owned application and installer declarations.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#declaration");

use std::collections::BTreeSet;
use std::path::{Component, Path};

use crate::manifest::{FieldError, FieldPath, declarant_path, is_command_name, is_portable_token};

use super::drop_inputs;
pub(super) use super::site::Site;
use super::template_validation::validate_template;
use super::*;

impl InstallerOptionDecl {
    /// Validate one option; `at` is its address, `installer_options[i]`.
    pub(crate) fn validate(&self, at: &FieldPath) -> Result<(), FieldError> {
        let refuse = |name: &str, message: String| FieldError::new(at.field(name), message);
        if !is_portable_token(&self.id) {
            return Err(refuse(
                "id",
                format!(
                    "[[installer_options]].id `{}` must be a portable lowercase token",
                    self.id
                ),
            ));
        }
        if self.name.trim().is_empty() {
            return Err(refuse(
                "name",
                format!("[[installer_options]] `{}` name must be nonempty", self.id),
            ));
        }
        if !is_portable_token(&self.platform) {
            return Err(refuse(
                "platform",
                format!(
                    "[[installer_options]] `{}` platform must be a portable lowercase token",
                    self.id
                ),
            ));
        }
        if self.platform_name.trim().is_empty() {
            return Err(refuse(
                "platform_name",
                format!(
                    "[[installer_options]] `{}` platform_name must be nonempty",
                    self.id
                ),
            ));
        }
        if !valid_arch(&self.arch) {
            return Err(refuse(
                "arch",
                format!(
                    "[[installer_options]] `{}` arch must be a portable lowercase architecture token",
                    self.id
                ),
            ));
        }
        if self
            .arch_name
            .as_ref()
            .is_some_and(|name| name.trim().is_empty())
        {
            return Err(refuse(
                "arch_name",
                format!(
                    "[[installer_options]] `{}` arch_name must be nonempty when present",
                    self.id
                ),
            ));
        }
        let native = matches!(
            self.installation,
            Some(InstallerMethod::Exe { .. } | InstallerMethod::Msi { .. })
        ) || self
            .steps
            .iter()
            .any(|step| matches!(step.kind, StepKind::Native { .. }));
        if native && self.platform != "windows" {
            return Err(refuse(
                "platform",
                format!(
                    "[[installer_options]] `{}` exe/msi methods require platform = windows",
                    self.id
                ),
            ));
        }
        let option = Site::new("[[installer_options]]", at.clone());
        if let Some(command) = &self.pre_install {
            validate_shell_command(command, &option, "pre_install")?;
        }
        if let Some(command) = &self.post_install {
            validate_shell_command(command, &option, "post_install")?;
        }
        match (&self.installation, self.steps.is_empty()) {
            (Some(method), true) => {
                let stray = if self.result.is_some() {
                    Some("result")
                } else if !self.features.is_empty() {
                    Some("features")
                } else if !self.profiles.is_empty() {
                    Some("profiles")
                } else {
                    None
                };
                if let Some(stray) = stray {
                    return Err(refuse(
                        stray,
                        format!(
                            "[[installer_options]] `{}` declares result, features or profiles, which apply only to [[steps]]",
                            self.id
                        ),
                    ));
                }
                method.validate(&at.table("installation"))?;
            }
            (None, false) => {}
            (Some(_), false) => {
                return Err(refuse(
                    "installation",
                    format!(
                        "[[installer_options]] `{}` declares both installation and [[steps]]; they are mutually exclusive",
                        self.id
                    ),
                ));
            }
            (None, true) => {
                return Err(FieldError::new(
                    at.clone(),
                    format!(
                        "[[installer_options]] `{}` declares neither installation nor [[steps]]",
                        self.id
                    ),
                ));
            }
        }
        self.validate_steps(at)?;
        self.validate_launchers_from(at)?;
        let mut public_names = BTreeSet::new();
        for (index, command) in self.public_commands.iter().enumerate() {
            let command_at = at.table("public_commands").index(index);
            if !is_command_name(&command.name) || command.name == "vibe" {
                return Err(FieldError::new(
                    command_at.field("name"),
                    format!(
                        "[[installer_options.public_commands]].name `{}` must be a command name (a lowercase letter or digit, then letters, digits, `-`, `_`, `.` or `+`, like `g++` or `python3.12`) other than `vibe`",
                        command.name
                    ),
                ));
            }
            if !public_names.insert(command.name.as_str()) {
                return Err(FieldError::new(
                    command_at.field("name"),
                    format!(
                        "[[installer_options.public_commands]].name `{}` is duplicated in installer option `{}`",
                        command.name, self.id
                    ),
                ));
            }
            // A launcher derived from a desktop entry exists only after
            // installation; the engine checks such a name then.
            if self.launchers_from.is_empty()
                && !self
                    .launchers
                    .iter()
                    .any(|launcher| launcher.id == command.launcher)
            {
                return Err(FieldError::new(
                    command_at.field("launcher"),
                    format!(
                        "[[installer_options.public_commands]] `{}` references unknown launcher `{}` in installer option `{}`",
                        command.name, command.launcher, self.id
                    ),
                ));
            }
        }
        self.validate_path_dirs(at)?;
        Ok(())
    }

    /// Check the directories this option asks to expose whole on PATH
    /// (`PROP-059#path-directories`): each is a payload-relative directory
    /// template contained in the application root — the launcher
    /// `executable` rules, minus the variables that leave the payload — and
    /// no entry repeats.
    fn validate_path_dirs(&self, at: &FieldPath) -> Result<(), FieldError> {
        let mut seen = BTreeSet::new();
        for (index, dir) in self.path_dirs.iter().enumerate() {
            let dir_at = at.table("path_dirs").index(index);
            let refuse = |message: String| FieldError::new(dir_at.clone(), message);
            validate_template(PATH_DIRS_TABLE, dir, "path_dirs", true, &dir_at)?;
            for variable in [
                "$project_root",
                "${project_root}",
                "$vibe_home",
                "${vibe_home}",
            ] {
                if dir.contains(variable) {
                    return Err(refuse(format!(
                        "{PATH_DIRS_TABLE}.path_dirs entry `{dir}` may name only a directory inside the \
                         application payload, so `{variable}` is not allowed; use a payload-relative \
                         path, `$app_root/…`, `$app_bin` or `.`"
                    )));
                }
            }
            if !seen.insert(dir.as_str()) {
                return Err(refuse(format!(
                    "{PATH_DIRS_TABLE}.path_dirs entry `{dir}` is duplicated in installer option `{}`",
                    self.id
                )));
            }
        }
        Ok(())
    }
}

/// The table every `path_dirs` message names.
const PATH_DIRS_TABLE: &str = "[[installer_options]]";

fn valid_arch(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 64
        && bytes[0].is_ascii_alphanumeric()
        && bytes[bytes.len() - 1].is_ascii_alphanumeric()
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
}

impl ApplicationDecl {
    pub(crate) fn validate(&self) -> Result<(), FieldError> {
        let at = FieldPath::root().table("application");
        if !is_portable_token(&self.id) {
            return Err(FieldError::new(
                at.field("id"),
                "[application].id must be a portable lowercase token",
            ));
        }
        self.validate_menu(&at)
    }
}

impl InstallerMethod {
    /// Validate the shorthand method; `at` is `installer_options[i].installation`.
    pub(crate) fn validate(&self, at: &FieldPath) -> Result<(), FieldError> {
        let table = "[installer_options.installation]";
        let site = Site::new(table, at.clone());
        let table = site.prose;
        match self {
            Self::Shell {
                app_root,
                install,
                force_install,
                update,
                uninstall,
                force_uninstall,
                interpreter: _,
            } => {
                validate_command_root(app_root, &site)?;
                validate_shell_command(install, &site, "install")?;
                validate_shell_command(force_install, &site, "force_install")?;
                if let Some(command) = update {
                    validate_shell_command(command, &site, "update")?;
                }
                validate_shell_command(uninstall, &site, "uninstall")?;
                validate_shell_command(force_uninstall, &site, "force_uninstall")?;
            }
            Self::Zip {
                source,
                sha256,
                payload_root,
            }
            | Self::TarGz {
                source,
                sha256,
                payload_root,
            } => {
                validate_source(source, &site)?;
                validate_sha256(sha256, &site)?;
                if let Some(root) = payload_root {
                    declarant_path(root).map_err(|fault| {
                        site.refuse(
                            "payload_root",
                            format!("{table}.payload_root: {}", fault.reason()),
                        )
                    })?;
                }
            }
            Self::SourceBuild { sources, recipe } => validate_source_build(sources, recipe, &site)?,
            Self::Exe {
                source,
                sha256,
                app_root,
                arguments,
                force_arguments,
                uninstall,
                force_uninstall,
            }
            | Self::Msi {
                source,
                sha256,
                app_root,
                arguments,
                force_arguments,
                uninstall,
                force_uninstall,
            } => {
                validate_source(source, &site)?;
                validate_sha256(sha256, &site)?;
                validate_command_root(app_root, &site)?;
                if arguments
                    .iter()
                    .any(|arg| arg.chars().any(char::is_control))
                {
                    return Err(site.refuse(
                        "arguments",
                        format!("{table}.arguments must not contain control characters"),
                    ));
                }
                if force_arguments.as_ref().is_some_and(|values| {
                    values.iter().any(|arg| arg.chars().any(char::is_control))
                }) {
                    return Err(site.refuse(
                        "force_arguments",
                        format!("{table}.force_arguments must not contain control characters"),
                    ));
                }
                if let Some(command) = uninstall {
                    validate_shell_command(command, &site, "uninstall")?;
                }
                if let Some(command) = force_uninstall {
                    validate_shell_command(command, &site, "force_uninstall")?;
                }
            }
        }
        Ok(())
    }
}

pub(super) fn validate_shell_command(
    value: &str,
    site: &Site<'_>,
    field: &str,
) -> Result<(), FieldError> {
    let table = site.prose;
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(site.refuse(
            field,
            format!("{table}.{field} must be a nonempty shell command without control characters"),
        ));
    }
    Ok(())
}

pub(super) fn validate_source(value: &str, site: &Site<'_>) -> Result<(), FieldError> {
    let table = site.prose;
    if value.starts_with("https://") {
        if !credential_free_https(value) {
            return Err(site.refuse(
                "source",
                format!("{table}.source must be credential-free HTTPS"),
            ));
        }
    } else {
        declarant_path(Path::new(value)).map_err(|fault| {
            site.refuse(
                "source",
                format!(
                    "{table}.source must be a portable package-relative path or HTTPS URL: {}",
                    fault.reason()
                ),
            )
        })?;
    }
    Ok(())
}

pub(super) fn validate_sha256(value: &str, site: &Site<'_>) -> Result<(), FieldError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(site.refuse(
            "sha256",
            format!(
                "{}.sha256 must be a 64-digit hexadecimal digest",
                site.prose
            ),
        ));
    }
    Ok(())
}

/// The messages always say `app_root`; a caller whose root is another
/// field (`result.external`) passes a whole site at that field.
pub(super) fn validate_command_root(value: &str, site: &Site<'_>) -> Result<(), FieldError> {
    let table = site.prose;
    let refuse = |message: String| site.refuse("app_root", message);
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(refuse(format!(
            "{table}.app_root must be a nonempty path template"
        )));
    }
    let template_suffix = value
        .strip_prefix("$vibe_home")
        .or_else(|| value.strip_prefix("${vibe_home}"));
    let anchored = value.starts_with('/')
        || (value.as_bytes().get(1) == Some(&b':')
            && matches!(value.as_bytes().get(2), Some(b'/' | b'\\')))
        || template_suffix.is_some();
    if !anchored {
        return Err(refuse(format!(
            "{table}.app_root must be absolute or start with $vibe_home"
        )));
    }
    if value.split(['/', '\\']).any(|component| component == "..") {
        return Err(refuse(format!(
            "{table}.app_root must not traverse with `..`"
        )));
    }
    if template_suffix.is_some_and(|suffix| {
        suffix.contains('$') || (!suffix.is_empty() && !suffix.starts_with('/'))
    }) || (template_suffix.is_none() && value.contains('$'))
    {
        return Err(refuse(format!(
            "{table}.app_root permits only a leading $vibe_home variable"
        )));
    }
    Ok(())
}

fn credential_free_https(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("https://") else {
        return false;
    };
    let authority = rest.split('/').next().unwrap_or_default();
    value.trim() == value
        && !authority.is_empty()
        && !authority.contains('@')
        && !value.contains(['?', '#', '\\'])
        && !value.bytes().any(|b| b.is_ascii_whitespace())
}

/// `source_build` model validation (`PROP-059#source-build`); `vibe-app`
/// executes the declaration this accepts.
fn validate_source_build(
    sources: &[SourceInputDecl],
    recipe: &RecipeDecl,
    site: &Site<'_>,
) -> Result<(), FieldError> {
    let table = site.prose;
    validate_sources(sources, site)?;
    declarant_path(&recipe.script).map_err(|fault| {
        FieldError::new(
            site.leaf(|at| at.table("recipe").field("script")),
            format!(
                "{table}.recipe.script must be a package-relative file: {}",
                fault.reason()
            ),
        )
    })?;
    Ok(())
}

/// The `[[sources]]` inputs of a `source_build` method or a `sources` step.
pub(super) fn validate_sources(
    sources: &[SourceInputDecl],
    site: &Site<'_>,
) -> Result<(), FieldError> {
    let table = site.prose;
    // No sources is valid: the package tree itself is then the only input.
    let mut names = BTreeSet::new();
    for (index, source) in sources.iter().enumerate() {
        let row = Site {
            at: site.leaf(|at| at.table("sources").index(index)),
            ..site.clone()
        };
        let single = declarant_path(Path::new(&source.name)).is_ok()
            && Path::new(&source.name).components().count() == 1
            && matches!(
                Path::new(&source.name).components().next(),
                Some(Component::Normal(_))
            );
        if !single {
            return Err(row.refuse(
                "name",
                format!(
                    "{table}.sources.name `{}` must be one portable directory name",
                    source.name
                ),
            ));
        }
        if !names.insert(source.name.to_ascii_lowercase()) {
            return Err(row.refuse(
                "name",
                format!("{table}.sources.name `{}` is duplicated", source.name),
            ));
        }
        drop_inputs::validate_file_without_drop(source, &row)?;
        match (&source.url, &source.path, &source.drop) {
            (None, None, Some(_)) => drop_inputs::validate_drop_input(source, &row)?,
            (Some(url), None, None) => {
                if !credential_free_http(url) {
                    return Err(row.refuse(
                        "url",
                        format!(
                            "{table}.sources `{}` url must be a credential-free HTTP(S) URL",
                            source.name
                        ),
                    ));
                }
            }
            (None, Some(path), None) => {
                declarant_path(path).map_err(|fault| {
                    row.refuse(
                        "path",
                        format!(
                            "{table}.sources `{}` path must be package-relative: {}",
                            source.name,
                            fault.reason()
                        ),
                    )
                })?;
            }
            _ => {
                return Err(FieldError::new(
                    row.at.clone(),
                    format!(
                        "{table}.sources `{}` must declare exactly one of url, path or drop",
                        source.name
                    ),
                ));
            }
        }
        validate_sha256(&source.sha256, &row)?;
    }
    Ok(())
}

pub(super) fn credential_free_http(value: &str) -> bool {
    match value.strip_prefix("http://") {
        Some(rest) => credential_free_https(&format!("https://{rest}")),
        None => credential_free_https(value),
    }
}
