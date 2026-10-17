//! Scoped M13 recipe for the old fixed Redbook consumer, not a resolver.
use super::*;
pub const LEGACY: &str = "redbook-consumer-legacy-r1";
pub const SELECTED: &str = "redbook-consumer-selected-r1";
pub const RECIPE: &str = "redbook-continuity-r1";
const REDBOOK: &str = "org.vibevm.world/redbook";
pub(super) fn known(value: &str) -> bool {
    matches!(value, LEGACY | SELECTED | RECIPE)
}

pub(super) fn plan(
    text: &str,
    selection: MigrationSelection<'_>,
) -> Checked<ManifestMigrationOutcome> {
    if !selection.source_decisions.is_empty() {
        return Err(shape("selection"));
    }
    let selected = [
        selection.source_profile,
        selection.target_profile,
        selection.recipe,
    ];
    let expected = [LEGACY, SELECTED, RECIPE];
    let fields = ["source_profile", "target_profile", "recipe"];
    if selected
        .iter()
        .zip(expected)
        .any(|(actual, expected)| actual.is_some_and(|v| v != expected))
    {
        return Err(refusal(
            MigrationRefusalKind::IncompatibleEdge,
            "selection",
            "no registered edge matches the explicit choices",
        ));
    }
    let questions = selected
        .iter()
        .enumerate()
        .filter(|(_, value)| value.is_none())
        .map(|(index, _)| MigrationInputRequirement {
            field: fields[index],
            choices: vec![expected[index].into()],
            authored_reference: None,
            resource_path: None,
        })
        .collect::<Vec<_>>();
    if !questions.is_empty() {
        return Ok(ManifestMigrationOutcome::NeedsInput(questions));
    }
    let before = Manifest::parse_str(text).map_err(|_| shape("document"))?;
    if !before.consumer_configuration.is_empty()
        || before.package.as_ref().is_some_and(|p| p.epoch.is_some())
    {
        return Err(shape("consumer_configuration"));
    }
    let consumer = before
        .consumer_node()
        .ok_or_else(|| shape("consumer"))?
        .coordinate();
    let document = Document::parse(text).map_err(|_| shape("document"))?;
    let packages = table(
        table(document.as_table().get("requires"), "requires")?.get("packages"),
        "requires.packages",
    )?;
    let mut entries = packages
        .iter()
        .filter(|(key, _)| key.strip_prefix("flow:").unwrap_or(key) == REDBOOK);
    let (_, item) = entries
        .next()
        .ok_or_else(|| shape("requires.packages.redbook"))?;
    if entries.next().is_some()
        || !before
            .requires
            .packages
            .iter()
            .any(|p| p.qualified_name() == REDBOOK)
    {
        return Err(shape("requires.packages.redbook"));
    }
    let edge = table(Some(item), "requires.packages.redbook")?;
    if !matches!(
        edge.get("version").and_then(Item::as_str),
        Some("^1.1.0" | "=1.1.0" | "1.1.0")
    ) {
        return Err(shape("requires.packages.redbook.version"));
    }
    let excluded = before
        .requires
        .excludes
        .get(REDBOOK)
        .cloned()
        .unwrap_or_default();
    let excluded_name = |name: &str| {
        excluded
            .iter()
            .any(|v| v == &format!("org.vibevm.world/{name}"))
    };
    let (mup, wal) = (excluded_name("multi-user-planning"), excluded_name("wal"));
    if mup == wal {
        return Ok(ManifestMigrationOutcome::NeedsInput(vec![
            MigrationInputRequirement {
                field: "continuity",
                choices: vec!["mup".into(), "wal".into()],
                authored_reference: Some(REDBOOK.into()),
                resource_path: None,
            },
        ]));
    }
    if wal
        && !before
            .requires
            .packages
            .iter()
            .any(|p| p.qualified_name() == "org.vibevm.world/multi-user-planning")
    {
        return Ok(ManifestMigrationOutcome::NeedsInput(vec![
            MigrationInputRequirement {
                field: "continuity",
                choices: vec![
                    "supply original independent MUP declaration or select intent explicitly"
                        .into(),
                ],
                authored_reference: Some(REDBOOK.into()),
                resource_path: None,
            },
        ]));
    }
    // A WAL branch cannot preserve a formerly active MUP-only addition.
    if mup && !excluded_name("discovery-prompt") || wal && !excluded_name("wal-specspaces") {
        return Ok(ManifestMigrationOutcome::NeedsInput(vec![
            MigrationInputRequirement {
                field: "branch-additions",
                choices: vec![
                    "explicitly exclude the inactive branch addition or retain it independently"
                        .into(),
                ],
                authored_reference: Some(REDBOOK.into()),
                resource_path: None,
            },
        ]));
    }
    let mut selections = std::collections::BTreeMap::new();
    use crate::manifest::{
        ConsumerConfiguration, ConsumerSelection, ConsumerSelectionIntent as Intent,
    };
    let mut add = |group: &str, intent, members: Vec<String>| {
        selections.insert(
            group.into(),
            ConsumerSelection {
                intent,
                members: Some(members),
            },
        );
    };
    add(
        "continuity",
        Intent::Exact,
        vec![if wal { "mup" } else { "wal" }.into()],
    );
    add(
        "additions",
        Intent::Additive,
        [
            "health-audit",
            "manual-tests",
            "comparative-research",
            "dev-runtime-docs",
        ]
        .into_iter()
        .filter(|n| !excluded_name(n))
        .map(str::to_owned)
        .collect(),
    );
    add(
        "mup-additions",
        Intent::Additive,
        if !wal || excluded_name("discovery-prompt") {
            vec![]
        } else {
            vec!["discovery-prompt".into()]
        },
    );
    add(
        "wal-additions",
        Intent::Additive,
        if wal || excluded_name("wal-specspaces") {
            vec![]
        } else {
            vec!["wal-specspaces".into()]
        },
    );
    let row = ConsumerConfiguration {
        dependency: REDBOOK.into(),
        domain: "workspace".into(),
        definition: consumer.clone(),
        definition_owner: "vibevm".into(),
        target: consumer,
        target_owner: "vibevm".into(),
        instance: REDBOOK.into(),
        configuration: "authored".into(),
        selections,
    };
    // This recipe owns only the finite former optional roster. Unknown
    // exclusions require a fresh reviewed recipe, never silent deletion.
    if excluded.iter().any(|v| {
        ![
            "multi-user-planning",
            "wal",
            "wal-specspaces",
            "discovery-prompt",
            "health-audit",
            "manual-tests",
            "comparative-research",
            "dev-runtime-docs",
        ]
        .iter()
        .any(|n| *v == format!("org.vibevm.world/{n}"))
    }) {
        return Err(shape("exclude"));
    }
    let array = edge
        .get("exclude")
        .and_then(Item::as_array)
        .ok_or_else(|| shape("exclude"))?;
    let mut edits = Vec::new();
    // Preserve whitespace and all comments inside the array as well. The
    // explicit selections replace these exclusion tokens, including addons.
    let span = array.span().ok_or_else(unprovable)?;
    let mut token = text[span.clone()].to_owned();
    for value in array.iter().collect::<Vec<_>>().into_iter().rev() {
        let value_span = value.span().ok_or_else(unprovable)?;
        // Reverse source order preserves the remaining original spans.
        token.replace_range(
            value_span.start - span.start..value_span.end - span.start,
            "",
        );
    }
    let mut comment = false;
    token = token
        .chars()
        .filter(|c| {
            if *c == '\n' {
                comment = false;
            }
            if *c == '#' {
                comment = true;
            }
            comment || *c != ','
        })
        .collect();
    token = token
        .split_inclusive('\n')
        .map(|line| {
            if line.trim().is_empty() {
                if line.ends_with("\r\n") {
                    "\r\n"
                } else if line.ends_with('\n') {
                    "\n"
                } else {
                    ""
                }
            } else {
                line
            }
        })
        .collect();
    edits.push(MigrationTokenEdit {
        field: "requires.packages.redbook.exclude",
        before_token: text[span.clone()].into(),
        before_span: span,
        after_token: token,
    });
    #[derive(serde::Serialize)]
    struct Rows {
        consumer_configuration: Vec<ConsumerConfiguration>,
    }
    let appended = toml::to_string(&Rows {
        consumer_configuration: vec![row],
    })
    .map_err(|_| unprovable())?;
    edits.push(MigrationTokenEdit { field: "consumer_configuration", before_span: text.len()..text.len(), before_token: String::new(), after_token: format!("\n# Explicit old Redbook intent; independent roots remain independently required.\n{appended}") });
    let after = replay_edits(text, &edits)?;
    let checked_manifest = Manifest::parse_str(&after).map_err(|_| shape("output"))?;
    let result = ManifestMigrationPlan {
        source_profile: LEGACY,
        target_profile: SELECTED,
        recipe: RECIPE,
        before: text.into(),
        before_identity: identity(b"vibe-manifest-content-r1", &[text.as_bytes()]),
        after_identity: identity(b"vibe-manifest-content-r1", &[after.as_bytes()]),
        diff_identity: diff_identity(LEGACY, SELECTED, RECIPE, text, &after, &edits),
        after,
        edits,
        checked_manifest,
        checked_features: None,
    };
    Ok(ManifestMigrationOutcome::Planned(result))
}
