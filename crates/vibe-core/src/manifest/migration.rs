//! Pure, explicitly selected authored-manifest migration plans.
//!
//! The r1 profiles recognize only the finite package shape in the governing
//! law. They are not the complete historical or current manifest language.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-PLAN");

use std::ops::Range;

use sha2::{Digest, Sha256};
use toml_edit::{Document, Item, TableLike, Value};

use super::Manifest;

pub const LEGACY_ABSENT: &str = "materialization-legacy-absent-r1";
pub const CURRENT_ABSENT: &str = "materialization-current-absent-r1";
pub const CURRENT_EPOCH1: &str = "materialization-current-epoch1-r1";
pub const SNAPSHOT_TO_COPY: &str = "snapshot-to-copy-r1";
pub const IDENTITY: &str = "identity-r1";

/// Caller choices; absence asks for input and never selects a default.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MigrationSelection<'a> {
    pub source_profile: Option<&'a str>,
    pub target_profile: Option<&'a str>,
    pub recipe: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationInputRequirement {
    pub field: &'static str,
    pub choices: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationRefusalKind {
    UnknownSelection,
    IncompatibleEdge,
    MalformedSyntax,
    UnrecognizedShape,
    InvalidCurrentManifest,
    UnprovableEdit,
}

/// Fixed diagnosis and an owning field; never raw parser/source contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationRefusal {
    pub kind: MigrationRefusalKind,
    pub field: String,
    pub reason: &'static str,
}

/// One original UTF-8 byte range and its proposed replacement token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationTokenEdit {
    pub field: &'static str,
    pub before_span: Range<usize>,
    pub before_token: String,
    pub after_token: String,
}

/// Owned in-memory proposal, not an application or filesystem grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestMigrationPlan {
    pub source_profile: &'static str,
    pub target_profile: &'static str,
    pub recipe: &'static str,
    pub before: String,
    pub after: String,
    pub edits: Vec<MigrationTokenEdit>,
    pub before_identity: String,
    pub after_identity: String,
    pub diff_identity: String,
    pub checked_manifest: Manifest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestMigrationOutcome {
    Planned(ManifestMigrationPlan),
    NoOp(ManifestMigrationPlan),
    NeedsInput(Vec<MigrationInputRequirement>),
    Refused(MigrationRefusal),
}

type Checked<T> = Result<T, MigrationRefusal>;
const EDGES: [(&str, &str, &str); 3] = [
    (LEGACY_ABSENT, CURRENT_ABSENT, SNAPSHOT_TO_COPY),
    (CURRENT_ABSENT, CURRENT_ABSENT, IDENTITY),
    (CURRENT_EPOCH1, CURRENT_EPOCH1, IDENTITY),
];

/// Plan from exact text without reads, writes, defaults, providers or apply.
/// Current Manifest validation is the final checker, not the frozen grammar.
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-PLAN",
    r = 1
)]
pub fn plan_manifest_migration(
    text: &str,
    selection: MigrationSelection<'_>,
) -> ManifestMigrationOutcome {
    match plan(text, selection) {
        Ok(outcome) => outcome,
        Err(refusal) => ManifestMigrationOutcome::Refused(refusal),
    }
}

fn refusal(kind: MigrationRefusalKind, field: &str, reason: &'static str) -> MigrationRefusal {
    MigrationRefusal {
        kind,
        field: field.to_owned(),
        reason,
    }
}

