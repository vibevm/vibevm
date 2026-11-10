//! Validation of `[[application.version_file]]`
//! (PROP-075 `#VERSION-FILE-FORMATS`, `#VERSION-FILE-KEYED`): a closed
//! format set, file names that are plain names of one directory, and no two
//! rows claiming the same file or a second `tool-versions` key. Every
//! refusal names its field as a TOML path in the sentence itself, the rule
//! of PROP-059 `#declaration`.
//!
//! The closed `separator` / `comment` / `extra` vocabularies and the two
//! fields a `keyed` row cannot omit are refused by the deserialiser, which
//! names the field and quotes its line — the same refusal an unknown
//! `format` has always had.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS");

use std::collections::BTreeSet;
use std::path::Path;

use crate::manifest::{ApplicationDecl, FieldError, FieldPath, VersionFileDecl};

impl ApplicationDecl {
    /// `[[application.version_file]]`: a closed format set, file names that
    /// are plain names of one directory, and no two rows claiming the same
    /// file or a second `tool-versions` key
    /// (PROP-075 `#VERSION-FILE-FORMATS`).
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS",
        r = 1
    )]
    pub(super) fn validate_version_files(&self, at: &FieldPath) -> Result<(), FieldError> {
        let rows = at.table("version_file");
        let mut claimed: BTreeSet<&str> = BTreeSet::new();
        let mut tool_versions: Option<usize> = None;
        for (position, row) in self.version_file.iter().enumerate() {
            let row_at = rows.index(position);
            // Every refusal names its field as a TOML path in the sentence
            // itself, the rule of PROP-059 `#declaration`.
            let refuse = |name: &str, message: String| {
                let at = row_at.field(name);
                FieldError::new(at.clone(), format!("{at}: {message}"))
            };
            // A row authored as `format = "tool-versions"` has no `names`
            // array of its own, so its file-name refusals name the one
            // field the author did write. Deciding that from the row's own
            // shape keeps the short spelling free of stored state
            // (PROP-075 `#VERSION-FILE-KEYED`).
            let names_field = if row.is_tool_versions() {
                "format"
            } else {
                "names"
            };
            let format = row.format();
            if let VersionFileDecl::Keyed { key, .. } = row {
                if key.trim().is_empty() || key.split_whitespace().count() != 1 {
                    return Err(refuse(
                        "key",
                        format!("format = \"{format}\" key `{key}` must be one nonempty word"),
                    ));
                }
                if row.is_tool_versions() {
                    if let Some(first) = tool_versions {
                        return Err(refuse(
                            "format",
                            format!(
                                "format = \"tool-versions\" is already declared at \
                                 application.version_file[{first}]; a package has one \
                                 `.tool-versions` key"
                            ),
                        ));
                    }
                    tool_versions = Some(position);
                }
            }
            let names = row.names();
            if names.is_empty() {
                return Err(refuse(
                    names_field,
                    format!("format = \"{format}\" needs at least one file name"),
                ));
            }
            for name in names {
                if let Some(reason) = version_file_name_fault(name) {
                    return Err(refuse(names_field, format!("name `{name}` {reason}")));
                }
                if !claimed.insert(name) {
                    return Err(refuse(
                        names_field,
                        format!(
                            "name `{name}` is claimed by another row; one file name belongs to \
                             exactly one row"
                        ),
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Why `name` is not a version file name: one plain name looked for in a
/// directory of the upward search — no separator, no `.`/`..`, no
/// surrounding whitespace. `None` when the name is well formed.
fn version_file_name_fault(name: &str) -> Option<&'static str> {
    if name.is_empty() || name != name.trim() {
        return Some("must be a nonempty name without surrounding whitespace");
    }
    if name == "." || name == ".." {
        return Some("is not a file name");
    }
    if Path::new(name).components().count() != 1
        || name.contains('/')
        || name.contains('\\')
        || name.contains(':')
    {
        return Some("must be a plain file name of one directory, with no path separator");
    }
    None
}
