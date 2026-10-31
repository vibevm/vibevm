//! Validation of package-owned application and installer declarations.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#declaration");

use std::collections::BTreeSet;
use std::path::{Component, Path};

use crate::manifest::{declarant_path, is_portable_token};

use super::*;

impl InstallerOptionDecl {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if !is_portable_token(&self.id) {
            return Err(format!(
                "[[installer_options]].id `{}` must be a portable lowercase token",
                self.id
            ));
        }
        if self.name.trim().is_empty() {
            return Err(format!(
                "[[installer_options]] `{}` name must be nonempty",
                self.id
            ));
        }
        if !is_portable_token(&self.platform) {
            return Err(format!(
                "[[installer_options]] `{}` platform must be a portable lowercase token",
                self.id
            ));
        }
        if self.platform_name.trim().is_empty() {
            return Err(format!(
                "[[installer_options]] `{}` platform_name must be nonempty",
                self.id
            ));
        }
        if !valid_arch(&self.arch) {
            return Err(format!(
                "[[installer_options]] `{}` arch must be a portable lowercase architecture token",
                self.id
            ));
        }
        if self
            .arch_name
            .as_ref()
            .is_some_and(|name| name.trim().is_empty())
        {
            return Err(format!(
                "[[installer_options]] `{}` arch_name must be nonempty when present",
                self.id
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
            return Err(format!(
                "[[installer_options]] `{}` exe/msi methods require platform = windows",
                self.id
            ));
        }
        if let Some(command) = &self.pre_install {
            validate_shell_command(command, "[[installer_options]]", "pre_install")?;
        }
        if let Some(command) = &self.post_install {
            validate_shell_command(command, "[[installer_options]]", "post_install")?;
        }
        match (&self.installation, self.steps.is_empty()) {
            (Some(method), true) => {
                if self.result.is_some() || !self.features.is_empty() {
                    return Err(format!(
                        "[[installer_options]] `{}` declares result or features, which apply only to [[steps]]",
                        self.id
                    ));
                }
                method.validate()?;
            }
            (None, false) => {}
            (Some(_), false) => {
                return Err(format!(
                    "[[installer_options]] `{}` declares both installation and [[steps]]; they are mutually exclusive",
                    self.id
                ));
            }
            (None, true) => {
                return Err(format!(
                    "[[installer_options]] `{}` declares neither installation nor [[steps]]",
                    self.id
                ));
            }
        }
        self.validate_steps()?;
        let mut public_names = BTreeSet::new();
        for command in &self.public_commands {
            if !is_portable_token(&command.name) || command.name == "vibe" {
                return Err(format!(
                    "[[installer_options.public_commands]].name `{}` must be a portable lowercase token other than `vibe`",
                    command.name
                ));
            }
            if !public_names.insert(command.name.as_str()) {
                return Err(format!(
                    "[[installer_options.public_commands]].name `{}` is duplicated in installer option `{}`",
                    command.name, self.id
                ));
            }
            if !self
                .launchers
                .iter()
                .any(|launcher| launcher.id == command.launcher)
            {
                return Err(format!(
                    "[[installer_options.public_commands]] `{}` references unknown launcher `{}` in installer option `{}`",
                    command.name, command.launcher, self.id
                ));
            }
        }
        Ok(())
    }
}

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
    pub(crate) fn validate(&self) -> Result<(), String> {
        if !is_portable_token(&self.id) {
            return Err("[application].id must be a portable lowercase token".into());
        }
        Ok(())
    }
}

