//! Pure exact-token origin proposal, never an authored-file apply grant.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-EXPLICIT-REBASE");
use super::*;
use crate::manifest::MigrationTokenEdit;
#[derive(Debug, Clone)]
pub struct ChangeOriginRebasePlan {
    pub before: String,
    pub after: String,
    pub edits: Vec<MigrationTokenEdit>,
    pub checked: ChangeManifest,
}
/// Replace one explicitly selected origin's value tokens. Row insertions and
/// removals preserve existing comments and unrelated declarations.
/// No declaration is promoted to verified retained evidence by this helper.
pub fn plan_origin_rebase(
    text: &str,
    context: &ChangeContext,
    directory: &Path,
    expected: &ChangeOrigin,
    next: &ChangeOrigin,
) -> Result<ChangeOriginRebasePlan, ChangeError> {
    let before = ChangeManifest::parse_str(text, context, directory)?;
    let index = before
        .origins()
        .iter()
        .position(|o| o == expected)
        .ok_or_else(|| {
            invalid(
                ChangeErrorKind::Origin,
                "rebase.origin",
                "accepted declaration differs from saved intent",
            )
        })?;
    let parsed = toml_edit::Document::parse(text)
        .map_err(|_| invalid(ChangeErrorKind::Syntax, "rebase.document", "invalid TOML"))?;
    let origins = parsed
        .get("origins")
        .and_then(toml_edit::Item::as_array_of_tables)
        .ok_or_else(|| {
            invalid(
                ChangeErrorKind::Origin,
                "rebase.origins",
                "missing captured origin tokens",
            )
        })?;
    let origin = origins.get(index).ok_or_else(|| {
        invalid(
            ChangeErrorKind::Origin,
            "rebase.origin",
            "missing selected origin table",
        )
    })?;
    let mut edits = vec![];
    for (field, next_value) in [
        ("package", next.package.as_str()),
        ("manifest_sha256", next.manifest_sha256.as_str()),
    ] {
        value_edit(text, origin.get(field), field, next_value, &mut edits)?;
    }
    let content = next.selected_content_hash.as_deref().ok_or_else(|| {
        invalid(
            ChangeErrorKind::Origin,
            "rebase.selected_content_hash",
            "explicit retained content binding required",
        )
    })?;
    if origin.contains_key("selected_content_hash") {
        value_edit(
            text,
            origin.get("selected_content_hash"),
            "selected_content_hash",
            content,
            &mut edits,
        )?;
    } else {
        let end = origin
            .get("manifest_sha256")
            .and_then(toml_edit::Item::as_value)
            .and_then(toml_edit::Value::span)
            .ok_or_else(|| {
                invalid(
                    ChangeErrorKind::Origin,
                    "rebase.selected_content_hash",
                    "missing insertion basis",
                )
            })?
            .end;
        let insertion = text[end..].find('\n').map_or(text.len(), |i| end + i + 1);
        let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
        let prefix = if insertion == text.len() && !text.ends_with('\n') {
            eol
        } else {
            ""
        };
        edits.push(MigrationTokenEdit {
            field: "selected_content_hash",
            before_span: insertion..insertion,
            before_token: String::new(),
            after_token: format!(
                "{prefix}selected_content_hash={}{eol}",
                toml_edit::Value::from(content)
            ),
        });
    }
    let rows = origin
        .get("documents")
        .and_then(toml_edit::Item::as_array_of_tables)
        .ok_or_else(|| {
            invalid(
                ChangeErrorKind::Origin,
                "rebase.documents",
                "missing captured document tokens",
            )
        })?;
    for (i, next_doc) in next.documents.iter().take(rows.len()).enumerate() {
        let row = rows.get(i).ok_or_else(|| {
            invalid(
                ChangeErrorKind::Origin,
                "rebase.documents",
                "missing document table",
            )
        })?;
        value_edit(
            text,
            row.get("address"),
            "address",
            &next_doc.address,
            &mut edits,
        )?;
        value_edit(
            text,
            row.get("sha256"),
            "sha256",
            &next_doc.sha256,
            &mut edits,
        )?;
    }
    for row in rows.iter().skip(next.documents.len()) {
        remove_document_tokens(text, row, &mut edits)?;
    }
    if next.documents.len() > rows.len() {
        let last = rows.iter().last().ok_or_else(|| {
            invalid(
                ChangeErrorKind::Origin,
                "rebase.documents",
                "missing insertion basis",
            )
        })?;
        let end = last
            .get("sha256")
            .and_then(toml_edit::Item::as_value)
            .and_then(toml_edit::Value::span)
            .ok_or_else(|| {
                invalid(
                    ChangeErrorKind::Origin,
                    "rebase.documents",
                    "missing last document value span",
                )
            })?
            .end;
        let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
        let insertion = text[end..].find('\n').map_or(text.len(), |i| end + i + 1);
        let mut extra = if insertion == text.len() && !text.ends_with('\n') {
            eol.into()
        } else {
            String::new()
        };
        for doc in next.documents.iter().skip(rows.len()) {
            extra.push_str(&format!(
                "[[origins.documents]]{eol}address={}{eol}sha256={}{eol}",
                toml_edit::Value::from(doc.address.as_str()),
                toml_edit::Value::from(doc.sha256.as_str())
            ));
        }
        edits.push(MigrationTokenEdit {
            field: "origins.documents",
            before_span: insertion..insertion,
            before_token: String::new(),
            after_token: extra,
        });
    }
    edits.sort_by_key(|e| e.before_span.start);
    let after = crate::manifest::replay_authored_tokens(text, &edits).map_err(|_| {
        invalid(
            ChangeErrorKind::Migration,
            "rebase.tokens",
            "M13 exact token replay refused",
        )
    })?;
    let checked = ChangeManifest::parse_str(&after, context, directory)?;
    Ok(ChangeOriginRebasePlan {
        before: text.into(),
        after,
        edits,
        checked,
    })
}
fn remove_document_tokens(
    text: &str,
    row: &toml_edit::Table,
    edits: &mut Vec<MigrationTokenEdit>,
) -> Result<(), ChangeError> {
    let span = row.span().ok_or_else(|| {
        invalid(
            ChangeErrorKind::Origin,
            "rebase.documents",
            "missing table span",
        )
    })?;
    let raw = text.get(span.clone()).ok_or_else(|| {
        invalid(
            ChangeErrorKind::Origin,
            "rebase.documents",
            "table span outside text",
        )
    })?;
    if !raw.starts_with("[[") {
        return Err(invalid(
            ChangeErrorKind::Origin,
            "rebase.documents",
            "table header span unavailable",
        ));
    }
    let header_end = raw.find("]]").ok_or_else(|| {
        invalid(
            ChangeErrorKind::Origin,
            "rebase.documents",
            "table header span unavailable",
        )
    })? + 2;
    edits.push(MigrationTokenEdit {
        field: "origins.documents",
        before_span: span.start..span.start + header_end,
        before_token: raw[..header_end].into(),
        after_token: String::new(),
    });
    for field in ["address", "sha256"] {
        let key = row
            .key(field)
            .and_then(toml_edit::Key::span)
            .ok_or_else(|| invalid(ChangeErrorKind::Origin, field, "missing document key span"))?;
        let value = row
            .get(field)
            .and_then(toml_edit::Item::as_value)
            .and_then(toml_edit::Value::span)
            .ok_or_else(|| {
                invalid(
                    ChangeErrorKind::Origin,
                    field,
                    "missing document value span",
                )
            })?;
        let span = key.start..value.end;
        let token = text
            .get(span.clone())
            .ok_or_else(|| invalid(ChangeErrorKind::Origin, field, "document span outside text"))?;
        edits.push(MigrationTokenEdit {
            field,
            before_span: span,
            before_token: token.into(),
            after_token: String::new(),
        });
    }
    Ok(())
}
fn value_edit(
    text: &str,
    item: Option<&toml_edit::Item>,
    field: &'static str,
    next: &str,
    edits: &mut Vec<MigrationTokenEdit>,
) -> Result<(), ChangeError> {
    let value = item
        .and_then(toml_edit::Item::as_value)
        .ok_or_else(|| invalid(ChangeErrorKind::Origin, field, "missing captured value"))?;
    let span = value
        .span()
        .ok_or_else(|| invalid(ChangeErrorKind::Origin, field, "missing exact value span"))?;
    let original = text
        .get(span.clone())
        .ok_or_else(|| invalid(ChangeErrorKind::Origin, field, "span outside text"))?;
    if value.as_str() == Some(next) {
        return Ok(());
    }
    edits.push(MigrationTokenEdit {
        field,
        before_span: span,
        before_token: original.into(),
        after_token: toml_edit::Value::from(next).to_string(),
    });
    Ok(())
}
