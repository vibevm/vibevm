//! Syntax validation of launchers, icon sets and menu metadata
//! (`PROP-059#launch-points`, `#menu`, `#desktop-entries`). Files are
//! checked against the package tree by [`super::icon_findings`].

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#menu");

use std::collections::BTreeSet;
use std::path::Path;

use crate::manifest::{declarant_path, declarant_path_pattern, is_portable_token};

use super::validation::validate_template;
use super::*;

impl LauncherDecl {
    /// Validate one row. `override_allowed` is true in an option with
    /// `launchers_from`, where a row may omit `name`, `executable` and
    /// `kind` to override a derived launcher field by field.
    pub(crate) fn validate(&self, override_allowed: bool) -> Result<(), String> {
        if !is_portable_token(&self.id) {
            return Err(format!(
                "[[launchers]].id `{}` must be a portable lowercase token",
                self.id
            ));
        }
        if !override_allowed && !self.is_complete() {
            return Err(format!(
                "[[launchers]] `{}` must declare a nonempty name, an executable and a kind (`gui`, `tui` or `command`)",
                self.id
            ));
        }
        if !self.executable.is_empty() {
            validate_template(&self.executable, "executable", true)?;
        }
        if let Some(directory) = &self.working_directory {
            validate_template(directory, "working_directory", true)?;
        }
        for arg in &self.args {
            validate_template(arg, "args", false)?;
            validate_placeholders(arg, &self.id)?;
        }
        if let (Some(kind), Some(terminal)) = (self.kind, self.terminal)
            && kind.terminal() != terminal
        {
            return Err(format!(
                "[[launchers]] `{}` sets terminal = {terminal}, which disagrees with kind = \"{}\" (terminal = {})",
                self.id,
                kind.as_str(),
                kind.terminal()
            ));
        }
        let table = format!("[[launchers]] `{}`", self.id);
        validate_categories(&self.categories, &table)?;
        validate_words(&self.keywords, &table, "keywords")?;
        for (field, value) in [
            ("generic_name", &self.generic_name),
            ("window_class", &self.window_class),
        ] {
            if value
                .as_ref()
                .is_some_and(|text| text.trim().is_empty() || text.chars().any(char::is_control))
            {
                return Err(format!("{table} {field} must be nonempty text"));
            }
        }
        for mime in &self.mime_types {
            let valid = mime.split_once('/').is_some_and(|(kind, sub)| {
                !kind.is_empty() && !sub.is_empty() && !sub.contains('/')
            }) && !mime.chars().any(|ch| ch.is_whitespace() || ch.is_control());
            if !valid {
                return Err(format!(
                    "{table} mime_types entry `{mime}` is not TYPE/SUBTYPE"
                ));
            }
        }
        validate_localised(&self.names, &table, "names")?;
        validate_localised(&self.descriptions, &table, "descriptions")?;
        if let Some(icon) = &self.icon {
            validate_icon(icon, &format!("{table} icon"), true)?;
        }
        let mut action_ids = BTreeSet::new();
        for action in &self.actions {
            let valid_id = !action.id.is_empty()
                && action
                    .id
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'));
            if !valid_id {
                return Err(format!(
                    "{table} action id `{}` must use letters, digits, `-`, `_` or `.`",
                    action.id
                ));
            }
            if !action_ids.insert(action.id.as_str()) {
                return Err(format!("{table} has duplicate action id `{}`", action.id));
            }
            if action.name.trim().is_empty() {
                return Err(format!(
                    "{table} action `{}` name must be nonempty",
                    action.id
                ));
            }
            for arg in &action.args {
                validate_template(arg, "actions.args", false)?;
                validate_placeholders(arg, &self.id)?;
            }
            if let Some(icon) = &action.icon {
                validate_icon(icon, &format!("{table} action `{}` icon", action.id), true)?;
            }
        }
        Ok(())
    }
}

impl ApplicationDecl {
    /// `menu_group`, default categories and keywords, and the application
    /// icon, whose files are always package-relative (`PROP-065#icon`).
    pub(super) fn validate_menu(&self) -> Result<(), String> {
        if self
            .menu_group
            .as_ref()
            .is_some_and(|group| group.trim().is_empty() || group.chars().any(char::is_control))
        {
            return Err("[application].menu_group must be nonempty text".into());
        }
        validate_categories(&self.categories, "[application]")?;
        validate_words(&self.keywords, "[application]", "keywords")?;
        if let Some(icon) = &self.icon {
            validate_icon(icon, "[application].icon", false)?;
        }
        Ok(())
    }
}

impl InstallerOptionDecl {
    /// `launchers_from` globs: relative to the application root, never
    /// absolute and never traversing with `..`.
    pub(super) fn validate_launchers_from(&self) -> Result<(), String> {
        for pattern in &self.launchers_from {
            if pattern.split(['/', '\\']).any(|part| part == "..") {
                return Err(format!(
                    "[[installer_options]] `{}` launchers_from `{pattern}` must not traverse with `..`",
                    self.id
                ));
            }
            declarant_path_pattern(Path::new(pattern)).map_err(|fault| {
                format!(
                    "[[installer_options]] `{}` launchers_from `{pattern}` must be a glob relative to the application root: {}",
                    self.id,
                    fault.reason()
                )
            })?;
        }
        Ok(())
    }
}

