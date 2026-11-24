//! The rows of an install plan, shared by `vibe install` and
//! `vibe install --plan-only` (PROP-067 `#list`, `#heading`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#list");

use vibe_wire::generated::install::e2::plan::{RemapPlanEntry, RemapPlanOutcome};

use crate::output::language::{Cell, List, Role, Row};

/// One package of a plan.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(crate) struct PlanRow {
    /// `group/name`.
    pub(crate) package: String,
    /// The selected version.
    pub(crate) version: String,
    /// The registry the source came from, when known.
    pub(crate) registry: Option<String>,
}

/// `plan for 1 package` / `plan for 3 packages`.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#heading")]
pub(crate) fn plan_label(count: usize) -> String {
    format!(
        "plan for {count} package{}",
        if count == 1 { "" } else { "s" }
    )
}

/// One declaration borrowed from selected preview metadata, now owned for display.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(crate) struct ContributionRow {
    /// Selected package identity; ids alone are not globally unique.
    pub(crate) package: String,
    /// Selected package version.
    pub(crate) version: String,
    /// Declared contribution id.
    pub(crate) id: String,
    /// Declared handler kind, not a guessed execution status.
    pub(crate) handler: String,
    /// Optional authored title, never synthesized from the id.
    pub(crate) title: Option<String>,
}

/// The packages as one measured list: name, version, registry.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(crate) fn plan_list(rows: &[PlanRow], width: usize) -> String {
    preview_list(rows, &[], width)
}

/// Selected packages with declarations beneath their exact owner, in held order.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(crate) fn preview_list(
    rows: &[PlanRow],
    contributions: &[ContributionRow],
    width: usize,
) -> String {
    List::new(
        rows.iter()
            .map(|row| {
                let mut package = Row::new(vec![
                    Cell::new(row.package.clone(), Role::Accent),
                    Cell::text(row.version.clone()),
                    Cell::new(
                        row.registry
                            .clone()
                            .unwrap_or_else(|| "unknown source".to_owned()),
                        Role::Dim,
                    ),
                ]);
                for (index, declaration) in contributions
                    .iter()
                    .filter(|declaration| {
                        declaration.package == row.package && declaration.version == row.version
                    })
                    .enumerate()
                {
                    package = package.child(format!(
                        "{}. {}  ({})  {}",
                        index + 1,
                        declaration.id,
                        declaration.handler,
                        declaration.title.as_deref().unwrap_or_default()
                    ));
                }
                package
            })
            .collect(),
    )
    .render_at(width)
}

/// The remaps of a preview as one measured list: package, version, outcome.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(crate) fn remap_list(remaps: &[RemapPlanEntry], width: usize) -> String {
    List::new(
        remaps
            .iter()
            .map(|remap| {
                let outcome = match &remap.outcome {
                    RemapPlanOutcome::OfferOnly => "offer only".to_owned(),
                    RemapPlanOutcome::NeedsInput => "needs input".to_owned(),
                    RemapPlanOutcome::Unknown(word) => word.clone(),
                };
                Row::new(vec![
                    Cell::new(remap.package.clone(), Role::Accent),
                    Cell::text(remap.version.clone().unwrap_or_default()),
                    Cell::new(outcome, Role::Dim),
                ])
            })
            .collect(),
    )
    .render_at(width)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_only_renders_rule_label_and_measured_rows() {
        let rows = [
            PlanRow {
                package: "org.example/x".into(),
                version: "1.0.0".into(),
                registry: Some("corp".into()),
            },
            PlanRow {
                package: "org.example/longer-name".into(),
                version: "12.3.4".into(),
                registry: None,
            },
        ];
        assert_eq!(plan_label(1), "plan for 1 package");
        assert_eq!(plan_label(2), "plan for 2 packages");
        assert_eq!(
            plan_list(&rows, 100),
            "  org.example/x            1.0.0   corp\n  org.example/longer-name  12.3.4  unknown source"
        );
    }
}