fn plan(text: &str, selection: MigrationSelection<'_>) -> Checked<ManifestMigrationOutcome> {
    let selected = [
        selection.source_profile,
        selection.target_profile,
        selection.recipe,
    ];
    let fields = ["source_profile", "target_profile", "recipe"];
    for (index, value) in selected.iter().enumerate() {
        if let Some(value) = value {
            let known = if index == 2 {
                matches!(*value, SNAPSHOT_TO_COPY | IDENTITY)
            } else {
                matches!(*value, LEGACY_ABSENT | CURRENT_ABSENT | CURRENT_EPOCH1)
            };
            if !known {
                return Err(refusal(
                    MigrationRefusalKind::UnknownSelection,
                    fields[index],
                    "unknown profile or recipe revision",
                ));
            }
        }
    }
    let edges: Vec<_> = EDGES
        .iter()
        .copied()
        .filter(|(source, target, recipe)| {
            selected
                .iter()
                .zip([source, target, recipe])
                .all(|(choice, value)| choice.is_none_or(|choice| choice == *value))
        })
        .collect();
    if edges.is_empty() {
        return Err(refusal(
            MigrationRefusalKind::IncompatibleEdge,
            "selection",
            "no registered edge matches the explicit choices",
        ));
    }
    if selected.iter().any(Option::is_none) {
        let mut requirements = Vec::new();
        for index in 0..3 {
            if selected[index].is_none() {
                let mut choices = Vec::new();
                for edge in &edges {
                    let value = [edge.0, edge.1, edge.2][index];
                    if !choices.contains(&value) {
                        choices.push(value);
                    }
                }
                requirements.push(MigrationInputRequirement {
                    field: fields[index],
                    choices,
                });
            }
        }
        return Ok(ManifestMigrationOutcome::NeedsInput(requirements));
    }
    let (source, target, recipe) = edges[0];
    let input = Document::parse(text).map_err(|_| {
        refusal(
            MigrationRefusalKind::MalformedSyntax,
            "document",
            "malformed TOML",
        )
    })?;
    check_shape(&input, source)?;
    let edits = if recipe == SNAPSHOT_TO_COPY {
        vec![materialization_edit(text, &input)?]
    } else {
        Vec::new()
    };
    let after = if let Some(edit) = edits.first() {
        let prefix = text.get(..edit.before_span.start).ok_or_else(unprovable)?;
        let suffix = text.get(edit.before_span.end..).ok_or_else(unprovable)?;
        let after = format!("{prefix}{}{suffix}", edit.after_token);
        if after.get(..prefix.len()) != Some(prefix)
            || after.get(prefix.len() + edit.after_token.len()..) != Some(suffix)
        {
            return Err(unprovable());
        }
        after
    } else {
        text.to_owned()
    };
    let output = Document::parse(after.as_str()).map_err(|_| unprovable())?;
    check_shape(&output, target)?;
    let checked_manifest = Manifest::parse_str(&after).map_err(|_| {
        refusal(
            MigrationRefusalKind::InvalidCurrentManifest,
            "document",
            "recognized output refused by current Manifest owner",
        )
    })?;
    let before_identity = identity(b"vibe-manifest-content-r1", &[text.as_bytes()]);
    let after_identity = identity(b"vibe-manifest-content-r1", &[after.as_bytes()]);
    let mut digest = Sha256::new();
    frame(&mut digest, b"vibe-manifest-migration-diff-r1");
    for value in [source, target, recipe, text, after.as_str()] {
        frame(&mut digest, value.as_bytes());
    }
    frame(&mut digest, &(edits.len() as u64).to_be_bytes());
    for edit in &edits {
        frame(&mut digest, edit.field.as_bytes());
        frame(&mut digest, &(edit.before_span.start as u64).to_be_bytes());
        frame(&mut digest, &(edit.before_span.end as u64).to_be_bytes());
        frame(&mut digest, edit.before_token.as_bytes());
        frame(&mut digest, edit.after_token.as_bytes());
    }
    let result = ManifestMigrationPlan {
        source_profile: source,
        target_profile: target,
        recipe,
        before: text.to_owned(),
        after,
        edits,
        before_identity,
        after_identity,
        diff_identity: format!("{:x}", digest.finalize()),
        checked_manifest,
    };
    Ok(if result.edits.is_empty() {
        ManifestMigrationOutcome::NoOp(result)
    } else {
        ManifestMigrationOutcome::Planned(result)
    })
}

fn frame(digest: &mut Sha256, bytes: &[u8]) {
    digest.update((bytes.len() as u64).to_be_bytes());
    digest.update(bytes);
}

fn identity(domain: &[u8], values: &[&[u8]]) -> String {
    let mut digest = Sha256::new();
    frame(&mut digest, domain);
    for bytes in values {
        frame(&mut digest, bytes);
    }
    format!("{:x}", digest.finalize())
}

fn shape(field: &str) -> MigrationRefusal {
    refusal(
        MigrationRefusalKind::UnrecognizedShape,
        field,
        "field, primitive shape or vocabulary outside frozen profile",
    )
}

fn table<'a>(item: Option<&'a Item>, field: &str) -> Checked<&'a dyn TableLike> {
    item.and_then(Item::as_table_like)
        .ok_or_else(|| shape(field))
}

fn string<'a>(table: &'a dyn TableLike, key: &str, field: &str) -> Checked<&'a str> {
    table
        .get(key)
        .and_then(Item::as_str)
        .ok_or_else(|| shape(field))
}

fn only(table: &dyn TableLike, keys: &[&str], field: &str) -> Checked<()> {
    for (key, _) in table.iter() {
        if !keys.contains(&key) {
            return Err(shape(&format!("{field}.{key}")));
        }
    }
    Ok(())
}

