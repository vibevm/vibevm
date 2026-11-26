//! Human-only paths and effective choices; wire provenance stays unchanged.

use std::path::Path;

use vibe_core::net::ModeOrigin;
use vibe_core::user_config::UserConfig;

use super::{ConfigReport, forward_slash_display};
use crate::commands::vvm::bundle::release_source::{self, SourceOrigin};
use crate::output::language::{Cell, Meta, Row};

pub(super) fn provenance(report: &ConfigReport, origin: &str) -> String {
    let path = match origin {
        "~/.vibe/registry.toml" => vibe_core::settings::registry_config_path(),
        "site-profile.toml" => vibe_core::settings::site_profile_path(),
        "vibe.toml" => Some(Path::new(&report.project).join("vibe.toml")),
        ".vibe/registry.local.toml" => {
            Some(Path::new(&report.project).join(".vibe/registry.local.toml"))
        }
        _ => None,
    };
    path.as_deref()
        .map(forward_slash_display)
        .unwrap_or_else(|| origin.to_owned())
}

fn row(group: &str, key: &str, value: &str, origin: &str) -> Row {
    Row::new(vec![Cell::text(group), Cell::text(key), Cell::text(value)])
        .meta(vec![Meta::State(origin.into())])
}

pub(super) fn effective(own: &UserConfig) -> Vec<Row> {
    let installed = vibe_core::settings::site_profile_path()
        .and_then(|path| vibe_core::site_profile::installed_at(&path).ok().flatten());
    let site = installed.as_ref().map(|site| &site.profile);
    let merged = match site {
        Some(site) => own.clone().over_site(site),
        None => own.clone(),
    };
    let mut rows = Vec::new();
    for path in vibe_core::user_config::effective_extra_ca(&merged.tls, &merged.proxy) {
        let alias = merged.proxy.ca_file.as_ref() == Some(&path);
        let user = if alias {
            own.proxy.ca_file.is_some()
        } else {
            own.tls.extra_ca.is_some()
        };
        rows.push(row(
            "tls",
            "extra_ca",
            &forward_slash_display(&path),
            if user { "user" } else { "site" },
        ));
    }
    let (mode, origin) =
        vibe_core::net::effective_mode(site.map(|site| &site.proxy), &own.proxy, None);
    let route = vibe_core::net::route();
    let (mode, origin) = if route.mode_origin() == ModeOrigin::Env {
        (
            match route.mode_name() {
                "direct" => "none",
                name => name,
            },
            "env",
        )
    } else {
        let mode = mode.or_else(|| merged.proxy.effective_mode());
        let origin = match origin {
            ModeOrigin::User => "user",
            ModeOrigin::Site => "site",
            _ if own.proxy.effective_mode().is_some() => "user",
            _ if merged.proxy.effective_mode().is_some() => "site",
            _ => "default",
        };
        (mode.map_or("none", |mode| mode.as_str()), origin)
    };
    rows.push(row("proxy", "mode", mode, origin));
    // The active resolver includes environment precedence and the release
    // decision taken whole from the declaring user or site layer.
    if let Ok(release) = release_source::active() {
        let origin = match release.origin {
            SourceOrigin::BuiltIn => "default",
            SourceOrigin::Environment => "env",
            SourceOrigin::SelfTable(_) | SourceOrigin::ArtifactsDrop(_) => {
                if !own.self_source.release_is_default() || own.artifacts.vibe.is_some() {
                    "user"
                } else {
                    "site"
                }
            }
        };
        rows.push(row("self", "source", release.source.kind(), origin));
    }
    rows
}
