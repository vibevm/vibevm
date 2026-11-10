//! Validation of `[[application.migration]]` (PROP-075
//! `#MIGRATION-DECLARATION`): stable unique ids, a nonempty name, and the
//! two step declarations restricted to the kinds `recipe` and `shell`.
//! Every refusal names its field as a TOML path in the sentence itself,
//! the rule of PROP-059 `#declaration`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-DECLARATION");

use std::collections::BTreeSet;

use crate::manifest::{
    ApplicationDecl, ECOSYSTEM_MIGRATION_PREFIX, FieldError, FieldPath, MigrationDecl, StepDecl,
    StepKind, StepPhase, is_portable_token,
};

impl ApplicationDecl {
    /// `[[application.migration]]`: unique portable ids, a nonempty name,
    /// and an `export` / `import` step of a permitted kind that asks for
    /// nothing the migration runner cannot give it (PROP-075
    /// `#MIGRATION-DECLARATION`).
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-075#MIGRATION-DECLARATION",
        r = 1
    )]
    pub(super) fn validate_migrations(&self, at: &FieldPath) -> Result<(), FieldError> {
        let rows = at.table("migration");
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for (position, row) in self.migration.iter().enumerate() {
            let row_at = rows.index(position);
            let refuse = |name: &str, message: String| {
                let at = row_at.field(name);
                FieldError::new(at.clone(), format!("{at}: {message}"))
            };
            if row.id.starts_with(ECOSYSTEM_MIGRATION_PREFIX) {
                // The namespace is reserved so a synthesised adapter entry
                // can never be shadowed (PROP-075 `#ECOSYSTEM-MIGRATION`).
                return Err(refuse(
                    "id",
                    format!(
                        "migration id `{}` is in the reserved `{ECOSYSTEM_MIGRATION_PREFIX}` \
                         namespace; vibe synthesises those entries from \
                         `[[application.ecosystem]]`",
                        row.id
                    ),
                ));
            }
            if !is_portable_token(&row.id) {
                return Err(refuse(
                    "id",
                    format!(
                        "migration id `{}` must be a portable lowercase token",
                        row.id
                    ),
                ));
            }
            if !seen.insert(row.id.as_str()) {
                return Err(refuse(
                    "id",
                    format!(
                        "migration id `{}` is declared twice; `--only ID` names exactly one entry",
                        row.id
                    ),
                ));
            }
            if row.name.trim().is_empty() {
                return Err(refuse(
                    "name",
                    format!("migration `{}` name must be nonempty", row.id),
                ));
            }
            for (side, step) in row.sides() {
                validate_side(&row_at, row, side.as_str(), step)?;
            }
        }
        Ok(())
    }
}

/// One side of a migration entry: a `recipe` or `shell` step of the
/// ordinary grammar that needs no feature, parameter, requirement or
/// phase the migration runner does not have.
fn validate_side(
    row_at: &FieldPath,
    row: &MigrationDecl,
    side: &str,
    step: &StepDecl,
) -> Result<(), FieldError> {
    let at = row_at.table(side);
    let refuse = |name: &str, message: String| {
        let at = at.field(name);
        FieldError::new(at.clone(), format!("{at}: {message}"))
    };
    if !matches!(step.kind, StepKind::Recipe { .. } | StepKind::Shell { .. }) {
        return Err(refuse(
            "kind",
            format!(
                "migration `{}` {side} step is `kind = \"{}\"`; a migration moves user state and \
                 takes only `recipe` or `shell`",
                row.id,
                step.kind.name()
            ),
        ));
    }
    if !is_portable_token(&step.id) {
        return Err(refuse(
            "id",
            format!(
                "migration `{}` {side} step id `{}` must be a portable lowercase token",
                row.id, step.id
            ),
        ));
    }
    if step.title.trim().is_empty() {
        return Err(refuse(
            "title",
            format!("migration `{}` {side} step title must be nonempty", row.id),
        ));
    }
    if !step.requires.is_empty() {
        return Err(refuse(
            "requires",
            format!(
                "migration `{}` {side} step declares `requires`; a migration step's PATH is the \
                 old and new installations', never a resolved requirement's",
                row.id
            ),
        ));
    }
    if !step.when.is_empty() || !step.unless.is_empty() {
        return Err(refuse(
            if step.when.is_empty() {
                "unless"
            } else {
                "when"
            },
            format!(
                "migration `{}` {side} step declares a condition; a migration selects entries by \
                 `--only ID`, not by features or parameters",
                row.id
            ),
        ));
    }
    if step.phase != StepPhase::Prepare {
        return Err(refuse(
            "phase",
            format!(
                "migration `{}` {side} step declares `phase`; a migration has no prepare and \
                 finish split",
                row.id
            ),
        ));
    }
    Ok(())
}
