//! The plan as a terminal shows it before the first step runs (PROP-065
//! `#plan`), drawn in the display language (PROP-067 `#heading`, `#list`,
//! `#children`): a rule naming the package, its fields as one list, then
//! the steps as a list with their requirements as children.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#plan");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#list");

use vibe_app::{Plan, PlannedDependency};
use vibe_core::manifest::StepPhase;

use crate::output::language::{Cell, Glyph, List, Meta, Role, Row, bytes, rule, warning};

/// The plan's blocks, separated by one blank line each. `invoker` is the
/// `--invoked-by` tool, the one right label a command's first rule may
/// carry.
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-067#heading")]
pub(crate) fn render(plan: &Plan, invoker: Option<&str>) -> String {
    let mut label = format!("plan for {}", plan.package.key());
    // A requirement's own plan is printed between the requiring plan and
    // its first step, so its rule says whose it is.
    if let Some(owner) = &plan.required_by {
        label.push_str(&format!(" required by {}", owner.package));
        if let Some(step) = &owner.step {
            label.push_str(&format!(" for step {step}"));
        }
    }
    let right = invoker.map(|tool| format!("installed by {tool}"));
    let mut blocks = vec![rule(&label, right.as_deref()), fields(plan)];
    if !plan.steps.is_empty() {
        blocks.push(steps(plan));
    }
    if !plan.warnings.is_empty() {
        // Identifiers lose their backticks in human output (`#inline`).
        let lines: Vec<String> = plan
            .warnings
            .iter()
            .map(|text| warning(&text.replace('`', "")))
            .collect();
        blocks.push(lines.join("\n"));
    }
    blocks.join("\n\n")
}

/// The installer, license, profile, features, parameters, download and
/// option-level requirements, one row each with the name in its own column.
fn fields(plan: &Plan) -> String {
    let mut rows = Vec::new();
    let mut field = |name: &str, value: String, meta: Vec<Meta>| {
        rows.push(Row::new(vec![Cell::new(name, Role::Dim), Cell::text(value)]).meta(meta));
    };
    let result = match plan.result.external_root() {
        Some(root) => format!("result external {root}"),
        None => "result owned".into(),
    };
    field(
        "installer",
        format!("{} ({}/{})", plan.option, plan.platform, plan.arch),
        vec![Meta::Note(result)],
    );
    if let Some(license) = &plan.license {
        field(
            "license",
            license.name.clone(),
            vec![Meta::State("accepted".into())],
        );
    }
    if let Some(profile) = &plan.profile {
        field("profile", profile.clone(), vec![]);
    }
    if !plan.declared.is_empty() {
        let features: Vec<String> = plan
            .declared
            .iter()
            .map(|feature| {
                let mark = if plan.features.contains(&feature.id) {
                    "x"
                } else {
                    " "
                };
                let why = plan
                    .implied
                    .iter()
                    .find(|implied| implied.feature == feature.id)
                    .map(|implied| match &implied.by {
                        Some(by) => format!(" ({} of {by})", implied.rule.as_str()),
                        None => format!(" ({})", implied.rule.as_str()),
                    })
                    .unwrap_or_default();
                format!("[{mark}] {}{why}", feature.id)
            })
            .collect();
        field("features", features.join("  "), vec![]);
    }
    for param in &plan.parameters {
        let value = if param.secret {
            "(secret)"
        } else {
            &param.value
        };
        field("parameter", format!("{} = {value}", param.id), vec![]);
    }
    if plan.download_bytes > 0 || plan.unknown_sizes > 0 || plan.drop_inputs > 0 {
        let mut meta = Vec::new();
        if plan.unknown_sizes > 0 {
            meta.push(Meta::Note(format!(
                "{} input(s) of unknown size",
                plan.unknown_sizes
            )));
        }
        // An input the drop already holds is copied, not transferred, so
        // the download it is NOT part of says where it comes from instead
        // (`PROP-066#drops`).
        if plan.drop_inputs > 0 {
            meta.push(Meta::Note(format!(
                "{} input(s) from the [artifacts] packages drop",
                plan.drop_inputs
            )));
        }
        field("download", bytes(plan.download_bytes), meta);
    }
    for dependency in plan.dependencies.iter().filter(|d| d.step.is_none()) {
        let (text, meta) = requirement(dependency);
        field("requires", text, meta);
    }
    List::new(rows).render()
}

/// The steps: number, id, kind, title, then `needs …` and the note as
/// metadata; a skipped step is a dim skipped row whose reason is its metadata;
/// each requirement a child under the id column.
fn steps(plan: &Plan) -> String {
    let rows = plan
        .steps
        .iter()
        .enumerate()
        .map(|(index, planned)| {
            let step = &planned.step;
            let finish = if step.phase == StepPhase::Finish {
                " [finish]"
            } else {
                ""
            };
            let mut meta = Vec::new();
            if let Some(reason) = &planned.reason {
                meta.push(Meta::Note(format!("skipped: {}", reason.replace('`', ""))));
            }
            if !step.needs.is_empty() {
                let needs: Vec<String> = step.needs.iter().map(ToString::to_string).collect();
                meta.push(Meta::Note(format!("needs {}", needs.join(", "))));
            }
            if let Some(note) = &planned.note {
                meta.push(Meta::Warn(note.replace('`', "")));
            }
            let mut row = Row::new(vec![
                Cell::text(format!("{}.", index + 1)),
                Cell::new(step.id.clone(), Role::Accent),
                Cell::new(format!("({}){finish}", step.kind.name()), Role::Dim),
                Cell::text(step.title.clone()),
            ])
            .meta(meta)
            .dim(!planned.run);
            if !planned.run {
                row = row.glyph(Glyph::Skipped, Role::Dim);
            }
            for dependency in plan
                .dependencies
                .iter()
                .filter(|dependency| dependency.step.as_deref() == Some(step.id.as_str()))
            {
                let (text, meta) = requirement(dependency);
                row = row.child(format!(
                    "{text}{}",
                    crate::output::language::join_meta(&meta)
                ));
            }
            row
        })
        .collect();
    List::new(rows).meta_column().children_under(1).render()
}

/// One requirement: `requires <key> (<scope>[, hidden]) for <requirement>`,
/// `[installed]` when an installed version is used as is; one with no such
/// word is installed first, and its own plan follows under its own rule.
fn requirement(dependency: &PlannedDependency) -> (String, Vec<Meta>) {
    let exposure = if dependency.hidden { ", hidden" } else { "" };
    let text = format!(
        "requires {} ({}{exposure}) for {}",
        dependency.key,
        dependency.scope.as_str(),
        dependency.package
    );
    let meta = if dependency.installed {
        vec![Meta::State("installed".into())]
    } else {
        vec![]
    };
    (text, meta)
}

#[cfg(test)]
#[path = "plan_text/tests.rs"]
mod tests;
