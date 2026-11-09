//! The syntax check of one launcher or path template
//! (`PROP-059#launcher-paths`): which variables a template may name, and
//! the portable-component and traversal rules its remainder must obey.
//! Runtime resolution applies the root-specific containment and the file
//! checks after substituting the installed record's values.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#launch-points");

use std::path::Path;

use crate::manifest::{FieldError, FieldPath, declarant_path};

/// Syntax-only check. Runtime resolution applies root-specific containment and
/// file checks after substituting values from the installed application record.
///
/// `field` is the name the message spells; `at` is the exact address of the
/// value (an element of `args`, a slot of an icon set).
pub(super) fn validate_template(
    table: &str,
    value: &str,
    field: &str,
    path: bool,
    at: &FieldPath,
) -> Result<(), FieldError> {
    let refuse = |message: String| FieldError::new(at.clone(), message);
    if path && value.is_empty() {
        return Err(refuse(format!("{table}.{field} must be nonempty")));
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
            return Err(refuse(format!(
                "{table}.{field} has an unterminated variable"
            )));
        }
        if !matches!(
            name.as_str(),
            "project_root" | "app_root" | "app_bin" | "vibe_home"
        ) {
            return Err(refuse(format!(
                "{table}.{field} has unknown variable `${name}`"
            )));
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
            .map_err(|fault| refuse(format!("{table}.{field}: {}", fault.reason())))?;
        if suffix.is_none() && value.starts_with('/') {
            return Err(refuse(format!(
                "{table}.{field} must be relative or start with a Vibe variable"
            )));
        }
    }
    Ok(())
}
