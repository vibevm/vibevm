//! Manifest schemas used throughout vibevm.
//!
//! - [`Manifest`] — the unified `vibe.toml` carried by every node: a plain
//!   project, a workspace member, a published package, a workspace
//!   coordinator. The node's role is expressed by which sections are
//!   present. Schema: `VIBEVM-SPEC.md` §7,
//!   `spec/modules/vibe-workspace/PROP-007-workspace.md`.
//! - [`Lockfile`] — `vibe.lock` at a workspace's absolute root. Schema:
//!   `VIBEVM-SPEC.md` §7.4.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-007#unified-manifest");

mod consumer;
mod decor;
mod document;
pub mod i18n;
mod lockfile;
mod package;
mod project;
pub mod purl;
mod redirect;
mod subskill;

pub use consumer::{ConsumerNode, NodeRole};
pub use document::{BootSection, Manifest, OriginSection, WorkspaceSection};
pub use lockfile::{
    CURRENT_SCHEMA_VERSION, LockedPackage, LockedSubskill, Lockfile, LockfileMeta, SourceKind,
    VirtualCapabilityRecord,
};
pub use package::{
    AccessLevel, AllowFriendsOverride, BinaryDecl, BootCategory, BootSnippet, BootSnippetFragment,
    Compatibility, ConditionalTarget, ConflictsList, FeaturesTable, GitPackageDep, GitRefKind,
    HooksDecl, LinkType, MCP_ARG_VARS, Materialization, McpServerDecl, Obsoletes, OverrideEntry,
    OverrideTable, OverrideTarget, PackageFormat, PackageMeta, PathPackageDep, Provides,
    PublishPosture, Recommends, Requires, RequiresAny, SkillDecl, Suggests, TargetOs,
    VarRegistryDep, VisibilityMeta, WhenCondition,
};
pub use project::{
    ActiveSection, AuthKind, DEFAULT_REGISTRY_GITVERSE_NAME, DEFAULT_REGISTRY_GITVERSE_URL,
    DEFAULT_REGISTRY_NAME, DEFAULT_REGISTRY_REF, DEFAULT_REGISTRY_URL, LlmSection, MirrorSection,
    NamingConvention, OverrideSection, ProjectSection, RegistrySection, SpecFormat,
};
pub use redirect::{RedirectFile, RedirectSection, RefPolicy, parse_redirect_bytes};
pub use subskill::{
    ActivationRules, DeliveryMode, SubskillConflicts, SubskillContent, SubskillManifest,
    SubskillMeta, SubskillRecommends,
};

use std::fs;
use std::path::Path;

use serde::{Serialize, de::DeserializeOwned};

use crate::error::{Error, Result};

pub(crate) fn read_toml<T, P>(path: P) -> Result<T>
where
    T: DeserializeOwned,
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let text = fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    toml::from_str::<T>(&text).map_err(|source| Error::parse_toml(path.to_path_buf(), source))
}

pub(crate) fn write_toml<T, P>(path: P, value: &T) -> Result<()>
where
    T: Serialize,
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let rendered = toml::to_string_pretty(value)?;
    let final_content = match fs::read_to_string(path) {
        Ok(existing) => merge_preserving_comments(&existing, &rendered),
        Err(_) => rendered,
    };
    fs::write(path, final_content).map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })
}

