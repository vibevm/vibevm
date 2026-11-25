//! Measured effective-installation sections of the site profile report.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile");

use vibe_wire::generated::self_apply_report::{Effective, RegistryScope, RegistryType};

use crate::commands::vvm::bundle::release_source::kind_label;
use crate::commands::vvm::site::layers::InstallBaseOrigin;
use crate::output::language::{Cell, List, Meta, Role, Row, cut, hint, rule_at};

/// The effective settings and declared registries, with origins and policy marks.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
pub(in crate::commands::vvm::site) fn sections(
    effective: &Effective,
    width: usize,
    rule: usize,
) -> String {
    let mut rows = Vec::new();
    let row = |key: &str, value: String, notes: Vec<Meta>| {
        Row::new(vec![Cell::text(key), Cell::text(value)]).meta(notes)
    };
    if let Some(root) = &effective.install_root {
        rows.push(row(
            "install root",
            root.path.clone(),
            vec![Meta::note(&root.from)],
        ));
        rows.push(row("shims", root.shim_dir.clone(), vec![]));
        rows.push(row("data", root.data_dir.clone(), vec![]));
        let (apps, note) = if root.from == InstallBaseOrigin::BuiltIn.describe() {
            (
                "under the settings root".to_owned(),
                "~/.vibe, or $VIBE_SETTINGS".to_owned(),
            )
        } else {
            (
                std::path::Path::new(&root.path)
                    .join("opt")
                    .join("apps")
                    .display()
                    .to_string(),
                format!("commands in {}", root.shim_dir),
            )
        };
        rows.push(row("applications", apps, vec![Meta::note(note)]));
    }
    match (&effective.release, &effective.release_error) {
        (Some(release), _) => rows.push(row(
            "release source",
            format!("{} {}", kind_label(release.source.clone()), release.location),
            vec![Meta::note(&release.from)],
        )),
        (None, Some(error)) => rows.push(row(
            "release source",
            "unusable".to_owned(),
            vec![Meta::Alert(error.clone())],
        )),
        (None, None) => {}
    }
    if !effective.exclude_hosting.is_empty() {
        rows.push(row(
            "never contacted",
            effective.exclude_hosting.join(", "),
            vec![],
        ));
    }
    if let Some(prefer) = &effective.prefer_hosting {
        rows.push(row("preferred hosting", prefer.join(", "), vec![]));
    }
    let proxy = &effective.proxy;
    rows.push(row(
        "proxy mode",
        proxy.mode.as_deref().unwrap_or("none").to_owned(),
        vec![Meta::note(&proxy.mode_origin)],
    ));
    for (key, value) in [
        ("proxy https", &proxy.https),
        ("proxy http", &proxy.http),
        ("proxy no_proxy", &proxy.no_proxy),
    ] {
        if let Some(value) = value {
            rows.push(row(key, value.clone(), vec![]));
        }
    }
    if let Some(pac) = &proxy.pac {
        let mut notes = vec![
            Meta::note(proxy.pac_kind.as_deref().unwrap_or("?")),
            Meta::note(&proxy.pac_origin),
        ];
        if let Some(resolved) = &proxy.pac_resolved {
            notes.push(Meta::note(resolved));
        }
        rows.push(row("proxy pac", pac.clone(), notes));
    }
    let port = proxy.px_port.map(|port| port.to_string());
    for (key, value) in [
        ("proxy upstream", &proxy.upstream),
        ("proxy px_port", &port),
        ("proxy px_path", &proxy.px_path),
        ("proxy ca_file", &proxy.ca_file),
    ] {
        if let Some(value) = value {
            rows.push(row(key, value.clone(), vec![]));
        }
    }
    if proxy.revocation != "check" {
        rows.push(row(
            "proxy revocation",
            proxy.revocation.clone(),
            vec![Meta::note(&proxy.revocation_origin)],
        ));
    }
    if !proxy.extra_ca.is_empty() {
        rows.push(row(
            "extra CA",
            proxy.extra_ca.join(", "),
            vec![Meta::note("added to the system's own")],
        ));
    }
    if effective.offline {
        rows.push(row(
            "offline",
            "yes".to_owned(),
            vec![Meta::note("[net] offline")],
        ));
    }
    if effective.site_remap_rows > 0 {
        rows.push(row(
            "site remap rows",
            effective.site_remap_rows.to_string(),
            vec![Meta::note("vibe remap list shows them")],
        ));
    }
    rows.push(row(
        "version selection",
        effective.versions_select.select.clone(),
        vec![Meta::note(&effective.versions_select.from)],
    ));
    let settings = List::new(rows).render_at(width);
    let mut registries = Vec::new();
    for registry in &effective.registries {
        let mut row = Row::new(vec![
            Cell::text(&registry.name),
            Cell::text(registry_type_label(&registry.type_)),
            Cell::text(&registry.url),
        ])
        .meta(vec![Meta::State(scope_label(&registry.scope).into())]);
        if let Some(selector) = &registry.disabled_by {
            row = row.child(format!(
                "{} [{selector}]",
                vibe_registry::multi_registry_resolver::DISABLED_BY_POLICY_MARK,
            ));
        }
        registries.push(row);
    }
    if registries.is_empty() {
        registries.push(Row::new(vec![Cell::new("none", Role::Dim)]));
    }
    let mut text = format!(
        "{}\n{settings}\n\n{}\n{}",
        rule_at("effective settings", None, rule),
        rule_at("registries, highest first", None, rule),
        List::new(registries).meta_column().render_at(width),
    );
    if effective
        .registries
        .iter()
        .any(|row| row.disabled_by.is_some())
    {
        let primary = effective
            .registries
            .iter()
            .find(|row| row.disabled_by.is_none())
            .map_or(
                "(none — the source policy disables every configured registry)",
                |row| row.name.as_str(),
            );
        text.push('\n');
        text.push_str(&cut(
            &format!("  {}", hint(&format!("effective primary: {primary}"))),
            width,
        ));
    }
    text
}

