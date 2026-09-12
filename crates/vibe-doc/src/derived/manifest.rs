//! `derived kind="manifest-field"` — a value read out of a package's own
//! manifest at build time (PROP-045 `##ROW-DOCVOCAB-DERIVED`).
//!
//! The grammar is X-030's, and it has three forms because a page needs
//! three different things:
//!
//! * `abstract` — no coordinate: a field of the package the page belongs
//!   to. This is what a card wants, and the page should not have to spell
//!   its own address to get at its own manifest.
//! * `org.vibevm.core/vibevm-docs` — a coordinate alone: the WHOLE
//!   manifest of that package, which is what a reference page shows when
//!   it says «here is a manifest».
//! * `org.vibevm.core/vibevm-docs#package.title` — a coordinate and a
//!   dotted field path: one field of that package.
//!
//! The manifest is read as TOML data rather than through a typed model on
//! purpose: the whole-manifest form has no type, the field form must
//! reach any key a manifest may grow, and neither wants this crate to
//! learn a manifest schema it would then have to keep in step.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-DERIVED");

use std::path::Path;

use crate::error::{DocError, Result};

/// The manifest file name.
pub const MANIFEST: &str = "vibe.toml";

/// Read what a `manifest-field` reference names.
///
/// `package_dir` is the documentation package the page belongs to, and
/// `coordinate` is that package's own `<group>/<name>` — the one address
/// this generator can resolve without a store.
pub fn generate(reference: &str, package_dir: &Path, coordinate: &str) -> Result<String> {
    let fail = |message: String| DocError::Derived {
        kind: "manifest-field",
        reference: reference.to_owned(),
        message,
    };
    let (address, field) = match reference.split_once('#') {
        Some((address, field)) => (address.trim(), Some(field.trim())),
        None if reference.contains('/') => (reference.trim(), None),
        None => ("", Some(reference.trim())),
    };
    if !address.is_empty() && address != coordinate {
        return Err(fail(format!(
            "names `{address}`, and this generator resolves only the documenting \
             package's own manifest (`{coordinate}`); a foreign manifest arrives \
             through the store, which the build has not opened here"
        )));
    }
    let path = package_dir.join(MANIFEST);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| fail(format!("cannot read `{}`: {e}", path.display())))?;
    let Some(field) = field else {
        // A coordinate alone: the whole manifest, exactly as authored.
        return Ok(trim(&text));
    };
    // `toml::from_str`, not `str::parse`: the latter reads a single TOML
    // VALUE, and a manifest is a document.
    let parsed: toml::Value = toml::from_str(&text)
        .map_err(|e| fail(format!("`{}` does not parse: {e}", path.display())))?;
    let value = lookup(&parsed, field)
        .ok_or_else(|| fail(format!("`{}` carries no `{field}`", path.display())))?;
    Ok(render(value))
}

/// Walk a dotted path. `package.title` and the bare `title` both reach
/// the same field: a manifest's scalars live in `[package]`, and making a
/// page spell the table name to read its own title would be ceremony.
fn lookup<'a>(root: &'a toml::Value, field: &str) -> Option<&'a toml::Value> {
    if let Some(found) = walk(root, field) {
        return Some(found);
    }
    root.get("package").and_then(|p| walk(p, field))
}

fn walk<'a>(from: &'a toml::Value, field: &str) -> Option<&'a toml::Value> {
    let mut cursor = from;
    for segment in field.split('.') {
        cursor = cursor.get(segment)?;
    }
    Some(cursor)
}

/// A scalar reads as itself — a page inserting `abstract` wants the
/// prose, not a quoted TOML string. Anything else keeps its TOML form,
/// which is the only honest rendering of a table or a list.
fn render(value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => trim(s),
        toml::Value::Integer(n) => n.to_string(),
        toml::Value::Float(n) => n.to_string(),
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Datetime(d) => d.to_string(),
        other => trim(&other.to_string()),
    }
}

fn trim(text: &str) -> String {
    let joined = text.replace("\r\n", "\n");
    let mut lines: Vec<&str> = joined.split('\n').map(str::trim_end).collect();
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests;
