//! Measured human rows for `vibe registry list`.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#registry");

use super::{ListReportOverride, ListReportRegistry};
use crate::output::language::{
    Block, Cell, Glyph, List, Meta, Role, Row, cut, hint, paint, rule_at,
};

/// Draw declared registries, their policy marks and mirrors, then overrides.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(super) fn render(
    registries: &[ListReportRegistry],
    effective_primary: Option<&str>,
    overrides: &[ListReportOverride],
    width: usize,
    rule: usize,
) -> String {
    let rows = registries
        .iter()
        .enumerate()
        .map(|(index, registry)| {
            let mut meta = vec![
                Meta::note(format!("naming {}", registry.naming)),
                Meta::note(format!("ref {}", registry.refname)),
            ];
            if registry.adapter.is_none() {
                meta.push(Meta::note("publish unsupported"));
            }
            if let Some(scope) = &registry.scope {
                meta.push(Meta::State(scope.clone()));
            }
            if effective_primary == Some(registry.name.as_str()) {
                meta.push(Meta::State("primary".into()));
            }
            let mut row = Row::new(vec![
                Cell::text((index + 1).to_string()),
                Cell::text(&registry.name),
                Cell::text(&registry.registry_type),
                Cell::text(&registry.url),
            ])
            .meta(meta);
            if registry.disabled_by_source_policy {
                row = row.child(format!(
                    "{}{}",
                    vibe_registry::multi_registry_resolver::DISABLED_BY_POLICY_MARK,
                    registry
                        .disabled_by
                        .as_deref()
                        .map(|selector| format!(" [{selector}]"))
                        .unwrap_or_default(),
                ));
            }
            for mirror in &registry.mirrors {
                row = row.child(format!(
                    "mirror {} · priority {}{}",
                    mirror.url,
                    mirror.priority,
                    if mirror.of == "*" { " · any registry" } else { "" },
                ));
            }
            row
        })
        .collect();
    let mut rendered = format!(
        "{}\n{}",
        rule_at("registries · declared order", None, rule),
        List::new(rows).meta_column().children_under(1).render_at(width),
    );
    if registries.iter().any(|row| row.disabled_by_source_policy) {
        rendered.push_str(&hint_line(
            &format!(
                "effective primary: {}",
                effective_primary
                    .unwrap_or("(none — the source policy disables every configured registry)")
            ),
            width,
        ));
    }
    rendered.push_str(&hint_line(&counts(registries, overrides), width));
    if !overrides.is_empty() {
        let rows = overrides
            .iter()
            .map(|entry| {
                let source = format!(
                    "{}{}",
                    entry.source_url,
                    entry.refname.as_deref().map(|r| format!("@{r}")).unwrap_or_default(),
                );
                let mut row = Row::new(vec![
                    Cell::text(&entry.pkgref),
                    Cell::text("→"),
                    Cell::text(source),
                ]);
                if let Some(reason) = &entry.reason {
                    row = row.meta(vec![Meta::note(reason)]);
                }
                row
            })
            .collect();
        rendered.push_str(&format!(
            "\n\n{}\n{}",
            rule_at("overrides", None, rule),
            List::new(rows).render_at(width),
        ));
    }
    rendered
}

/// Name an empty declaration layer and the command that adds a registry.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(super) fn render_empty(label: &str, width: usize) -> String {
    format!(
        "{}\n  {}",
        Block::new(Glyph::Skipped, format!("no registries in {label}")).render_at(width),
        paint(Role::Accent, "vibe registry add <name> <url>"),
    )
}

/// Summarize the declared rows, including policy-disabled rows and mirrors.
pub(super) fn counts(
    registries: &[ListReportRegistry],
    overrides: &[ListReportOverride],
) -> String {
    let n = registries.len();
    let m = registries.iter().map(|row| row.mirrors.len()).sum::<usize>();
    let k = overrides.len();
    let d = registries.iter().filter(|row| row.disabled_by_source_policy).count();
    let mut counts = format!(
        "{n} {} · {m} {} · {k} {}",
        if n == 1 { "registry" } else { "registries" },
        if m == 1 { "mirror" } else { "mirrors" },
        if k == 1 { "override" } else { "overrides" },
    );
    if d > 0 {
        counts.push_str(&format!(" · {d} disabled by the source policy"));
    }
    counts
}

fn hint_line(text: &str, width: usize) -> String {
    format!("\n{}", cut(&format!("  {}", hint(text)), width))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::registry::config::ListReportMirror;

    #[test]
    #[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
    fn registry_list_renders_measured_rows() {
        let registries = vec![
            ListReportRegistry {
                name: "vibespecs".into(),
                registry_type: "github".into(),
                scope: Some("user".into()),
                url: "https://github.com/vibespecs".into(),
                refname: "main".into(),
                naming: "fqdn".into(),
                host: "github.com".into(),
                org: "vibespecs".into(),
                adapter: Some("github".into()),
                mirrors: vec![],
                disabled_by_source_policy: true,
                disabled_by: Some("github".into()),
            },
            ListReportRegistry {
                name: "corp".into(),
                registry_type: "localdir".into(),
                scope: Some("user".into()),
                url: "file:///srv/reg".into(),
                refname: "main".into(),
                naming: "fqdn".into(),
                host: "".into(),
                org: "".into(),
                adapter: Some("localdir".into()),
                mirrors: vec![ListReportMirror {
                    of: "corp".into(),
                    url: "file:///mirror/corp".into(),
                    priority: 5,
                }],
                disabled_by_source_policy: false,
                disabled_by: None,
            },
            ListReportRegistry {
                name: "team".into(),
                registry_type: "monorepo".into(),
                scope: Some("site".into()),
                url: "file:///srv/team.git".into(),
                refname: "main".into(),
                naming: "name".into(),
                host: "".into(),
                org: "".into(),
                adapter: Some("monorepo".into()),
                mirrors: vec![],
                disabled_by_source_policy: false,
                disabled_by: None,
            },
        ];
        let overrides = vec![ListReportOverride {
            pkgref: "org.x/y".into(),
            source_url: "file:///src/y".into(),
            refname: Some("dev".into()),
            reason: Some("local fix".into()),
        }];
        assert_eq!(
            render(&registries, Some("corp"), &overrides, 100, 100),
            concat!(
                "── registries · declared order ─────────────────────────────────────────────────────────────────────\n",
                "  1  vibespecs  github    https://github.com/vibespecs  naming fqdn · ref main  [user]\n",
                "     └ disabled by site policy (exclude_hosting) — not used [github]\n",
                "  2  corp       localdir  file:///srv/reg               naming fqdn · ref main  [user]  [primary]\n",
                "     └ mirror file:///mirror/corp · priority 5\n",
                "  3  team       monorepo  file:///srv/team.git          naming name · ref main  [site]\n",
                "  effective primary: corp\n",
                "  3 registries · 1 mirror · 1 override · 1 disabled by the source policy\n",
                "\n",
                "── overrides ───────────────────────────────────────────────────────────────────────────────────────\n",
                "  org.x/y  →  file:///src/y@dev · local fix",
            )
        );
        assert_eq!(
            render_empty("the site profile · none applied", 100),
            "○ no registries in the site profile · none applied\n  vibe registry add <name> <url>"
        );
    }
}
