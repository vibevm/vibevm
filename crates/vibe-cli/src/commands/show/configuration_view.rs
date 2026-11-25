//! The human views of the configuration commands of vibe show: retained data and previews.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#SURFACE-PARITY");

use crate::output::language::{Block, Cell, Glyph, List, Meta, Role, Row, cut, hint, rule_at};
use vibe_wire::generated::ordinary_configuration_preview_result::OrdinaryConfigurationPreviewResult;
use vibe_wire::generated::ordinary_integer_preview_result::OrdinaryIntegerPreviewResult;

/// Retained configuration values already rendered by their typed Display implementation.
pub(super) struct DataRow {
    pub(super) package: String,
    pub(super) version: String,
    pub(super) values: Vec<(String, String)>,
    pub(super) parameters: Vec<(String, String)>,
}

/// Draw retained values as children of their package, not as a wire document.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(super) fn data_view(rows: &[DataRow], width: usize, rule: usize) -> String {
    if rows.is_empty() {
        return Block::new(Glyph::Skipped, "no retained configuration data in this project")
            .render_at(width);
    }
    let rows = rows
        .iter()
        .map(|data| {
            let w = data
                .values
                .iter()
                .chain(&data.parameters)
                .map(|(key, _)| key.chars().count())
                .max()
                .unwrap_or(0);
            let mut row = Row::new(vec![Cell::new(
                format!("{}@{}", data.package, data.version),
                Role::Accent,
            )]);
            for (key, value) in &data.values {
                row = row.child(format!("{key:<w$}  {value}"));
            }
            for (key, value) in &data.parameters {
                row = row.child(format!("{key:<w$}  {value}  [parameter]"));
            }
            row
        })
        .collect();
    format!(
        "{}\n{}",
        rule_at("configuration data", None, rule),
        List::new(rows).render_at(width)
    )
}

/// Draw a boolean preview description and its optional resolved selection.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(super) fn preview_view(
    result: &OrdinaryConfigurationPreviewResult,
    width: usize,
    rule: usize,
) -> String {
    let mut rows = Vec::new();
    for choice in &result.description.choices {
        rows.push(
            Row::new(vec![
                Cell::text("choice"),
                Cell::text(&choice.id),
                Cell::text(
                    choice
                        .members
                        .iter()
                        .map(|member| member.id.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                ),
            ])
            .meta(vec![Meta::note(&choice.name)]),
        );
    }
    for parameter in &result.description.parameters {
        let mut meta = vec![Meta::note(&parameter.name)];
        if parameter.required {
            meta.push(Meta::State("required".into()));
        }
        rows.push(
            Row::new(vec![
                Cell::text("parameter"),
                Cell::text(&parameter.id),
                Cell::text("bool"),
            ])
            .meta(meta),
        );
    }
    let mut text = format!(
        "{}\n{}\n  {}",
        preview_head(&result.package, &result.version, &result.source.registry, rule),
        List::new(rows).render_at(width),
        hint(&format!("content {}", result.source.content_hash)),
    );
    if let Some(resolved) = &result.resolved {
        let rows = resolved
            .features
            .iter()
            .map(|feature| Row::new(vec![Cell::text("feature"), Cell::text(feature)]))
            .chain(resolved.parameters.iter().map(|(id, value)| {
                Row::new(vec![
                    Cell::text("parameter"),
                    Cell::text(id),
                    Cell::text(value.to_string()),
                ])
            }))
            .collect();
        text.push_str(&format!(
            "\n\n{}\n{}",
            rule_at("resolved", None, rule),
            selected_rows(rows, width)
        ));
    } else if !result.questions.is_empty() {
        text.push('\n');
        text.push_str(&cut(
            &format!(
                "  {}",
                hint(&format!(
                    "answers needed: {}; pass --input <file>, or --interactive in a terminal",
                    result.questions.len(),
                ))
            ),
            width,
        ));
    }
    text
}

/// Draw an integer preview description and its optional effective values.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(super) fn integer_view(
    result: &OrdinaryIntegerPreviewResult,
    width: usize,
    rule: usize,
) -> String {
    let definition = &result.definition;
    let rows = vec![
        Row::new(vec![
            Cell::text("parameter"),
            Cell::text(&definition.parameter),
            Cell::text(format!("{}..{}", definition.min, definition.max)),
        ])
        .meta(vec![
            Meta::note(&definition.name),
            Meta::note(format!("preset {}", definition.preset)),
        ]),
        Row::new(vec![Cell::text("profile"), Cell::text(&definition.profile)])
            .meta(vec![Meta::note(&definition.profile_name)]),
        Row::new(vec![
            Cell::text("group"),
            Cell::text(&definition.group),
            Cell::text(definition.members.join(", ")),
        ])
        .meta(vec![Meta::note(&definition.group_name)]),
    ];
    let mut text = format!(
        "{}\n{}\n  {}",
        preview_head(&result.package, &result.version, &result.source.registry, rule),
        List::new(rows).render_at(width),
        hint(&format!("content {}", result.source.content_hash)),
    );
    if let Some(effective) = &result.effective {
        let rows = effective
            .iter()
            .map(|(key, value)| {
                Row::new(vec![Cell::text(key), Cell::text(value.to_string())])
            })
            .collect();
        text.push_str(&format!(
            "\n\n{}\n{}",
            rule_at("effective", None, rule),
            selected_rows(rows, width)
        ));
    }
    text
}

/// Draw the common preview rule with the source registry on its right.
fn preview_head(package: &str, version: &str, registry: &str, rule: usize) -> String {
    rule_at(
        &format!("configuration preview of {package}@{version}"),
        Some(&format!("registry {registry}")),
        rule,
    )
}

/// Make an empty resolved list explicit rather than leaving a bare rule.
fn selected_rows(mut rows: Vec<Row>, width: usize) -> String {
    if rows.is_empty() {
        rows.push(Row::new(vec![Cell::new("none", Role::Dim)]));
    }
    List::new(rows).render_at(width)
}

#[cfg(test)]
#[path = "configuration_view/tests.rs"]
mod tests;