/// Merge a freshly-rendered TOML payload (`new_rendered`) into the
/// existing file's comment / whitespace decoration so that
/// human-edited comments survive a `vibe install` / `vibe uninstall`
/// / `vibe registry add` write.
///
/// Strategy:
///
/// 1. Parse both strings as `toml_edit::DocumentMut`. The new
///    document is the authoritative source of structure (the values
///    in memory after the schema's serde-Serialize); the existing
///    document is the source of decoration (leading whitespace,
///    `#`-prefixed comments, blank-line padding).
///
/// 2. Copy the **document-level prefix** (everything before the
///    first table — header comments and blank lines) from existing
///    onto new.
///
/// 3. Recursively pair matching table keys and value shapes, copying
///    their formatting decoration from existing. Tables that only
///    exist in new (e.g. `[requires]` after the operator's first
///    install) get their default decoration. Tables that only existed
///    in existing (e.g. `[active]` if something deletes it) drop with
///    their decoration — structural change wins over decoration
///    preservation. Arrays of tables keep their historical index
///    pairing; ordinary arrays recurse only when their lengths match.
///
/// 4. **Document-level suffix** (anything after the last table —
///    typically operator's footer comments) is preserved by
///    setting `trailing` on the merged document.
///
/// On any parse / merge failure, fall back to the unmerged new
/// rendering. Worst case behaviour matches the prior (pre-toml_edit)
/// implementation, so this strictly improves UX.
fn merge_preserving_comments(existing: &str, new_rendered: &str) -> String {
    let Ok(mut new_doc) = new_rendered.parse::<toml_edit::DocumentMut>() else {
        return new_rendered.to_string();
    };
    let Ok(existing_doc) = existing.parse::<toml_edit::DocumentMut>() else {
        return new_rendered.to_string();
    };

    // 1. Document-level header (everything before the first
    //    table). For an empty `vibe.toml`, `existing_doc.decor()`
    //    has no prefix; for one starting with comments, prefix is
    //    those comments verbatim.
    let existing_root = existing_doc.as_table();
    let new_root = new_doc.as_table_mut();
    if let Some(prefix) = existing_root.decor().prefix() {
        new_root.decor_mut().set_prefix(prefix.clone());
    }

    // 2. Nested decoration. The document root keeps its separately-owned
    //    prefix above; the recursive walker starts at its matching children.
    decor::copy_matching_table_items(existing_root, new_doc.as_table_mut());

    // 3. Document-level trailing — anything after the last
    //    table. `DocumentMut::trailing()` returns the
    //    `&RawString` that holds it; `set_trailing` accepts an
    //    `impl Into<RawString>` (a `&str` works directly). The
    //    distinction matters: top-level table `decor().suffix()`
    //    is empty for documents whose last entry is itself a
    //    table — operator-supplied footer comments live in
    //    `trailing` instead.
    let trailing = existing_doc.trailing().clone();
    new_doc.set_trailing(trailing);

    new_doc.to_string()
}

#[cfg(test)]
mod merge_tests {
    use super::merge_preserving_comments;

    #[test]
    fn header_comments_survive_full_rewrite() {
        let existing = "\
# This is my project's vibe.toml.
# Edit with care.

[project]
name = \"old\"
version = \"0.0.1\"
";
        let new_rendered = "\
[project]
name = \"new\"
version = \"0.0.1\"
";
        let merged = merge_preserving_comments(existing, new_rendered);
        assert!(
            merged.contains("# This is my project's vibe.toml."),
            "header comment must survive:\n{merged}"
        );
        assert!(merged.contains("# Edit with care."));
        // The new value (`name = "new"`) wins over the old one.
        assert!(merged.contains("name = \"new\""));
        assert!(!merged.contains("name = \"old\""));
    }

