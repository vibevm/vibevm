//! Validation of package-owned application and installer declarations.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#declaration");

use std::collections::BTreeSet;
use std::path::Path;

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
        if matches!(
            self.installation,
            InstallerMethod::Exe { .. } | InstallerMethod::Msi { .. }
        ) && self.platform != "windows"
        {
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
        self.installation.validate()?;
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
            if matches!(
                &self.installation,
                InstallerMethod::Zip { runners, .. } | InstallerMethod::TarGz { runners, .. }
                    if runners.iter().any(|runner| runner.name == command.name)
            ) {
                return Err(format!(
                    "[[installer_options.public_commands]].name `{}` conflicts with an archive runner in installer option `{}`",
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
        match (&self.installer_package, self.runtime, &self.entry) {
            (None, None, None) => {}
            (Some(package), Some(_), Some(entry)) => {
                validate_external(package, entry, "[application]")?;
            }
            _ => return Err(
                "[application] legacy installer_package/runtime/entry must be declared together"
                    .into(),
            ),
        }
        let mut seen = BTreeSet::new();
        for command in &self.commands {
            if !is_portable_token(command) {
                return Err(format!(
                    "[application].commands value `{command}` is not a portable lowercase token"
                ));
            }
            if !seen.insert(command) {
                return Err(format!(
                    "[application].commands contains duplicate `{command}`"
                ));
            }
        }
        if let Some(distribution) = &self.distribution {
            distribution.validate()?;
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
                runners,
            }
            | Self::TarGz {
                source,
                sha256,
                payload_root,
                runners,
            } => {
                validate_source(source, table)?;
                validate_sha256(sha256, table)?;
                if let Some(root) = payload_root {
                    declarant_path(root)
                        .map_err(|fault| format!("{table}.payload_root: {}", fault.reason()))?;
                }
                let mut names = BTreeSet::new();
                let mut paths = BTreeSet::new();
                for runner in runners {
                    declarant_path(&runner.path)
                        .map_err(|fault| format!("{table}.runners.path: {}", fault.reason()))?;
                    if !is_portable_token(&runner.name) || !names.insert(runner.name.as_str()) {
                        return Err(format!(
                            "{table}.runners.name `{}` must be a unique portable token",
                            runner.name
                        ));
                    }
                    if !paths.insert(&runner.path) {
                        return Err(format!(
                            "{table}.runners.path `{}` is duplicated",
                            runner.path.display()
                        ));
                    }
                }
            }
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

fn validate_shell_command(value: &str, table: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(format!(
            "{table}.{field} must be a nonempty shell command without control characters"
        ));
    }
    Ok(())
}

fn validate_source(value: &str, table: &str) -> Result<(), String> {
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

fn validate_sha256(value: &str, table: &str) -> Result<(), String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "{table}.sha256 must be a 64-digit hexadecimal digest"
        ));
    }
    Ok(())
}

fn validate_command_root(value: &str, table: &str) -> Result<(), String> {
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

fn validate_external(package: &PackageRef, entry: &Path, table: &str) -> Result<(), String> {
    if package.group.is_none() || !package.version.is_exact_pin() {
        return Err(format!(
            "{table}.installer_package must be fully qualified and exactly pinned"
        ));
    }
    declarant_path(entry).map_err(|fault| {
        format!(
            "{table}.entry must be a portable provider-relative file path: {}",
            fault.reason()
        )
    })?;
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

impl ApplicationDistributionDecl {
    fn validate(&self) -> Result<(), String> {
        if !valid_repository(&self.repository) {
            return Err("[application.distribution].repository must be owner/name".into());
        }
        if !valid_release_token(&self.release_tag) {
            return Err(
                "[application.distribution].release_tag must be a portable release token".into(),
            );
        }
        declarant_path(PathBuf::from(&self.index_asset).as_path()).map_err(|fault| {
            format!(
                "[application.distribution].index_asset must be one portable file name: {}",
                fault.reason()
            )
        })?;
        if PathBuf::from(&self.index_asset).components().count() != 1 {
            return Err(
                "[application.distribution].index_asset must be one portable file name".into(),
            );
        }
        Ok(())
    }
}

impl ApplicationSourceDecl {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if !credential_free_https(&self.url) {
            return Err(
                "[application_source].url must be credential-free HTTPS without query or fragment"
                    .into(),
            );
        }
        if !self.tracked_ref.starts_with("refs/heads/") || !valid_git_ref(&self.tracked_ref) {
            return Err(
                "[application_source].tracked_ref must be an explicit safe refs/heads/... name"
                    .into(),
            );
        }
        declarant_path(&self.registry_path).map_err(|fault| {
            format!(
                "[application_source].registry_path must be portable and relative: {}",
                fault.reason()
            )
        })?;
        Ok(())
    }
}

fn valid_repository(value: &str) -> bool {
    let mut parts = value.split('/');
    matches!((parts.next(), parts.next(), parts.next()), (Some(owner), Some(repo), None)
        if is_slug(owner) && is_slug(repo))
}

fn is_slug(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

fn valid_release_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
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

fn valid_git_ref(value: &str) -> bool {
    !value.ends_with('/')
        && !value.ends_with('.')
        && !value.contains("..")
        && !value.contains("@{")
        && !value.contains("//")
        && !value
            .chars()
            .any(|ch| ch.is_control() || ch.is_whitespace() || "~^:?*[\\".contains(ch))
}
