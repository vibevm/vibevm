//! Validation of `[[application.version_file]]`
//! (PROP-075 `#VERSION-FILE-FORMATS`): a closed format set, file names
//! that are plain names of one directory, and no two rows claiming the
//! same file or a second `tool-versions` key. Every refusal names its
//! field as a TOML path in the sentence itself, the rule of PROP-059
//! `#declaration`.

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
            match row {
                VersionFileDecl::Plain { names } => {
                    if names.is_empty() {
                        return Err(refuse(
                            "names",
                            "format = \"plain\" needs at least one file name".to_owned(),
                        ));
                    }
                    for name in names {
                        if let Some(reason) = version_file_name_fault(name) {
                            return Err(refuse("names", format!("name `{name}` {reason}")));
                        }
                        if !claimed.insert(name.as_str()) {
                            return Err(refuse(
                                "names",
                                format!(
                                    "name `{name}` is claimed by another row; one file name \
                                     belongs to exactly one row"
                                ),
                            ));
                        }
                    }
                }
                VersionFileDecl::ToolVersions { key } => {
                    if key.trim().is_empty() || key.split_whitespace().count() != 1 {
                        return Err(refuse(
                            "key",
                            format!(
                                "format = \"tool-versions\" key `{key}` must be one nonempty word"
                            ),
                        ));
                    }
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
                    if !claimed.insert(crate::manifest::TOOL_VERSIONS_FILE) {
                        return Err(refuse(
                            "format",
                            format!(
                                "`{}` is claimed by a plain row as well as by this \
                                 tool-versions row",
                                crate::manifest::TOOL_VERSIONS_FILE
                            ),
                        ));
                    }
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
