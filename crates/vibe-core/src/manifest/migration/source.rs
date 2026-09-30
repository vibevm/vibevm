//! One finite old boot-source declaration, routed through the M-13 planner.
//!
//! `format = "normal"` and `[boot_snippet].source` establish that this exact
//! boot file is Vibe source. They say nothing about its media type. No suffix
//! or file content participates in this migration decision.

use std::path::Path;

use toml_edit::{Document, Item};

use super::*;
use crate::manifest::{
    SourceInterpretation, SourceLoading, SourceRole, SourceSelector, declarant_path,
};

pub(super) fn check_source_shape(document: &Document<&str>, profile: &str) -> Checked<()> {
    let root = document.as_table();
    only(root, &["package", "boot_snippet"], "document")?;
    let package = table(root.get("package"), "package")?;
    let keys = if profile == SOURCE_BOOT_LEGACY {
        &[
            "name",
            "group",
            "kind",
            "version",
            "format",
            "description",
            "license",
        ][..]
    } else {
        &[
            "name",
            "group",
            "kind",
            "version",
            "format",
            "description",
            "license",
            "sources",
        ][..]
    };
    only(package, keys, "package")?;
    for key in ["name", "group", "version"] {
        string(package, key, &format!("package.{key}"))?;
    }
    if string(package, "kind", "package.kind")? != "flow"
        || string(package, "format", "package.format")? != "normal"
    {
        return Err(shape("package.kind|format"));
    }
    for key in ["description", "license"] {
        if package.contains_key(key) {
            string(package, key, &format!("package.{key}"))?;
        }
    }
    let boot = table(root.get("boot_snippet"), "boot_snippet")?;
    only(boot, &["source"], "boot_snippet")?;
    let path = string(boot, "source", "boot_snippet.source")?;
    declarant_path(Path::new(path)).map_err(|_| shape("boot_snippet.source"))?;
    if profile == SOURCE_BOOT_LEGACY {
        if package.contains_key("sources") {
            return Err(shape("package.sources"));
        }
        return Ok(());
    }
    let rows = package
        .get("sources")
        .and_then(Item::as_array_of_tables)
        .ok_or_else(|| shape("package.sources"))?;
    if rows.len() != 1 {
        return Err(shape("package.sources"));
    }
    let row = rows.get(0).ok_or_else(|| shape("package.sources"))?;
    only(
        row,
        &["file", "media", "interpretation", "role", "loading"],
        "package.sources[0]",
    )?;
    if string(row, "file", "package.sources[0].file")? != path
        || !matches!(
            string(row, "media", "package.sources[0].media")?,
            "application/xml" | "text/markdown"
        )
        || string(row, "interpretation", "package.sources[0].interpretation")? != "vibe-spec"
        || string(row, "role", "package.sources[0].role")? != "spec"
        || string(row, "loading", "package.sources[0].loading")? != "explicit"
    {
        return Err(shape("package.sources[0]"));
    }
    Ok(())
}

pub(super) fn source_insert_edit(
    text: &str,
    document: &Document<&str>,
    decisions: &[SourceDescriptor],
) -> Checked<Result<MigrationTokenEdit, MigrationInputRequirement>> {
    let boot = table(document.as_table().get("boot_snippet"), "boot_snippet")?;
    let path = string(boot, "source", "boot_snippet.source")?;
    if decisions.is_empty() {
        return Ok(Err(MigrationInputRequirement {
            field: "source_media",
            choices: vec!["application/xml", "text/markdown"],
            resource_path: Some(path.to_owned()),
        }));
    }
    if decisions.len() != 1 {
        return Err(shape("source_decisions"));
    }
    let decision = &decisions[0];
    if !matches!(decision.selector(), SourceSelector::File(file) if file == path) {
        return Err(shape("source_decisions.file"));
    }
    if !matches!(decision.media(), "application/xml" | "text/markdown")
        || decision.interpretation() != SourceInterpretation::VibeSpec
        || decision.role() != SourceRole::Spec
        || decision.loading() != SourceLoading::Explicit
    {
        return Err(shape("source_decisions"));
    }
    let newline = if text.contains("\r\n") && text.replace("\r\n", "").find('\n').is_none() {
        "\r\n"
    } else {
        "\n"
    };
    let prefix = if text.ends_with('\n') {
        newline
    } else {
        "\n\n"
    };
    let path_token = Value::from(path).to_string();
    let media_token = Value::from(decision.media()).to_string();
    let insertion = format!(
        "{prefix}[[package.sources]]{newline}file = {path_token}{newline}media = {media_token}{newline}interpretation = \"vibe-spec\"{newline}role = \"spec\"{newline}loading = \"explicit\"{newline}",
    );
    Ok(Ok(MigrationTokenEdit {
        field: "package.sources",
        before_span: text.len()..text.len(),
        before_token: String::new(),
        after_token: insertion,
    }))
}
