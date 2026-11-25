//! Measured lists for the effective configuration and its declaration layers.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#registry");

use super::ConfigReport;
use crate::output::language::{Cell, List, Meta, Row, rule_at};

/// Draw the configuration report as lists under section rules.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(super) fn render(
    report: &ConfigReport,
    width: usize,
    rule: usize,
    effective: &[Row],
    verbose: bool,
) -> String {
    render_with_provenance(report, width, rule, effective, verbose, |origin| {
        super::layer_view::provenance(report, origin)
    })
}

fn render_with_provenance(
    report: &ConfigReport,
    width: usize,
    rule: usize,
    effective: &[Row],
    verbose: bool,
    provenance: impl Fn(&str) -> String,
) -> String {
    let mut sections = Vec::new();
    let mut section = |label: &str, list: List| {
        if !list.rows.is_empty() {
            sections.push(format!(
                "{}\n{}",
                rule_at(label, None, rule),
                list.render_at(width)
            ));
        }
    };
    section(
        "project",
        List::new(vec![
            Row::new(vec![
                Cell::text(&report.project_name),
                Cell::text(&report.project_version),
            ])
            .meta(vec![Meta::note(&report.project)]),
        ]),
    );
    let mut registries: Vec<_> = report
        .registries
        .iter()
        .enumerate()
        .map(|(index, registry)| {
            let mut meta = vec![
                Meta::note(format!("ref {}", registry.refname)),
                Meta::note(format!("naming {}", registry.naming)),
                Meta::State(provenance(registry.provenance)),
            ];
            if index == 0 {
                meta.push(Meta::State("primary".into()));
            }
            Row::new(vec![
                Cell::text((index + 1).to_string()),
                Cell::text(&registry.name),
                Cell::text(&registry.url),
            ])
            .meta(meta)
        })
        .collect();
    if registries.is_empty() {
        registries.push(Row::new(vec![Cell::text("none")]).dim(true));
    }
    section(
        "registries · primary first",
        List::new(registries).meta_column(),
    );
    section(
        "mirrors",
        List::new(
            report
                .mirrors
                .iter()
                .map(|mirror| {
                    Row::new(vec![Cell::text(&mirror.of), Cell::text(&mirror.url)]).meta(vec![
                        Meta::note(format!("priority {}", mirror.priority)),
                        Meta::State(provenance(mirror.provenance)),
                    ])
                })
                .collect(),
        )
        .meta_column(),
    );
    section(
        "overrides",
        List::new(
            report
                .overrides
                .iter()
                .map(|entry| {
                    let source = match &entry.refname {
                        Some(reference) => format!("{}@{reference}", entry.source_url),
                        None => entry.source_url.clone(),
                    };
                    let mut meta = Vec::new();
                    if let Some(reason) = &entry.reason {
                        meta.push(Meta::note(reason));
                    }
                    meta.push(Meta::State(provenance(entry.provenance)));
                    Row::new(vec![
                        Cell::text(&entry.pkgref),
                        Cell::text("→"),
                        Cell::text(source),
                    ])
                    .meta(meta)
                })
                .collect(),
        )
        .meta_column(),
    );

    let user = &report.user_config;
    let user_row = match &user.path {
        Some(path) => {
            let mut meta = Vec::new();
            let state = if user.loaded {
                "loaded"
            } else if let Some(error) = &user.error {
                meta.push(Meta::Alert(error.clone()));
                "parse error"
            } else {
                "not present"
            };
            meta.push(Meta::State(state.into()));
            Row::new(vec![Cell::text("user config"), Cell::text(path)]).meta(meta)
        }
        None => Row::new(vec![
            Cell::text("user config"),
            Cell::text("no path resolved"),
        ])
        .meta(vec![Meta::note(
            "set HOME, VIBE_SETTINGS or VIBEVM_USER_CONFIG",
        )]),
    };
    let site = &report.site_profile;
    let site_row = match &site.path {
        Some(path) => {
            let mut meta = Vec::new();
            let state = if site.loaded {
                if let Some(name) = &site.name {
                    meta.push(Meta::note(name));
                }
                "applied"
            } else if let Some(error) = &site.error {
                meta.push(Meta::Alert(error.clone()));
                "unreadable"
            } else {
                "none applied"
            };
            meta.push(Meta::State(state.into()));
            Row::new(vec![Cell::text("site profile"), Cell::text(path)]).meta(meta)
        }
        None => Row::new(vec![
            Cell::text("site profile"),
            Cell::text("no settings directory resolved"),
        ]),
    };
    section(
        "layers",
        List::new(vec![
            user_row,
            site_row,
            Row::new(vec![
                Cell::text("version selection"),
                Cell::text(report.versions.select),
            ])
            .meta(vec![Meta::State(report.versions.provenance.into())]),
        ])
        .meta_column(),
    );
    section(
        "effective choices",
        List::new(effective.to_vec()).meta_column(),
    );
    section(
        "environment",
        List::new(
            report
                .env
                .iter()
                .map(|entry| {
                    let mut row = Row::new(vec![
                        Cell::text(entry.name),
                        Cell::text(entry.value.as_deref().unwrap_or("unset")),
                    ])
                    .meta(vec![Meta::State(entry.provenance.into())]);
                    if verbose {
                        for line in crate::output::language::wrap(
                            entry.description,
                            width.saturating_sub(4),
                        ) {
                            row = row.child(line);
                        }
                    }
                    row
                })
                .collect(),
        )
        .meta_column()
        .children_under(0),
    );
    section(
        "invoked by",
        List::new(vec![
            Row::new(vec![Cell::text(
                report
                    .invoked_by_resolution
                    .value
                    .as_deref()
                    .unwrap_or("unset"),
            )])
            .meta(vec![Meta::State(
                report.invoked_by_resolution.provenance.into(),
            )]),
        ])
        .meta_column(),
    );
    sections.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::super::{
        ConfigEnvEntry, ConfigInvokedBy, ConfigMirror, ConfigRegistry, ConfigSiteProfileSummary,
        ConfigUserConfigSummary, ConfigVersions,
    };
    use super::*;

    #[test]
    fn show_config_renders_sections_as_lists() {
        let report = ConfigReport {
            ok: true,
            command: "show:config",
            project: "/srv/app".into(),
            project_name: "app".into(),
            project_version: "0.1.0".into(),
            registries: [
                ("team", "file:///team/registry", "vibe.toml"),
                ("mine", "file:///home/me/registry", "~/.vibe/registry.toml"),
                ("corp", "file:///site/corp", "site-profile.toml"),
            ]
            .into_iter()
            .map(|(name, url, provenance)| ConfigRegistry {
                name: name.into(),
                url: url.into(),
                refname: "main".into(),
                naming: "fqdn".into(),
                provenance,
            })
            .collect(),
            mirrors: vec![ConfigMirror {
                of: "corp".into(),
                url: "file:///site/corp-mirror".into(),
                priority: 5,
                provenance: "site-profile.toml",
            }],
            overrides: vec![],
            env: vec![ConfigEnvEntry {
                name: "VIBEVM_PUBLISH_TOKEN_GITVERSE",
                value: None,
                provenance: "default",
                description: "Publish token.",
            }],
            user_config: ConfigUserConfigSummary {
                path: Some("/u/.vibe/config.toml".into()),
                loaded: true,
                error: None,
            },
            site_profile: ConfigSiteProfileSummary {
                path: Some("/u/.vibe/site-profile.toml".into()),
                loaded: true,
                name: Some("corp-site".into()),
                error: None,
            },
            versions: ConfigVersions {
                select: "newest",
                provenance: "default",
                description: "…",
            },
            invoked_by_resolution: ConfigInvokedBy {
                value: None,
                provenance: "none",
                description: "…",
            },
        };
        let expected = "── project ─────────────────────────────────────────────────────────────────────────────────────────
  app  0.1.0 · /srv/app

── registries · primary first ──────────────────────────────────────────────────────────────────────
  1  team  file:///team/registry     ref main · naming fqdn  [vibe.toml]  [primary]
  2  mine  file:///home/me/registry  ref main · naming fqdn  [~/.vibe/registry.toml]
  3  corp  file:///site/corp         ref main · naming fqdn  [site-profile.toml]

── mirrors ─────────────────────────────────────────────────────────────────────────────────────────
  corp  file:///site/corp-mirror  priority 5  [site-profile.toml]

── layers ──────────────────────────────────────────────────────────────────────────────────────────
  user config        /u/.vibe/config.toml        [loaded]
  site profile       /u/.vibe/site-profile.toml  corp-site  [applied]
  version selection  newest                      [default]

── environment ─────────────────────────────────────────────────────────────────────────────────────
  VIBEVM_PUBLISH_TOKEN_GITVERSE  unset  [default]

── invoked by ──────────────────────────────────────────────────────────────────────────────────────
  unset  [none]";
        // Keep the original exact 100-column layout contract isolated from
        // machine-local settings paths. Production supplies the path adapter;
        // this fixture supplies deterministic declaration labels.
        let rendered = render_with_provenance(&report, 100, 100, &[], false, str::to_owned);
        assert_eq!(rendered, expected);
        assert!(!rendered.contains("Publish token."));
    }
}
