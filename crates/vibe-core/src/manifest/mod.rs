//! Manifest schemas used throughout vibevm.
//!
//! - [`Manifest`] — the unified `vibe.toml` carried by every node: a plain
//!   project, a workspace member, a published package, a workspace
//!   coordinator. The node's role is expressed by which sections are
//!   present. Schema: `VIBEVM-SPEC.md` §7,
//!   `spec/modules/vibe-workspace/PROP-007-workspace.md`.
//! - [`Lockfile`] — `vibe.lock` at a workspace's absolute root. Schema:
//!   `VIBEVM-SPEC.md` §7.4.

specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-007#unified-manifest");

mod document;
pub mod i18n;
mod lockfile;
mod package;
mod project;
pub mod purl;
mod redirect;
mod subskill;

pub use document::{BootSection, Manifest, OriginSection, WorkspaceSection};
pub use lockfile::{
    CURRENT_SCHEMA_VERSION, LockedPackage, LockedSubskill, Lockfile, LockfileMeta, SourceKind,
    VirtualCapabilityRecord,
};
pub use package::{
    BinaryDecl, BootCategory, BootSnippet, Compatibility, ConditionalTarget, ConflictsList,
    FeaturesTable, GitPackageDep, GitRefKind, HooksDecl, LinkType, Materialization, Obsoletes,
    PackageMeta, PathPackageDep, Provides, PublishPosture, Recommends, Requires, RequiresAny,
    SkillDecl, Suggests, TargetOs, VarRegistryDep, WhenCondition,
};
pub use project::{
    ActiveSection, AuthKind, DEFAULT_REGISTRY_GITVERSE_NAME, DEFAULT_REGISTRY_GITVERSE_URL,
    DEFAULT_REGISTRY_NAME, DEFAULT_REGISTRY_REF, DEFAULT_REGISTRY_URL, LlmSection, MirrorSection,
    NamingConvention, OverrideSection, ProjectSection, RegistrySection,
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
    toml::from_str::<T>(&text).map_err(|source| Error::ParseToml {
        path: path.to_path_buf(),
        source,
    })
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
/// 3. For each top-level table key that appears in **both** the
///    new and the existing document, copy the table's `decor()`
///    (the `prefix` part — comments and blank lines that come
///    immediately *before* the table header) from existing.
///    Tables that only exist in new (e.g. `[requires]` after the
///    operator's first install) get their default decoration.
///    Tables that only existed in existing (e.g. `[active]` if
///    something deletes it) drop with their decoration —
///    structural change wins over decoration preservation.
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

    // 2. Per-table decoration. `Item::Table` carries its own
    //    leading decor; `Item::ArrayOfTables` carries decoration
    //    on each element (`[[registry]]`). Inside each preserved
    //    table, `copy_inline_kv_decor` walks the (key, Value)
    //    pairs and copies prefix / suffix decoration on
    //    matching keys — that's how `# inline note` comments
    //    inside a `[[registry]]` block survive a write.
    for (key, existing_item) in existing_root.iter() {
        let Some(new_item) = new_doc.as_table_mut().get_mut(key) else {
            continue;
        };
        match (existing_item, new_item) {
            (toml_edit::Item::Table(et), toml_edit::Item::Table(nt)) => {
                if let Some(prefix) = et.decor().prefix() {
                    nt.decor_mut().set_prefix(prefix.clone());
                }
                copy_inline_kv_decor(et, nt);
            }
            (toml_edit::Item::ArrayOfTables(eaot), toml_edit::Item::ArrayOfTables(naot)) => {
                // Copy element-level decor up to the shorter of the
                // two arrays — operators rarely add comments
                // intermediate to array elements, and a strict
                // index-pairing is the simplest defensible
                // approximation.
                let pair_count = eaot.len().min(naot.len());
                for i in 0..pair_count {
                    if let (Some(et), Some(nt)) = (eaot.get(i), naot.get_mut(i)) {
                        if let Some(prefix) = et.decor().prefix() {
                            nt.decor_mut().set_prefix(prefix.clone());
                        }
                        copy_inline_kv_decor(et, nt);
                    }
                }
            }
            _ => {
                // Type changed (e.g. table → value or vice versa).
                // Don't try to preserve decor across a type
                // mismatch — the structure changed enough that
                // copying comments would be misleading.
            }
        }
    }

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

/// Copy per-key inline decoration (the prefix / suffix attached
/// to a `Value`'s `Decor`) from `existing` onto matching keys in
/// `new`. This is what preserves comments and blank-line padding
/// **inside** a `[[registry]]` block — between
/// `name = "internal"` and `url = "..."`, for example.
///
/// The pairing is by string-equal key. Keys that exist only in
/// one side fall through with their default decoration (a
/// brand-new `[requires]` written by `vibe install` doesn't try
/// to inherit decor from anywhere).
///
/// Does not recurse into nested tables; deeper nesting is
/// unusual in `vibe.toml` (the schema is mostly flat) and
/// adding recursion would extend correctness obligations
/// without a corresponding payoff. If a future schema grows
/// nested tables and an operator's inline comments matter, this
/// helper extends naturally.
fn copy_inline_kv_decor(existing: &toml_edit::Table, new: &mut toml_edit::Table) {
    // toml_edit splits per-key decor across two surfaces:
    //
    //   - the **Key** carries the leading whitespace + comments
    //     up to the `=` (where `# host migrated…` between two
    //     entries actually lives).
    //   - the **Value** carries the post-`=` decoration plus any
    //     same-line trailing comment (`name = "x"  # this`).
    //
    // Both must be cloned for full inline-preservation. We
    // collect the read side first, then apply mutably — the
    // borrow checker doesn't allow holding an immutable iter
    // open while we mutate via `get_mut`.
    use toml_edit::Decor;
    let mut updates: Vec<(String, Option<Decor>, Option<Decor>)> = Vec::new();
    for (key, _) in new.iter() {
        let key_str = key.to_string();
        let key_decor = existing.key(&key_str).map(|k| k.leaf_decor().clone());
        let val_decor = match existing.get(&key_str) {
            Some(toml_edit::Item::Value(v)) => Some(v.decor().clone()),
            _ => None,
        };
        if key_decor.is_some() || val_decor.is_some() {
            updates.push((key_str, key_decor, val_decor));
        }
    }
    for (key_str, key_decor, val_decor) in updates {
        if let Some(decor) = key_decor
            && let Some(mut k) = new.key_mut(&key_str)
        {
            *k.leaf_decor_mut() = decor;
        }
        if let Some(decor) = val_decor
            && let Some(toml_edit::Item::Value(nv)) = new.get_mut(&key_str)
        {
            *nv.decor_mut() = decor;
        }
    }
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