/// The wire value of one registry scope, spelled for a human.
fn scope_label(scope: &RegistryScope) -> &'static str {
    match scope {
        RegistryScope::User => "user",
        RegistryScope::Site => "site",
        RegistryScope::Default => "default",
    }
}

/// The wire value of one registry type, as written in `[[registry]].type`.
fn registry_type_label(kind: &RegistryType) -> &'static str {
    match kind {
        RegistryType::Github => vibe_core::manifest::RegistryType::Github.as_str(),
        RegistryType::Gitverse => vibe_core::manifest::RegistryType::Gitverse.as_str(),
        RegistryType::Git => vibe_core::manifest::RegistryType::Git.as_str(),
        RegistryType::Monorepo => vibe_core::manifest::RegistryType::Monorepo.as_str(),
        RegistryType::Localdir => vibe_core::manifest::RegistryType::Localdir.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibe_wire::generated::self_apply_report::{
        InstallRootRow, ProxyRow, RegistryRow, ReleaseRow, ReleaseSourceKind, VersionsSelectRow,
    };

    const GOLDEN: &str = "\
── effective settings ──────────────────────────────────────────────────────────────────────────────
  install root       /srv/iroot · $VIBEVM_INSTALL_ROOT
  shims              /srv/iroot/opt/bin
  data               /srv/iroot/opt/vibevm
  applications       /srv/iroot/opt/apps · commands in /srv/iroot/opt/bin
  release source     localdir /srv/site/releases · site profile
  never contacted    github
  proxy mode         px · site profile
  proxy no_proxy     localhost
  proxy upstream     proxy.corp:8080
  extra CA           /srv/ca.pem · added to the system's own
  version selection  newest · default

── registries, highest first ───────────────────────────────────────────────────────────────────────
  vibespecs  github    https://github.com/vibespecs  [default]
  └ disabled by site policy (exclude_hosting) — not used [github]
  corp       localdir  file:///srv/registry          [site]
  effective primary: corp";

    #[test]
    fn self_apply_effective_settings_render_as_measured_sections() {
        let fixture = Effective {
            install_root: Some(InstallRootRow {
                path: "/srv/iroot".into(),
                from: "$VIBEVM_INSTALL_ROOT".into(),
                shim_dir: "/srv/iroot/opt/bin".into(),
                data_dir: "/srv/iroot/opt/vibevm".into(),
            }),
            registries: vec![
                RegistryRow {
                    name: "vibespecs".into(),
                    type_: RegistryType::Github,
                    url: "https://github.com/vibespecs".into(),
                    scope: RegistryScope::Default,
                    disabled_by: Some("github".into()),
                },
                RegistryRow {
                    name: "corp".into(),
                    type_: RegistryType::Localdir,
                    url: "file:///srv/registry".into(),
                    scope: RegistryScope::Site,
                    disabled_by: None,
                },
            ],
            release: Some(ReleaseRow {
                source: ReleaseSourceKind::Localdir,
                location: "/srv/site/releases".into(),
                from: "site profile".into(),
            }),
            release_error: None,
            packages_drop: None,
            prefer_hosting: None,
            exclude_hosting: vec!["github".into()],
            proxy: ProxyRow {
                mode: Some("px".into()),
                mode_origin: "site profile".into(),
                https: None,
                http: None,
                no_proxy: Some("localhost".into()),
                pac: None,
                pac_kind: None,
                pac_origin: "default".into(),
                pac_resolved: None,
                upstream: Some("proxy.corp:8080".into()),
                px_port: None,
                px_path: None,
                ca_file: None,
                extra_ca: vec!["/srv/ca.pem".into()],
                revocation: "check".into(),
                revocation_origin: "default".into(),
            },
            offline: false,
            site_remap_rows: 0,
            versions_select: VersionsSelectRow {
                select: "newest".into(),
                from: "default".into(),
            },
        };
        assert_eq!(sections(&fixture, 100, 100), GOLDEN);
    }
}