fn check_shape(document: &Document<&str>, profile: &str) -> Checked<()> {
    let root = document.as_table();
    only(root, &["package", "requires"], "document")?;
    let package = table(root.get("package"), "package")?;
    only(
        package,
        &[
            "name",
            "group",
            "kind",
            "version",
            "epoch",
            "materialization",
            "license",
            "description",
            "homepage",
            "authors",
            "keywords",
            "bridge",
            "frozen",
        ],
        "package",
    )?;
    for key in ["name", "group", "kind", "version"] {
        string(package, key, &format!("package.{key}"))?;
    }
    if !matches!(
        string(package, "kind", "package.kind")?,
        "flow" | "feat" | "stack" | "tool" | "mcp" | "lang"
    ) {
        return Err(shape("package.kind"));
    }
    let epoch = package.get("epoch");
    if (profile == CURRENT_EPOCH1 && epoch.and_then(Item::as_integer) != Some(1))
        || (profile != CURRENT_EPOCH1 && epoch.is_some())
    {
        return Err(shape("package.epoch"));
    }
    for key in ["materialization", "license", "description", "homepage"] {
        if package.contains_key(key) {
            string(package, key, &format!("package.{key}"))?;
        }
    }
    for key in ["authors", "keywords"] {
        if let Some(item) = package.get(key) {
            let array = item
                .as_array()
                .ok_or_else(|| shape(&format!("package.{key}")))?;
            if array.iter().any(|value| value.as_str().is_none()) {
                return Err(shape(&format!("package.{key}")));
            }
        }
    }
    for key in ["bridge", "frozen"] {
        if package
            .get(key)
            .is_some_and(|item| item.as_bool().is_none())
        {
            return Err(shape(&format!("package.{key}")));
        }
    }
    let mode = package.get("materialization").and_then(Item::as_str);
    if if profile == LEGACY_ABSENT {
        mode != Some("snapshot")
    } else {
        !matches!(mode, None | Some("copy" | "hardlink" | "in-place"))
    } {
        return Err(shape("package.materialization"));
    }
    if let Some(item) = root.get("requires") {
        let requires = table(Some(item), "requires")?;
        only(requires, &["packages"], "requires")?;
        if let Some(item) = requires.get("packages") {
            let packages = table(Some(item), "requires.packages")?;
            for (key, item) in packages.iter() {
                let field = format!("requires.packages[{key}]");
                qualified_key(key, &field)?;
                if item.as_str().is_some() {
                    continue;
                }
                let entry = table(Some(item), &field)?;
                only(entry, &["version", "link"], &field)?;
                string(entry, "version", &format!("{field}.version"))?;
                if entry.contains_key("link")
                    && !matches!(
                        string(entry, "link", &format!("{field}.link"))?,
                        "static" | "dynamic" | "static-transitive" | "static-hard"
                    )
                {
                    return Err(shape(&format!("{field}.link")));
                }
            }
        }
    }
    Ok(())
}

// Frozen structural qualification only; Group/name semantic validity remains
// the current owning Manifest checker's responsibility.
fn qualified_key(key: &str, field: &str) -> Checked<()> {
    if key.contains('@') || key.matches(':').count() > 1 {
        return Err(shape(field));
    }
    let coordinate = if let Some((kind, coordinate)) = key.split_once(':') {
        if !matches!(kind, "flow" | "feat" | "stack" | "tool" | "mcp" | "lang") {
            return Err(shape(field));
        }
        coordinate
    } else {
        key
    };
    let Some((group, name)) = coordinate.split_once('/') else {
        return Err(shape(field));
    };
    if group.is_empty() || name.is_empty() || name.contains('/') {
        return Err(shape(field));
    }
    Ok(())
}

fn unprovable() -> MigrationRefusal {
    refusal(
        MigrationRefusalKind::UnprovableEdit,
        "package.materialization",
        "selected token span or replacement cannot be proved",
    )
}

fn materialization_edit(text: &str, document: &Document<&str>) -> Checked<MigrationTokenEdit> {
    let package = table(document.as_table().get("package"), "package")?;
    let value = package
        .get("materialization")
        .and_then(Item::as_value)
        .ok_or_else(unprovable)?;
    let span = value
        .span()
        .filter(|span| !span.is_empty())
        .ok_or_else(unprovable)?;
    let token = text.get(span.clone()).ok_or_else(unprovable)?;
    let delimiter = ["\"\"\"", "'''", "\"", "'"]
        .into_iter()
        .find(|delimiter| token.starts_with(delimiter) && token.ends_with(delimiter))
        .ok_or_else(unprovable)?;
    let newline = if delimiter.len() == 3 {
        let body = token.get(3..).ok_or_else(unprovable)?;
        if body.starts_with("\r\n") {
            "\r\n"
        } else if body.starts_with('\n') {
            "\n"
        } else {
            ""
        }
    } else {
        ""
    };
    let replacement = format!("{delimiter}{newline}copy{delimiter}");
    let checked: Value = replacement.parse().map_err(|_| unprovable())?;
    if value.as_str() != Some("snapshot") || checked.as_str() != Some("copy") {
        return Err(unprovable());
    }
    Ok(MigrationTokenEdit {
        field: "package.materialization",
        before_span: span,
        before_token: token.to_owned(),
        after_token: replacement,
    })
}

#[cfg(test)]
#[path = "migration/tests.rs"]
mod tests;
