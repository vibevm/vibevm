//! Frozen package-role grammar and its exact scalar rename.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-PLAN");
use super::*;
use toml_edit::{Document, Item, TableLike, Value};

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

pub(super) fn check_shape(document: &Document<&str>, profile: &str) -> Checked<()> {
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

pub(super) fn materialization_edit(
    text: &str,
    document: &Document<&str>,
) -> Checked<MigrationTokenEdit> {
    let unprovable = || unprovable("package.materialization");
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
