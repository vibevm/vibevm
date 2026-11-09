//! The installation as it reads through its layers, where a person puts
//! releases and payloads, and what in the user's own files defeats the
//! site (`PROP-019#site-profile`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile");

use std::path::PathBuf;

use anyhow::Result;
use vibe_core::remap::drop::{package_lookup, release_dir_version};
use vibe_core::{GlobalRegistryConfig, RegistryLayers, RegistryScope};

use super::super::bundle::release_source::{self, ReleaseSource, SourceOrigin};
use super::super::store::VersionStore;
use super::ConfigLayers;
use vibe_wire::generated::self_apply_report::{
    Effective, InstallRootRow, PackagesWhere, ProxyRow, RegistryRow, RegistryScope as WireScope,
    RegistryType, ReleaseRow, VersionedWhere, WhereToPut,
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
    install_root: Option<(PathBuf, super::layers::InstallBaseOrigin)>,
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
        type_: registry_type(row.effective_type()),
        name: row.name,
        url: row.url,
        scope: match scope {
            RegistryScope::Site => WireScope::Site,
            _ => WireScope::User,
        },
    })
    .collect();
    if !user_file_exists && site.as_ref().is_none_or(|site| site.registries.is_empty()) {
        registries.extend(
            vibe_core::default_registries()
                .into_iter()
                .map(|row| RegistryRow {
                    type_: registry_type(row.effective_type()),
                    name: row.name,
                    url: row.url,
                    scope: WireScope::Default,
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
                source: release.source.wire_kind(),
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
        install_root: install_root.map(|(base, origin)| {
            let store = VersionStore::new(base.join("opt"));
            InstallRootRow {
                path: base.display().to_string(),
                from: origin.describe(),
                shim_dir: store.shim_dir().display().to_string(),
                data_dir: store.data_dir().display().to_string(),
            }
        }),
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
        proxy: proxy_row(layers, &merged.proxy),
        offline: merged.net.offline,
        site_remap_rows: layers.site.as_ref().map_or(0, |site| {
            u32::try_from(site.profile.remap.len()).unwrap_or(u32::MAX)
        }),
    })
}

/// The effective `[proxy]` with the layer behind its mode and PAC
/// (`PROP-068#proxy`). The environment switches (`VIBEVM_PROXY_MODE`,
/// `VIBEVM_PROXY_PAC`) are this process's: they show when the installed
/// route says it took them from there.
fn proxy_row(layers: &ConfigLayers, merged: &vibe_core::user_config::ProxyConfig) -> ProxyRow {
    use vibe_core::net::{ModeOrigin, effective_mode, effective_pac};
    let site = layers.site.as_ref().map(|site| &site.profile.proxy);
    let (mode, mut mode_origin) = effective_mode(site, &layers.own.proxy, None);
    let mut mode = mode
        .or_else(|| merged.effective_mode())
        .map(|m| m.as_str().to_owned());
    let (mut pac, mut pac_origin) = effective_pac(site, &layers.own.proxy, None);
    let route = vibe_core::net::route();
    if route.mode_origin() == ModeOrigin::Env {
        mode_origin = ModeOrigin::Env;
        mode = Some(match route.mode_name() {
            "direct" => "none".to_owned(),
            name => name.to_owned(),
        });
    }
    if let Some(settings) = route.px()
        && settings.pac_origin == ModeOrigin::Env
    {
        pac_origin = ModeOrigin::Env;
        pac = settings.pac.as_ref().map(|source| match source {
            vibe_core::user_config::PacSource::Url(v)
            | vibe_core::user_config::PacSource::File(v) => v.clone(),
            vibe_core::user_config::PacSource::System => "system".to_owned(),
        });
    }
    let pac_kind = pac
        .as_deref()
        .and_then(|raw| vibe_core::user_config::PacSource::parse(raw).ok())
        .map(|source| source.kind().to_owned());
    ProxyRow {
        mode,
        mode_origin: mode_origin.as_str().to_owned(),
        https: merged.https.clone(),
        http: merged.http.clone(),
        no_proxy: merged.no_proxy.clone(),
        pac,
        pac_kind,
        pac_origin: pac_origin.as_str().to_owned(),
        upstream: merged.upstream.clone(),
        px_port: merged.px_port.map(u32::from),
        px_path: merged.px_path.as_ref().map(|p| p.display().to_string()),
        ca_file: merged.ca_file.as_ref().map(|p| p.display().to_string()),
    }
}

/// Where releases and payloads go after this profile: the running version
/// and the next patch for a drop or raw repository, and the package drop
/// convention with an example (`vibe artifacts where`'s owners).
pub(super) fn where_to_put(layers: &ConfigLayers, warnings: &mut Vec<String>) -> WhereToPut {
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
                .map(|version| {
                    let release = crate::commands::artifacts::release_where_for(&release, &version);
                    // The writer flattens `release_where` beside the version,
                    // so the source is repeated per release directory.
                    VersionedWhere {
                        version,
                        source: release.source,
                        location: release.location,
                        from: release.from,
                        example_dir: release.example_dir,
                        files: release.files,
                    }
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
    WhereToPut { releases, packages }
}

/// One effective registry type as the wire vocabulary spells it.
fn registry_type(kind: vibe_core::manifest::RegistryType) -> RegistryType {
    match kind {
        vibe_core::manifest::RegistryType::Github => RegistryType::Github,
        vibe_core::manifest::RegistryType::Gitverse => RegistryType::Gitverse,
        vibe_core::manifest::RegistryType::Git => RegistryType::Git,
        vibe_core::manifest::RegistryType::Monorepo => RegistryType::Monorepo,
        vibe_core::manifest::RegistryType::Localdir => RegistryType::Localdir,
    }
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
        .filter(|row| row.scope == WireScope::User)
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
        && (!layers.own.self_source.release_is_default() || layers.own.artifacts.vibe.is_some())
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
