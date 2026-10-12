//! Explicit pure change epoch proposal using the incumbent M13 replay law.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-MIGRATION-REPLAY");
use super::*;
use crate::manifest::MigrationTokenEdit;
pub const CHANGE_UPGRADE: &str = "change-schema0-to1-r1";
pub const CHANGE_IDENTITY: &str = "change-schema1-identity-r1";
#[derive(Debug, Clone, Copy)]
pub struct ChangeMigrationSelection<'a> {
    pub source_epoch: u32,
    pub target_epoch: u32,
    pub recipe: &'a str,
}
#[derive(Debug, Clone)]
pub struct ChangeMigrationPlan {
    pub recipe: &'static str,
    pub before: String,
    pub after: String,
    pub edits: Vec<MigrationTokenEdit>,
    pub checked: ChangeManifest,
}
pub fn plan_change_migration(
    text: &str,
    context: &ChangeContext,
    directory: &Path,
    selection: ChangeMigrationSelection<'_>,
) -> Result<ChangeMigrationPlan, ChangeError> {
    if selection.target_epoch != CHANGE_SCHEMA
        || ![0, CHANGE_SCHEMA].contains(&selection.source_epoch)
    {
        return Err(invalid(
            ChangeErrorKind::UnknownEpoch,
            "migration.selection",
            "unknown change migration edge",
        ));
    }
    let recipe = if selection.source_epoch == 0 {
        CHANGE_UPGRADE
    } else {
        CHANGE_IDENTITY
    };
    if selection.recipe != recipe {
        return Err(invalid(
            ChangeErrorKind::Migration,
            "migration.recipe",
            "unknown/incompatible change recipe",
        ));
    }
    ChangeManifest::parse_epoch(text, context, directory, selection.source_epoch)?;
    let edits = if selection.source_epoch == CHANGE_SCHEMA {
        vec![]
    } else {
        let document = toml_edit::Document::parse(text).map_err(|_| {
            invalid(
                ChangeErrorKind::Syntax,
                "migration.document",
                "invalid TOML",
            )
        })?;
        let span = document
            .get("schema")
            .and_then(toml_edit::Item::as_value)
            .and_then(toml_edit::Value::span)
            .ok_or_else(|| {
                invalid(
                    ChangeErrorKind::Migration,
                    "schema",
                    "schema token has no exact span",
                )
            })?;
        let before_token = text.get(span.clone()).ok_or_else(|| {
            invalid(
                ChangeErrorKind::Migration,
                "schema",
                "schema span outside captured text",
            )
        })?;
        vec![MigrationTokenEdit {
            field: "schema",
            before_span: span,
            before_token: before_token.into(),
            after_token: CHANGE_SCHEMA.to_string(),
        }]
    };
    let after = crate::manifest::replay_authored_tokens(text, &edits).map_err(|_| {
        invalid(
            ChangeErrorKind::Migration,
            "token replay",
            "M13 exact token edit refused",
        )
    })?;
    let checked = ChangeManifest::parse_str(&after, context, directory)?;
    Ok(ChangeMigrationPlan {
        recipe,
        before: text.into(),
        after,
        edits,
        checked,
    })
}