/// Every `{name}` in an argument is one of the four placeholders, and a
/// list placeholder (`{files}`, `{urls}`) is the whole argument. `${…}`
/// is a template variable, not a placeholder.
fn validate_placeholders(arg: &str, launcher: &str) -> Result<(), String> {
    let bytes = arg.as_bytes();
    let mut at = 0;
    while let Some(offset) = arg[at..].find('{') {
        let open = at + offset;
        at = open + 1;
        if open > 0 && bytes[open - 1] == b'$' {
            continue;
        }
        let Some(close) = arg[open..].find('}').map(|end| open + end) else {
            break;
        };
        let inner = &arg[open + 1..close];
        if inner.is_empty() || !inner.bytes().all(|b| b.is_ascii_alphabetic() || b == b'_') {
            continue;
        }
        let placeholder = &arg[open..=close];
        if !ARG_PLACEHOLDERS.contains(&placeholder) {
            return Err(format!(
                "[[launchers]] `{launcher}` argument `{arg}` has unknown placeholder `{placeholder}`; use {{file}}, {{files}}, {{url}} or {{urls}}"
            ));
        }
        if matches!(placeholder, "{files}" | "{urls}") && arg != placeholder {
            return Err(format!(
                "[[launchers]] `{launcher}` placeholder `{placeholder}` must be a whole argument"
            ));
        }
        at = close + 1;
    }
    Ok(())
}

fn validate_categories(categories: &[String], table: &str) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for category in categories {
        if !MENU_CATEGORIES.contains(&category.as_str()) {
            return Err(format!(
                "{table} category `{category}` is not one of {}",
                MENU_CATEGORIES.join(", ")
            ));
        }
        if !seen.insert(category) {
            return Err(format!("{table} category `{category}` is duplicated"));
        }
    }
    Ok(())
}

fn validate_words(words: &[String], table: &str, field: &str) -> Result<(), String> {
    if words.iter().any(|word| {
        word.trim().is_empty() || word.contains(';') || word.chars().any(char::is_control)
    }) {
        return Err(format!(
            "{table} {field} entries must be nonempty text without `;`"
        ));
    }
    Ok(())
}

/// A language tag (`de`, `pt-BR`, `sr@latin`) keying a localised value.
fn validate_localised(
    table_values: &std::collections::BTreeMap<String, String>,
    table: &str,
    field: &str,
) -> Result<(), String> {
    for (tag, value) in table_values {
        let (base, modifier) = tag.split_once('@').unwrap_or((tag, ""));
        let mut parts = base.split(['-', '_']);
        let language = parts.next().unwrap_or_default();
        let valid = (2..=3).contains(&language.len())
            && language.bytes().all(|b| b.is_ascii_alphabetic())
            && parts
                .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_alphanumeric()))
            && modifier.bytes().all(|b| b.is_ascii_alphanumeric())
            && !(tag.contains('@') && modifier.is_empty());
        if !valid {
            return Err(format!("{table} {field} key `{tag}` is not a language tag"));
        }
        if value.trim().is_empty() || value.chars().any(char::is_control) {
            return Err(format!("{table} {field}.{tag} must be nonempty text"));
        }
    }
    Ok(())
}

/// One icon: a short-form PNG or SVG, or a set whose every file has its
/// slot's extension and whose raster keys are decimal pixel sizes.
/// Launcher icons are templates (`templates = true`); application icons
/// are package-relative literal paths.
fn validate_icon(icon: &IconDecl, label: &str, templates: bool) -> Result<(), String> {
    let path = |value: &str| -> Result<(), String> {
        if templates {
            validate_template(value, "icon", true)
        } else if value.contains('$') {
            Err(format!(
                "{label} `{value}` must be a package-relative file, not a template"
            ))
        } else {
            declarant_path(Path::new(value))
                .map(|_| ())
                .map_err(|fault| {
                    format!(
                        "{label} `{value}` must be a package-relative file: {}",
                        fault.reason()
                    )
                })
        }
    };
    let extension = |value: &str, allowed: &[&str], slot: &str| -> Result<(), String> {
        let lower = value.to_ascii_lowercase();
        if allowed.iter().any(|ext| lower.ends_with(ext)) {
            Ok(())
        } else {
            Err(format!(
                "{label} {slot} `{value}` must end in {}",
                allowed.join(" or ")
            ))
        }
    };
    match icon {
        IconDecl::Path(value) if value.is_empty() => Ok(()),
        IconDecl::Path(value) => {
            path(value)?;
            extension(value, &[".png", ".svg"], "path")
        }
        IconDecl::Set(set) => {
            if set.is_empty() {
                return Err(format!("{label} set names no file"));
            }
            for size in set.png.keys() {
                let decimal = !size.is_empty()
                    && size.bytes().all(|b| b.is_ascii_digit())
                    && !size.starts_with('0')
                    && size.parse::<u32>().is_ok();
                if !decimal {
                    return Err(format!(
                        "{label} png key `{size}` must be a decimal pixel size"
                    ));
                }
            }
            for (slot, value) in set.files() {
                path(value)?;
                let allowed: &[&str] = match slot {
                    IconSlot::Svg => &[".svg"],
                    IconSlot::Png(_) => &[".png"],
                    IconSlot::Ico => &[".ico"],
                    IconSlot::Icns => &[".icns"],
                    IconSlot::Symbolic => &[".svg", ".png"],
                };
                extension(value, allowed, slot.name())?;
            }
            Ok(())
        }
    }
}