    #[test]
    fn pre_table_comments_survive_for_unchanged_sections() {
        let existing = "\
[project]
name = \"demo\"
version = \"0.0.1\"

# Primary registry — host migrated from GitVerse on 2026-04-29.
[[registry]]
name = \"vibespecs\"
url = \"https://github.com/vibespecs\"
";
        // Simulate `vibe install flow:wal` adding a [requires] section
        // — re-render the manifest with all sections, including the new
        // one.
        let new_rendered = "\
[project]
name = \"demo\"
version = \"0.0.1\"

[[registry]]
name = \"vibespecs\"
url = \"https://github.com/vibespecs\"

[requires]
packages = [\"flow:wal@^0.1.0\"]
";
        let merged = merge_preserving_comments(existing, new_rendered);
        assert!(
            merged.contains("# Primary registry — host migrated from GitVerse on 2026-04-29."),
            "pre-table comment on [[registry]] must survive:\n{merged}"
        );
        assert!(merged.contains("[requires]"));
        assert!(merged.contains("flow:wal@^0.1.0"));
    }

    #[test]
    fn trailing_comments_survive() {
        let existing = "\
[project]
name = \"demo\"
version = \"0.0.1\"

# Footer — please don't remove this.
";
        let new_rendered = "\
[project]
name = \"demo\"
version = \"0.0.2\"
";
        let merged = merge_preserving_comments(existing, new_rendered);
        assert!(
            merged.contains("# Footer — please don't remove this."),
            "trailing comment must survive:\n{merged}"
        );
        assert!(merged.contains("version = \"0.0.2\""));
    }

    #[test]
    fn inline_kv_comments_survive_inside_array_of_tables() {
        // The headline use case for inline-decor preservation:
        // operator hand-edited a comment between `name` and `url`
        // inside a `[[registry]]` block. A subsequent
        // `vibe install` re-renders the manifest; the inline
        // comment must not be wiped out.
        let existing = "\
[project]
name = \"demo\"
version = \"0.0.1\"

[[registry]]
name = \"vibespecs\"
# host migrated from GitVerse on 2026-04-29 — keep this in sync.
url = \"https://github.com/vibespecs\"
";
        // `vibe install`-shape rewrite: same registry, but with a
        // freshly-added `[requires]` block at the bottom.
        let new_rendered = "\
[project]
name = \"demo\"
version = \"0.0.1\"

[[registry]]
name = \"vibespecs\"
url = \"https://github.com/vibespecs\"

[requires]
packages = [\"flow:wal@^0.1.0\"]
";
        let merged = merge_preserving_comments(existing, new_rendered);
        assert!(
            merged.contains("# host migrated from GitVerse on 2026-04-29 — keep this in sync."),
            "inline comment between name and url must survive:\n{merged}"
        );
        assert!(merged.contains("[requires]"));
        assert!(merged.contains("flow:wal@^0.1.0"));
    }

    #[test]
    fn nested_extension_tables_preserve_comments_and_new_values_recursively() {
        let existing = r#"[[extension]] # KEEP-EXTENSION-HEADER
id = "announce"
point = "phase:build"

# KEEP-HANDLER-PREFIX
[extension.handler] # KEEP-HANDLER-HEADER
kind = "builtin" # KEEP-HANDLER-KIND
# KEEP-HANDLER-NAME
name = "log"

# KEEP-CONFIG-PREFIX
[extension.config]
# KEEP-CONFIG-MESSAGE
message = "old message"

# KEEP-PASS-PREFIX
[extension.pass]
kind = "transform" # KEEP-PASS-KIND
level = "document"

# KEEP-PREBUILT-PREFIX
[extension.handler.prebuilt]
# KEEP-PREBUILT-PLATFORM
windows = "old.dll"
"#;
        let new_rendered = r#"[[extension]]
id = "announce"
point = "phase:build"

[extension.handler]
kind = "builtin"
name = "log"

[extension.config]
message = "new message"

[extension.pass]
kind = "transform"
level = "document"

[extension.handler.prebuilt]
windows = "new.dll"
"#;

        let merged = merge_preserving_comments(existing, new_rendered);
        for marker in [
            "KEEP-EXTENSION-HEADER",
            "KEEP-HANDLER-PREFIX",
            "KEEP-HANDLER-HEADER",
            "KEEP-HANDLER-KIND",
            "KEEP-HANDLER-NAME",
            "KEEP-CONFIG-PREFIX",
            "KEEP-CONFIG-MESSAGE",
            "KEEP-PASS-PREFIX",
            "KEEP-PASS-KIND",
            "KEEP-PREBUILT-PREFIX",
            "KEEP-PREBUILT-PLATFORM",
        ] {
            assert!(merged.contains(marker), "lost {marker}:\n{merged}");
        }
        assert!(merged.contains("message = \"new message\""), "{merged}");
        assert!(merged.contains("windows = \"new.dll\""), "{merged}");
        assert!(!merged.contains("old message"), "{merged}");
        assert!(!merged.contains("old.dll"), "{merged}");
    }

    #[test]
    fn inline_table_preserves_inner_and_outer_decoration() {
        let existing = r#"[[extension]]
id = "inline"
handler = { kind=  "builtin" , name =  "log" } # KEEP-OUTER
"#;
        let new_rendered = r#"[[extension]]
id = "inline"
handler = { kind = "builtin", name = "log" }
"#;

        let merged = merge_preserving_comments(existing, new_rendered);
        assert!(
            merged.contains("handler = { kind=  \"builtin\" , name =  \"log\" } # KEEP-OUTER"),
            "inline-table decor must survive byte-for-byte:\n{merged}"
        );
    }

    #[test]
    fn equal_length_nested_arrays_preserve_element_decoration_and_new_values() {
        let existing = r#"[[extension]]
id = "array"

[extension.config]
paths = [
    "old-alpha", # KEEP-ALPHA
    # KEEP-BETA
    "old-beta",
]
"#;
        let new_rendered = r#"[[extension]]
id = "array"

[extension.config]
paths = ["new-alpha", "new-beta"]
"#;

        let merged = merge_preserving_comments(existing, new_rendered);
        assert!(merged.contains("KEEP-ALPHA"), "{merged}");
        assert!(merged.contains("KEEP-BETA"), "{merged}");
        assert!(merged.contains("new-alpha"), "{merged}");
        assert!(merged.contains("new-beta"), "{merged}");
        assert!(!merged.contains("old-alpha"), "{merged}");
        assert!(!merged.contains("old-beta"), "{merged}");
    }

    #[test]
    fn decoration_does_not_cross_type_key_or_array_length_mismatches() {
        let type_existing = r#"[[extension]]
id = "shape"
# DROP-TYPE-KEY
config = { mode = "old" } # DROP-TYPE-MISMATCH
"#;
        let type_new = r#"[[extension]]
id = "shape"

[extension.config]
mode = "new"
"#;
        let type_merged = merge_preserving_comments(type_existing, type_new);
        assert!(!type_merged.contains("DROP-TYPE-KEY"));
        assert!(!type_merged.contains("DROP-TYPE-MISMATCH"));
        assert!(type_merged.contains("mode = \"new\""));

        let key_existing = r#"[[extension]]
id = "key"
legacy = "old" # DROP-KEY-MISMATCH
"#;
        let key_new = r#"[[extension]]
id = "key"
replacement = "new"
"#;
        let key_merged = merge_preserving_comments(key_existing, key_new);
        assert!(!key_merged.contains("DROP-KEY-MISMATCH"));
        assert!(key_merged.contains("replacement = \"new\""));

        let length_existing = r#"[[extension]]
id = "length"

[extension.config]
# DROP-LENGTH-KEY
paths = [
    "one", # DROP-LENGTH-MISMATCH
    "two",
]
"#;
        let length_new = r#"[[extension]]
id = "length"

[extension.config]
paths = ["one", "two", "three"]
"#;
        let length_merged = merge_preserving_comments(length_existing, length_new);
        assert!(!length_merged.contains("DROP-LENGTH-KEY"));
        assert!(!length_merged.contains("DROP-LENGTH-MISMATCH"));
        assert!(length_merged.contains("three"));
    }

    #[test]
    fn merge_falls_back_safely_on_invalid_existing() {
        // If the existing file is unparseable garbage, the merge
        // returns the new rendering unchanged.
        let existing = "this is not valid TOML !@#";
        let new_rendered = "[project]\nname = \"x\"\nversion = \"0.1.0\"\n";
        let merged = merge_preserving_comments(existing, new_rendered);
        assert_eq!(merged, new_rendered);
    }

    #[test]
    fn merge_falls_back_safely_on_invalid_new() {
        // Same direction — defensive against a bug in the serde
        // serialiser producing something toml_edit can't parse.
        let existing = "[project]\nname = \"x\"\n";
        let new_rendered = "this is not valid TOML !@#";
        let merged = merge_preserving_comments(existing, new_rendered);
        assert_eq!(merged, new_rendered);
    }
}
