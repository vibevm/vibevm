//! The judgment a `slot:` contribution carries — the four fields an
//! ordinary package's multi-step installer was missing (PROP-065
//! `##ordinary-grammar`, PROP-054 `##SLOT-CONTRIBUTION-JUDGMENT`).
//!
//! What runs stays the handler taxonomy of PROP-054 `##HANDLER-KINDS`;
//! what is shared with the application plane is only whether and in what
//! order something runs, and how it is seen. So a `slot:` row gains exactly
//! `title`, `when` / `unless` over PROP-065 `##conditions`' closed
//! vocabulary, `needs`, and `optional` — and the fields of the application
//! plane that cannot mean anything here are refused **by name** rather than
//! ignored, the precedent PROP-075 `##P6-AS-BUILT` set for a migration
//! step's inapplicable fields.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-054#SLOT-CONTRIBUTION-JUDGMENT");

use specmark::spec;

use crate::judgment::{Condition, Need};

pub(super) const SLOT_JUDGMENT: &str =
    "spec://org.vibevm.core/vibevm/common/PROP-054#SLOT-CONTRIBUTION-JUDGMENT";
const ORDINARY_GRAMMAR: &str = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-grammar";

/// Fields of the application step plane that a `slot:` contribution refuses
/// by name, each with the reason it cannot mean anything here.
pub(super) const REFUSED_FIELDS: [(&str, &str); 3] = [
    (
        "requires",
        "installing an application from inside an ordinary package's materialisation is a second \
         install plane; use `needs = [\"command:NAME\"]`, which refuses up front and names what is \
         missing",
    ),
    (
        "phase",
        "the point already is the phase — `slot:pre-install` is PROP-065's `prepare` and \
         `slot:post-install` its `finish`",
    ),
    (
        "result",
        "an ordinary package has no payload, no app root and no `$VIBE_OUT`; its materialised slot \
         is its content",
    ),
];

/// Template variables of the application plane, refused anywhere in a
/// `slot:` contribution because none of the things they name exists here.
pub(super) const REFUSED_VARIABLES: [&str; 4] = ["$out", "$sources", "$app_root", "$app_bin"];

/// The judgment half of one `slot:` contribution.
///
/// Present exactly when the row declared at least one of the four fields;
/// a row that declared none carries `None`, so every manifest written
/// before this grammar serialises byte-identically.
///
/// ```
/// use vibe_core::manifest::SlotJudgment;
///
/// let judgment = SlotJudgment::default();
/// assert!(judgment.title.is_none());
/// assert!(judgment.when.is_empty());
/// assert!(!judgment.optional);
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-grammar")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SlotJudgment {
    /// The human line a plan and a panel print beside the `id`.
    pub title: Option<String>,
    /// Conditions that must all hold for the contribution to run.
    pub when: Vec<Condition>,
    /// Conditions of which none may hold.
    pub unless: Vec<Condition>,
    /// What must be true before anything starts.
    pub needs: Vec<Need>,
    /// An unmet need or a failure is recorded and does not stop the rest.
    pub optional: bool,
}

impl SlotJudgment {
    /// Whether the row declared nothing at all, in which case the carrier is
    /// dropped rather than written as an empty table.
    ///
    /// ```
    /// use vibe_core::manifest::SlotJudgment;
    ///
    /// assert!(SlotJudgment::default().is_empty());
    /// ```
    /// The name of the first of the four fields this row declared — what a
    /// refusal at a non-`slot:` point names.
    ///
    /// ```
    /// use vibe_core::manifest::SlotJudgment;
    ///
    /// let judgment = SlotJudgment {
    ///     optional: true,
    ///     ..SlotJudgment::default()
    /// };
    /// assert_eq!(judgment.first_declared_field(), "optional");
    /// ```
    pub fn first_declared_field(&self) -> &'static str {
        if self.title.is_some() {
            "title"
        } else if !self.when.is_empty() {
            "when"
        } else if !self.unless.is_empty() {
            "unless"
        } else if !self.needs.is_empty() {
            "needs"
        } else {
            "optional"
        }
    }

    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.when.is_empty()
            && self.unless.is_empty()
            && self.needs.is_empty()
            && !self.optional
    }
}

/// Parse one authored `when` / `unless` table into the closed vocabulary.
///
/// One key is one condition, and **every** key of the table must hold
/// (`when`) or none of them may (`unless`) — the conjunction PROP-065
/// `##conditions` gives an array of tables, expressed in the one table this
/// grammar's field already carries. A key outside the vocabulary is a
/// refusal naming it, never a silently ignored declaration.
///
/// ```
/// use vibe_core::manifest::parse_slot_conditions;
///
/// let table = toml::from_str(r#"command_present = "cargo""#).unwrap();
/// let parsed = parse_slot_conditions(&table, "when", "build").unwrap();
/// assert_eq!(parsed.len(), 1);
///
/// let bad: toml::Table = toml::from_str("sunny = true").unwrap();
/// assert!(parse_slot_conditions(&bad, "when", "build").is_err());
/// ```
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#conditions")]
pub fn parse_slot_conditions(
    table: &toml::Table,
    field: &str,
    id: &str,
) -> Result<Vec<Condition>, String> {
    let mut conditions = Vec::with_capacity(table.len());
    for (key, value) in table {
        let mut single = toml::Table::new();
        single.insert(key.clone(), value.clone());
        let condition: Condition = toml::Value::Table(single).try_into().map_err(|error| {
            format!(
                "[[extension]] `{id}` field `{field}` key `{key}` is not a condition of the \
                     closed vocabulary: {error} ({ORDINARY_GRAMMAR})"
            )
        })?;
        conditions.push(condition);
    }
    Ok(conditions)
}

/// Refuse any application-plane template variable in one authored string.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-grammar")]
pub(super) fn refuse_variables(value: &str, field: &str, id: &str) -> Result<(), String> {
    for variable in REFUSED_VARIABLES {
        if value.contains(variable) {
            return Err(format!(
                "[[extension]] `{id}` field `{field}` names template variable `{variable}` at a \
                 `slot:` point; an ordinary package has no payload, no sources and no app root, so \
                 none of `{}` is defined here ({ORDINARY_GRAMMAR})",
                REFUSED_VARIABLES.join("`, `"),
            ));
        }
    }
    Ok(())
}

/// Every authored string of the judgment half, scanned for the refused
/// variables.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-grammar")]
pub(super) fn refuse_variables_in(judgment: &SlotJudgment, id: &str) -> Result<(), String> {
    if let Some(title) = &judgment.title {
        refuse_variables(title, "title", id)?;
    }
    for (field, conditions) in [("when", &judgment.when), ("unless", &judgment.unless)] {
        for condition in conditions {
            if let Condition::FilePresent(path) = condition {
                refuse_variables(path, field, id)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