impl InstallerMethod {
    pub(crate) fn validate(&self) -> Result<(), String> {
        let table = "[installer_options.installation]";
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
                validate_command_root(app_root, table)?;
                validate_shell_command(install, table, "install")?;
                validate_shell_command(force_install, table, "force_install")?;
                if let Some(command) = update {
                    validate_shell_command(command, table, "update")?;
                }
                validate_shell_command(uninstall, table, "uninstall")?;
                validate_shell_command(force_uninstall, table, "force_uninstall")?;
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
                validate_source(source, table)?;
                validate_sha256(sha256, table)?;
                if let Some(root) = payload_root {
                    declarant_path(root)
                        .map_err(|fault| format!("{table}.payload_root: {}", fault.reason()))?;
                }
            }
            Self::SourceBuild { sources, recipe } => validate_source_build(sources, recipe)?,
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
                validate_source(source, table)?;
                validate_sha256(sha256, table)?;
                validate_command_root(app_root, table)?;
                if arguments
                    .iter()
                    .any(|arg| arg.chars().any(char::is_control))
                {
                    return Err(format!(
                        "{table}.arguments must not contain control characters"
                    ));
                }
                if force_arguments.as_ref().is_some_and(|values| {
                    values.iter().any(|arg| arg.chars().any(char::is_control))
                }) {
                    return Err(format!(
                        "{table}.force_arguments must not contain control characters"
                    ));
                }
                if let Some(command) = uninstall {
                    validate_shell_command(command, table, "uninstall")?;
                }
                if let Some(command) = force_uninstall {
                    validate_shell_command(command, table, "force_uninstall")?;
                }
            }
        }
        Ok(())
    }
}

pub(super) fn validate_shell_command(value: &str, table: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(format!(
            "{table}.{field} must be a nonempty shell command without control characters"
        ));
    }
    Ok(())
}

pub(super) fn validate_source(value: &str, table: &str) -> Result<(), String> {
    if value.starts_with("https://") {
        if !credential_free_https(value) {
            return Err(format!("{table}.source must be credential-free HTTPS"));
        }
    } else {
        declarant_path(Path::new(value)).map_err(|fault| {
            format!(
                "{table}.source must be a portable package-relative path or HTTPS URL: {}",
                fault.reason()
            )
        })?;
    }
    Ok(())
}

pub(super) fn validate_sha256(value: &str, table: &str) -> Result<(), String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "{table}.sha256 must be a 64-digit hexadecimal digest"
        ));
    }
    Ok(())
}

pub(super) fn validate_command_root(value: &str, table: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(format!("{table}.app_root must be a nonempty path template"));
    }
    let template_suffix = value
        .strip_prefix("$vibe_home")
        .or_else(|| value.strip_prefix("${vibe_home}"));
    let anchored = value.starts_with('/')
        || (value.as_bytes().get(1) == Some(&b':')
            && matches!(value.as_bytes().get(2), Some(b'/' | b'\\')))
        || template_suffix.is_some();
    if !anchored {
        return Err(format!(
            "{table}.app_root must be absolute or start with $vibe_home"
        ));
    }
    if value.split(['/', '\\']).any(|component| component == "..") {
        return Err(format!("{table}.app_root must not traverse with `..`"));
    }
    if template_suffix.is_some_and(|suffix| {
        suffix.contains('$') || (!suffix.is_empty() && !suffix.starts_with('/'))
    }) || (template_suffix.is_none() && value.contains('$'))
    {
        return Err(format!(
            "{table}.app_root permits only a leading $vibe_home variable"
        ));
    }
    Ok(())
}

impl LauncherDecl {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if !is_portable_token(&self.id) {
            return Err(format!(
                "[[launchers]].id `{}` must be a portable lowercase token",
                self.id
            ));
        }
        if self.name.trim().is_empty() {
            return Err(format!("[[launchers]] `{}` name must be nonempty", self.id));
        }
        validate_template(&self.executable, "executable", true)?;
        if let Some(directory) = &self.working_directory {
            validate_template(directory, "working_directory", true)?;
        }
        if let Some(icon) = &self.icon
            && !icon.is_empty()
        {
            validate_template(icon, "icon", true)?;
        }
        for arg in &self.args {
            validate_template(arg, "args", false)?;
        }
        Ok(())
    }
}

