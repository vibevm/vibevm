//! The installation as it reads through its layers, where a person puts
//! releases and payloads, and what in the user's own files defeats the
//! site (`PROP-019#site-profile`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile");

use anyhow::Result;
use vibe_core::remap::drop::{package_lookup, release_dir_version};
use vibe_core::{GlobalRegistryConfig, RegistryLayers, RegistryScope};

use super::super::bundle::release_source::{self, ReleaseSource, SourceOrigin};
use super::ConfigLayers;
use super::report::{
    Effective, PackagesWhere, ProxyRow, RegistryRow, ReleaseRow, VersionedWhere, Where,
};

/// The installation as it reads with `layers`: registries by layer,
/// release source, drops, policy, proxy. `active` is this invocation's
/// resolved release source (`--show`); without it the source is resolved
/// from `layers` (an apply, whose layer is not in force yet in this
/// process).
pub(super) fn effective(
    layers: &ConfigLayers,
    active: Option<
        Result<release_source::EffectiveReleaseSource, release_source::ReleaseSourceError>,
    >,
    warnings: &mut Vec<String>,
) -> Result<Effective> {
    let user = match vibe_core::settings::registry_config_path() {
        Some(path) => GlobalRegistryConfig::load_from(&path)?,
        None => GlobalRegistryConfig::default(),
    };
    let user_file_exists = vibe_core::settings::registry_config_path().is_some_and(|p| p.exists());
    let site = layers
        .site
        .as_ref()
        .map(|site| site.profile.registry_layer());
    let mut registries: Vec<RegistryRow> = RegistryLayers {
        local: None,
        project: None,
        user: &user,
        site: site.as_ref(),
    }
    .origins()
    .into_iter()
    .map(|(scope, row)| RegistryRow {
        kind: row.effective_type().to_string(),
        name: row.name,
        url: row.url,
        scope: match scope {
            RegistryScope::Site => "site",
            _ => "user",
        },
    })
    .collect();
    if !user_file_exists && site.as_ref().is_none_or(|site| site.registries.is_empty()) {
        registries.extend(
            vibe_core::default_registries()
                .into_iter()
                .map(|row| RegistryRow {
                    kind: row.effective_type().to_string(),
                    name: row.name,
                    url: row.url,
                    scope: "default",
                }),
        );
    }
    let release = match active {
        Some(active) => active,
        None => layers.release(None),
    };
    let (release, release_error) = match release {
        Ok(release) => (
            Some(ReleaseRow {
                source: release.source.kind(),
                location: release.source.location(),
                from: release.origin.describe(),
            }),
            None,
        ),
        Err(error) => (None, Some(error.to_string())),
    };
    let policy = match &site {
        Some(site) => user.source_policy.over(&site.source_policy),
        None => user.source_policy.clone(),
    };
    let merged = match &layers.site {
        Some(site) => layers.own.clone().over_site(&site.profile),
        None => layers.own.clone(),
    };
    Ok(Effective {
        registries,
        release,
        release_error,
        packages_drop: layers
            .packages_drop(warnings)
            .map(|(drop, _)| drop.display().to_string()),
        prefer_hosting: policy
            .prefer_hosting
            .as_ref()
            .map(|prefer| prefer.iter().map(ToString::to_string).collect()),
        exclude_hosting: policy
            .exclude_hosting
            .iter()
            .map(ToString::to_string)
            .collect(),
        proxy: ProxyRow {
            https: merged.proxy.https,
            http: merged.proxy.http,
            no_proxy: merged.proxy.no_proxy,
        },
        offline: merged.net.offline,
        site_remap_rows: layers
            .site
            .as_ref()
            .map_or(0, |site| site.profile.remap.len()),
    })
}

/// Where releases and payloads go after this profile: the running version
/// and the next patch for a drop or raw repository, and the package drop
/// convention with an example (`vibe artifacts where`'s owners).
pub(super) fn where_to_put(layers: &ConfigLayers, warnings: &mut Vec<String>) -> Where {
    let current = env!("CARGO_PKG_VERSION");
    let next = release_dir_version(&format!("v{current}"))
        .map(|mut version| {
            version.patch += 1;
            version.pre = semver::Prerelease::EMPTY;
            version.to_string()
        })
        .unwrap_or_else(|| current.to_owned());
    let releases = match layers.release(None) {
        Ok(release)
            if matches!(
                release.source,
                ReleaseSource::Localdir { .. } | ReleaseSource::Nexus { .. }
            ) =>
        {
            [current.to_owned(), next]
                .into_iter()
                .map(|version| VersionedWhere {
                    release: crate::commands::artifacts::release_where_for(&release, &version),
                    version,
                })
                .collect()
        }
        _ => Vec::new(),
    };
    let packages = layers.packages_drop(warnings).map(|(drop, _)| {
        let target = release_source::release_target().unwrap_or("<target>");
        let example_file = format!("tool-1.2.0-{target}.tar.gz");
        PackagesWhere {
            dir: drop.display().to_string(),
            layout: ["<drop>/G/N/V/F", "<drop>/G/N/F", "<drop>/D/F", "<drop>/F"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            example: package_lookup(
                &drop,
                Some("org.example/tool@1.2.0"),
                "<sha256>",
                Some(&example_file),
            )
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        }
    });
    Where { releases, packages }
}

/// What in the user's own files outranks the site and defeats it: a
/// registry on a host the site excludes, a release decision of their own,
/// the install scripts' environment variable.
pub(super) fn conflict_warnings(layers: &ConfigLayers, effective: &Effective) -> Vec<String> {
    let mut warnings = Vec::new();
    let Some(site) = &layers.site else {
        return warnings;
    };
    let user_file = vibe_core::settings::registry_config_path()
        .map_or_else(|| "registry.toml".to_owned(), |p| p.display().to_string());
    let excluded = &site.profile.source_policy.exclude_hosting;
    for row in effective
        .registries
        .iter()
        .filter(|row| row.scope == "user")
    {
        if let Some(selector) = excluded
            .iter()
            .find(|selector| selector.matches_endpoint(&row.url))
        {
            warnings.push(format!(
                "`{user_file}` declares registry `{}` ({}) on `{selector}`, which the site profile \
                 excludes; it is tried before the site's registries and refused — remove it with \
                 `vibe registry remove {} --scope user`",
                row.name, row.url, row.name
            ));
        }
    }
    if site.profile.declares_release_source()
        && (!layers.own.self_source.is_default() || layers.own.artifacts.vibe.is_some())
    {
        warnings.push(
            "your own `[self]` / `[artifacts] vibe` in config.toml outranks the site profile's \
             release source; `vibe self source reset` (and removing `[artifacts] vibe`) hands it \
             to the site"
                .to_owned(),
        );
    }
    if matches!(
        release_source::active().map(|active| active.origin),
        Ok(SourceOrigin::Environment)
    ) {
        warnings.push(format!(
            "${} is set and outranks every configured release source for this shell",
            release_source::RELEASE_ORIGIN_ENV
        ));
    }
    warnings
}
