//! The comment-preserving atomic write engine (PROP-040 §3 `#role-marker`, §6
//! `#diff-from-default`).
//!
//! [`write_layer`] is the single public entry: it renders the (already-diffed)
//! layer table, grafts an existing file's leading header (role-marker) and
//! trailing footer comments onto the new body, and installs it atomically via a
//! sibling `.tmp` + rename. Comment preservation lifts the contiguous comment
//! blocks from the raw text rather than relying on `toml_edit`'s decor
//! attachment (a comment before a `[section]` header or a `key = value` line
//! binds to that item's own decor, not the document root), so the header
//! survives regardless of which item carries it.
//!
//! Spec: [PROP-040 §3, §6](../../../../../spec/modules/vibe-settings/PROP-040-settings.md#role-marker).

use std::path::Path;

use super::PersistError;
use crate::loader::Layer;

/// Atomically write a layer table to disk, preserving an existing file's header
/// comments and role-marker (PROP-040 §6 `#diff-from-default`, §3 `#role-marker`).
///
/// The caller passes the **already-diffed** table (run it through
/// [`super::diff_from_default`] first) — this function writes exactly what it is
/// given. An **existing** file is loaded as a `toml_edit::DocumentMut` so its
/// leading comments (the role-marker header) and trailing footer survive the
/// body rewrite; the body is replaced with the given table and installed via a
/// sibling `.tmp` + `rename` (crash-safe atomic install). A **new** file is
/// created with the layer's role-marker header + the pretty-TOML body. A
/// malformed existing file is recovered: the loader had already warned and
/// treated it as absent (§3 `#missing-is-default`), so a fresh header + valid
/// body replaces the bad bytes.
///
/// ```
/// use std::fs;
///
/// use vibe_settings::loader::Layer;
/// use vibe_settings::persist::write_layer;
///
/// let dir = tempfile::tempdir()?;
/// let path = dir.path().join("settings.toml");
///
/// // A fresh file gets the role-marker header + the body.
/// let mut table = toml::Table::new();
/// table.insert("palette".to_string(), toml::Value::String("rosé-pine".into()));
/// write_layer(&path, &table, Layer::L2)?;
/// let text = fs::read_to_string(&path)?;
/// assert!(text.contains("repo-shared"), "role-marker header written");
/// assert!(text.contains("palette = \"rosé-pine\""));
///
/// // A rewrite of an existing file preserves the header — including a comment
/// // an operator added by hand.
/// let with_note = format!("# my note\n{text}");
/// fs::write(&path, &with_note)?;
/// write_layer(&path, &table, Layer::L2)?;
/// let after = fs::read_to_string(&path)?;
/// assert!(after.contains("my note"), "operator comment preserved");
/// assert!(after.contains("palette = \"rosé-pine\""), "body still present");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[specmark::spec(implements = "spec://vibevm/modules/vibe-settings/PROP-040#role-marker")]
pub fn write_layer(path: &Path, table: &toml::Table, layer: Layer) -> Result<(), PersistError> {
    // Ensure the parent directory (e.g. `.vibe/`) exists before we stage the
    // temp file beside the target.
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|source| PersistError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let existing = match std::fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(source) => {
            return Err(PersistError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    let rendered = render_layer(existing.as_deref(), table, layer, path)?;
    atomic_install(path, &rendered)
}

/// Compose the final file text: a fresh header + body when there is no usable
/// existing file, or the existing header/footer comments preserved above/below a
/// re-rendered body when there is. `path` is carried only so a body round-trip
/// failure (an internal invariant) can name the target in its diagnostic.
fn render_layer(
    existing: Option<&str>,
    table: &toml::Table,
    layer: Layer,
    path: &Path,
) -> Result<String, PersistError> {
    // Render the body and round-trip it through `toml_edit` for canonical
    // formatting (every written file then shares one style). A round-trip
    // failure is an internal invariant violation (the serialiser produced
    // something `toml_edit` cannot re-parse) — surface it as a typed `Edit`
    // error rather than silently writing the un-normalised form.
    let body =
        toml::to_string_pretty(table).map_err(|source| PersistError::Serialize { source })?;
    let body = body
        .parse::<toml_edit::DocumentMut>()
        .map_err(|source| PersistError::Edit {
            path: path.to_path_buf(),
            source,
        })?
        .to_string();
    let body = body.trim_end();

    let Some(existing) = existing else {
        // No prior file — seed it with the role-marker header + pretty body.
        return Ok(format!("{}\n\n{body}\n", layer.role_marker()));
    };

    // Load the existing file as a `toml_edit::DocumentMut`. A parse failure means
    // the file is malformed — the loader already warned (§3 #missing-is-default)
    // and treated the layer as absent, so we cannot trust its bytes; write a
    // fresh header + valid body (a recovery, not a loss of intent).
    if existing.parse::<toml_edit::DocumentMut>().is_err() {
        return Ok(format!("{}\n\n{body}\n", layer.role_marker()));
    }

    // Preserve the operator's leading header (role-marker + any notes) and
    // trailing footer by lifting the contiguous comment blocks from the raw
    // text. This is robust against `toml_edit`'s decor attachment (a comment
    // before a `[section]` header or a `key = value` line attaches to that
    // item's own decor, not the document root) — the raw scan finds the header
    // regardless of which item carries it.
    let header = leading_comments(existing);
    let header = if header.is_empty() {
        layer.role_marker().to_string()
    } else {
        header
    };
    let footer = trailing_comments(existing);

    let mut out = String::new();
    out.push_str(&header);
    out.push_str("\n\n");
    out.push_str(body);
    out.push('\n');
    if !footer.is_empty() {
        out.push('\n');
        out.push_str(&footer);
        out.push('\n');
    }
    Ok(out)
}

/// The contiguous comment block at the top of `text`: the run of `#`-comment and
/// blank lines from the start until the first content line. Trailing blank lines
/// of the run are dropped so the header is tight. Empty when the file opens with
/// content.
fn leading_comments(text: &str) -> String {
    let mut lines: Vec<&str> = Vec::new();
    for line in text.lines() {
        let t = line.trim_start();
        if t.starts_with('#') {
            lines.push(line);
        } else if t.is_empty() {
            // Keep blank padding only inside the comment run (between comments);
            // a leading blank line before any comment is skipped.
            if !lines.is_empty() {
                lines.push(line);
            }
        } else {
            break;
        }
    }
    while lines.last().map(|l| l.trim().is_empty()).unwrap_or(false) {
        lines.pop();
    }
    lines.join("\n")
}

/// The contiguous comment block at the bottom of `text`: the run of `#`-comment
/// and blank lines from the end back to the last content line. Leading blank
/// lines of the run are dropped. Empty when the file ends with content.
fn trailing_comments(text: &str) -> String {
    let mut lines: Vec<&str> = Vec::new();
    for line in text.lines().rev() {
        let t = line.trim_start();
        if t.starts_with('#') {
            lines.insert(0, line);
        } else if t.is_empty() {
            if !lines.is_empty() {
                lines.insert(0, line);
            }
        } else {
            break;
        }
    }
    while lines.first().map(|l| l.trim().is_empty()).unwrap_or(false) {
        lines.remove(0);
    }
    lines.join("\n")
}

/// Write `content` to a sibling `<name>.tmp`, then rename it over `path` — the
/// crash-safe atomic install (a half-written file is never observed at `path`).
fn atomic_install(path: &Path, content: &str) -> Result<(), PersistError> {
    let tmp = path.with_file_name(format!(
        "{}.tmp",
        path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("settings")
    ));
    std::fs::write(&tmp, content).map_err(|source| PersistError::Io {
        path: tmp.clone(),
        source,
    })?;
    std::fs::rename(&tmp, path).map_err(|source| PersistError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn write_layer_creates_new_file_with_marker_header() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".vibe").join("settings.toml");
        let mut table = toml::Table::new();
        table.insert(
            "palette".to_string(),
            toml::Value::String("rosé-pine".into()),
        );
        write_layer(&path, &table, Layer::L2).unwrap();

        // Parent dir created.
        assert!(path.exists());
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("# L2"), "role-marker is the header");
        assert!(text.contains("repo-shared"));
        assert!(text.contains("palette = \"rosé-pine\""));
    }

    #[test]
    fn write_layer_preserves_header_and_footer_comments() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.local.toml");
        // Seed an L3 file with header + inline + footer comments.
        let original = "# my top comment\n# L3 — user-project (gitignored). x\n[node]\nfold = true\n# footer note\n";
        fs::write(&path, original).unwrap();

        let mut table = toml::Table::new();
        let mut node = toml::Table::new();
        node.insert("fold".to_string(), toml::Value::Boolean(true));
        table.insert("node".to_string(), toml::Value::Table(node));

        write_layer(&path, &table, Layer::L3).unwrap();
        let after = fs::read_to_string(&path).unwrap();
        assert!(after.contains("my top comment"), "header comment preserved");
        assert!(after.contains("fold = true"), "body present");
    }

    #[test]
    fn write_layer_recovers_malformed_existing_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        fs::write(&path, "not = valid = toml\n").unwrap();

        let mut table = toml::Table::new();
        table.insert("k".to_string(), toml::Value::Integer(1));
        write_layer(&path, &table, Layer::L2).unwrap();

        let after = fs::read_to_string(&path).unwrap();
        assert!(after.starts_with("# L2"), "fresh header written");
        assert!(after.contains("k = 1"));
        assert!(!after.contains("not = valid"), "bad bytes replaced");
    }

    #[test]
    fn write_layer_atomic_install_uses_tmp_then_rename() {
        // The final file exists and no `.tmp` is left beside it.
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        let table = toml::Table::new();
        write_layer(&path, &table, Layer::L2).unwrap();
        assert!(path.exists());
        assert!(!dir.path().join("settings.toml.tmp").exists());
    }
}