/// Syntax-only check. Runtime resolution applies root-specific containment and
/// file checks after substituting values from the installed application record.
fn validate_template(value: &str, field: &str, path: bool) -> Result<(), String> {
    if path && value.is_empty() {
        return Err(format!("[[launchers]].{field} must be nonempty"));
    }
    if path && value == "." {
        return Ok(());
    }
    let mut rendered = String::new();
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '$' {
            rendered.push(ch);
            continue;
        }
        if chars.peek() == Some(&'$') {
            chars.next();
            rendered.push('$');
            continue;
        }
        let braced = chars.peek() == Some(&'{');
        if braced {
            chars.next();
        }
        let mut name = String::new();
        while chars
            .peek()
            .is_some_and(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        {
            if let Some(next) = chars.next() {
                name.push(next);
            }
        }
        if braced && chars.next() != Some('}') {
            return Err(format!(
                "[[launchers]].{field} has an unterminated variable"
            ));
        }
        if !matches!(
            name.as_str(),
            "project_root" | "app_root" | "app_bin" | "vibe_home"
        ) {
            return Err(format!(
                "[[launchers]].{field} has unknown variable `${name}`"
            ));
        }
        rendered.push_str("placeholder");
    }
    if path {
        // An authored variable at the start may become absolute at launch;
        // its suffix must still obey portable component and traversal rules.
        let suffix = value
            .strip_prefix("$app_root")
            .or_else(|| value.strip_prefix("${app_root}"))
            .or_else(|| value.strip_prefix("$app_bin"))
            .or_else(|| value.strip_prefix("${app_bin}"))
            .or_else(|| value.strip_prefix("$project_root"))
            .or_else(|| value.strip_prefix("${project_root}"))
            .or_else(|| value.strip_prefix("$vibe_home"))
            .or_else(|| value.strip_prefix("${vibe_home}"));
        let check = match suffix {
            Some(s) => s.strip_prefix('/').unwrap_or(s),
            None => rendered.as_str(),
        };
        if check.is_empty() && suffix.is_some() {
            return Ok(());
        }
        declarant_path(Path::new(check))
            .map_err(|fault| format!("[[launchers]].{field}: {}", fault.reason()))?;
        if suffix.is_none() && value.starts_with('/') {
            return Err(format!(
                "[[launchers]].{field} must be relative or start with a Vibe variable"
            ));
        }
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
fn validate_source_build(sources: &[SourceInputDecl], recipe: &RecipeDecl) -> Result<(), String> {
    let table = "[installer_options.installation]";
    validate_sources(sources, table)?;
    declarant_path(&recipe.script).map_err(|fault| {
        format!(
            "{table}.recipe.script must be a package-relative file: {}",
            fault.reason()
        )
    })?;
    Ok(())
}

/// The `[[sources]]` inputs of a `source_build` method or a `sources` step.
pub(super) fn validate_sources(sources: &[SourceInputDecl], table: &str) -> Result<(), String> {
    // No sources is valid: the package tree itself is then the only input.
    let mut names = BTreeSet::new();
    for source in sources {
        let single = declarant_path(Path::new(&source.name)).is_ok()
            && Path::new(&source.name).components().count() == 1
            && matches!(
                Path::new(&source.name).components().next(),
                Some(Component::Normal(_))
            );
        if !single {
            return Err(format!(
                "{table}.sources.name `{}` must be one portable directory name",
                source.name
            ));
        }
        if !names.insert(source.name.to_ascii_lowercase()) {
            return Err(format!(
                "{table}.sources.name `{}` is duplicated",
                source.name
            ));
        }
        match (&source.url, &source.path) {
            (Some(url), None) => {
                if !credential_free_http(url) {
                    return Err(format!(
                        "{table}.sources `{}` url must be a credential-free HTTP(S) URL",
                        source.name
                    ));
                }
            }
            (None, Some(path)) => {
                declarant_path(path).map_err(|fault| {
                    format!(
                        "{table}.sources `{}` path must be package-relative: {}",
                        source.name,
                        fault.reason()
                    )
                })?;
            }
            _ => {
                return Err(format!(
                    "{table}.sources `{}` must declare exactly one of url or path",
                    source.name
                ));
            }
        }
        validate_sha256(&source.sha256, table)?;
    }
    Ok(())
}

pub(super) fn credential_free_http(value: &str) -> bool {
    match value.strip_prefix("http://") {
        Some(rest) => credential_free_https(&format!("https://{rest}")),
        None => credential_free_https(value),
    }
}
