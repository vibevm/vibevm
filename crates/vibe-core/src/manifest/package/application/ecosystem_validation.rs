//! Validation of `[[application.ecosystem]]` and its `recommends` rows
//! (PROP-075 `#ECOSYSTEM-ADAPTER`, `#ECOSYSTEM-RECOMMENDS`). Every refusal
//! names its field as a TOML path in the sentence itself, the rule of
//! PROP-059 `#declaration`.
//!
//! The one refusal that is a *law* and not a style note is the `recipe`
//! restriction on `install` and `remove`: items are appended to the step's
//! argv as separate elements and are never interpolated into a command
//! string, and a `shell` step is a command string into which a list could
//! only be spliced. Refusing the declaration is what makes the injection
//! rule structural.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-RECOMMENDS");

use std::collections::BTreeSet;

use crate::manifest::{
    ApplicationDecl, EcosystemDecl, EcosystemVerb, FieldError, FieldPath, StepDecl, StepKind,
    StepPhase, is_portable_token,
};

impl ApplicationDecl {
    /// `[[application.ecosystem]]`: unique opaque ids, a nonempty name, up
    /// to four verbs of a permitted kind with `install` and `remove`
    /// restricted to `recipe`, and recommendation rows naming a declared
    /// adapter (PROP-075 `#ECOSYSTEM-ADAPTER`, `#ECOSYSTEM-RECOMMENDS`).
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER",
        r = 1
    )]
    pub(super) fn validate_ecosystems(&self, at: &FieldPath) -> Result<(), FieldError> {
        let rows = at.table("ecosystem");
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for (position, row) in self.ecosystem.iter().enumerate() {
            let row_at = rows.index(position);
            let refuse = |name: &str, message: String| {
                let at = row_at.field(name);
                FieldError::new(at.clone(), format!("{at}: {message}"))
            };
            if !is_portable_token(&row.id) {
                return Err(refuse(
                    "id",
                    format!(
                        "ecosystem id `{}` must be a portable lowercase token; vibe never \
                         interprets it and only ever echoes it",
                        row.id
                    ),
                ));
            }
            if !seen.insert(row.id.as_str()) {
                return Err(refuse(
                    "id",
                    format!(
                        "ecosystem id `{}` is declared twice; each adapter is addressed by its \
                         own `id`",
                        row.id
                    ),
                ));
            }
            if row.name.trim().is_empty() {
                return Err(refuse(
                    "name",
                    format!("ecosystem `{}` name must be nonempty", row.id),
                ));
            }
            for verb in EcosystemVerb::all() {
                if let Some(step) = row.verb(verb) {
                    validate_verb(&row_at, row, verb, step)?;
                }
            }
            self.validate_recommends(&row_at, row)?;
        }
        Ok(())
    }

    /// `[[application.ecosystem.recommends]]`: a portable id unique across
    /// the package, a nonempty opaque item, and an `ecosystem` naming an
    /// adapter the package declares.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-RECOMMENDS",
        r = 1
    )]
    fn validate_recommends(
        &self,
        row_at: &FieldPath,
        row: &EcosystemDecl,
    ) -> Result<(), FieldError> {
        let rows = row_at.table("recommends");
        for (position, recommend) in row.recommends.iter().enumerate() {
            let at = rows.index(position);
            let refuse = |name: &str, message: String| {
                let at = at.field(name);
                FieldError::new(at.clone(), format!("{at}: {message}"))
            };
            if !is_portable_token(&recommend.id) {
                return Err(refuse(
                    "id",
                    format!(
                        "recommendation id `{}` must be a portable lowercase token; it is the id \
                         of the feature the row is shorthand for",
                        recommend.id
                    ),
                ));
            }
            if self
                .ecosystem_recommends()
                .filter(|other| other.id == recommend.id)
                .count()
                > 1
            {
                return Err(refuse(
                    "id",
                    format!(
                        "recommendation id `{}` is declared twice; each row synthesises one \
                         feature of that id",
                        recommend.id
                    ),
                ));
            }
            if recommend.item.trim().is_empty() {
                return Err(refuse(
                    "item",
                    format!(
                        "recommendation `{}` item must be nonempty; it is the opaque string \
                         handed to the adapter",
                        recommend.id
                    ),
                ));
            }
            if self.ecosystem_adapter(&recommend.ecosystem).is_none() {
                return Err(refuse(
                    "ecosystem",
                    format!(
                        "recommendation `{}` names ecosystem `{}`, which this package does not \
                         declare; declared: {}",
                        recommend.id,
                        recommend.ecosystem,
                        declared_ids(self)
                    ),
                ));
            }
        }
        Ok(())
    }
}

/// The declared adapter ids, for a refusal that has to name them.
fn declared_ids(application: &ApplicationDecl) -> String {
    if application.ecosystem.is_empty() {
        return "none".to_owned();
    }
    application
        .ecosystem
        .iter()
        .map(|row| format!("`{}`", row.id))
        .collect::<Vec<_>>()
        .join(", ")
}

/// One verb of one adapter: a `recipe` or `shell` step of the ordinary
/// grammar — `recipe` only for `install` and `remove` — that needs no
/// feature, parameter, requirement or phase the adapter runner does not
/// have.
fn validate_verb(
    row_at: &FieldPath,
    row: &EcosystemDecl,
    verb: EcosystemVerb,
    step: &StepDecl,
) -> Result<(), FieldError> {
    let verb_name = verb.as_str();
    let at = row_at.table(verb_name);
    let refuse = |name: &str, message: String| {
        let at = at.field(name);
        FieldError::new(at.clone(), format!("{at}: {message}"))
    };
    if !matches!(step.kind, StepKind::Recipe { .. } | StepKind::Shell { .. }) {
        return Err(refuse(
            "kind",
            format!(
                "ecosystem `{}` {verb_name} verb is `kind = \"{}\"`; an adapter runs a command \
                 inside an installed runtime and takes only `recipe` or `shell`",
                row.id,
                step.kind.name()
            ),
        ));
    }
    if verb.takes_items() && !matches!(step.kind, StepKind::Recipe { .. }) {
        return Err(refuse(
            "kind",
            format!(
                "ecosystem `{}` {verb_name} verb is `kind = \"{}\"`; it must be \
                 `kind = \"recipe\"`, because the items are appended to the step's argv as \
                 separate elements and are never interpolated into a command string — a `shell` \
                 step is a command string into which a list could only be spliced",
                row.id,
                step.kind.name()
            ),
        ));
    }
    if !is_portable_token(&step.id) {
        return Err(refuse(
            "id",
            format!(
                "ecosystem `{}` {verb_name} step id `{}` must be a portable lowercase token",
                row.id, step.id
            ),
        ));
    }
    if step.title.trim().is_empty() {
        return Err(refuse(
            "title",
            format!(
                "ecosystem `{}` {verb_name} step title must be nonempty",
                row.id
            ),
        ));
    }
    if !step.requires.is_empty() {
        return Err(refuse(
            "requires",
            format!(
                "ecosystem `{}` {verb_name} step declares `requires`; an adapter verb's PATH is \
                 the configured installation's, never a resolved requirement's",
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
                "ecosystem `{}` {verb_name} step declares a condition; an adapter verb runs \
                 because it was asked for, not because a feature is selected",
                row.id
            ),
        ));
    }
    if step.phase != StepPhase::Prepare {
        return Err(refuse(
            "phase",
            format!(
                "ecosystem `{}` {verb_name} step declares `phase`; an adapter verb has no \
                 prepare and finish split",
                row.id
            ),
        ));
    }
    Ok(())
}
