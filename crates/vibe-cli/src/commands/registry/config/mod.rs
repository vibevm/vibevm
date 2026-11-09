//! `vibe registry list / add / set-mirror / remove / test` —
//! `[[registry]]` and `[[mirror]]` configuration management,
//! one submodule per subcommand.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#registry");

mod add;
mod list;
mod mirror;
mod remove;
mod scope;
mod test;

#[cfg(test)]
mod tests;

pub(super) use add::run_add;
pub(super) use list::run_list;
pub(super) use mirror::run_set_mirror;
pub(super) use remove::run_remove;
pub(super) use test::run_test;

use anyhow::{Result, anyhow};
use serde::Serialize;
use vibe_core::manifest::NamingConvention;

#[derive(Debug, Serialize)]
struct ListReportRegistry {
    name: String,
    /// Effective backend family (PROP-002 `#registry-type`) — declared or
    /// inferred from the url.
    #[serde(rename = "type")]
    registry_type: String,
    /// Declaration layer the row came from (`local`, `project`, `user`,
    /// `site`).
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<String>,
    url: String,
    #[serde(rename = "ref")]
    refname: String,
    naming: String,
    host: String,
    org: String,
    /// Adapter that `vibe registry publish` would dispatch to for this
    /// registry's host. `null` if the host has no adapter today.
    adapter: Option<String>,
    /// Mirrors that fall through to this registry, in priority order.
    mirrors: Vec<ListReportMirror>,
}

#[derive(Debug, Serialize)]
struct ListReportMirror {
    of: String,
    url: String,
    priority: i32,
}

/// Map a host segment to the `RepoCreator` adapter `creator_for_url`
/// would pick. `None` means there is no adapter and `vibe registry
/// publish` would fail with `UnsupportedHost`. Pure read of the
/// dispatch rule in `vibe-publish::creator_for_url`; kept in sync by
/// hand because the rule is short and keeping it in code-as-data
/// would defer the user-facing label here from the rule there for no
/// real win.
fn adapter_for_host(host: &str) -> Option<&'static str> {
    let h = host.to_ascii_lowercase();
    if h == "github.com" || h.ends_with(".github.com") {
        return Some("github");
    }
    if h == "gitverse.ru" || h.ends_with(".gitverse.ru") {
        return Some("gitverse");
    }
    None
}

/// The publish adapter a registry of this type dispatches to; `None` for a
/// generic `git` registry (publish there needs `--repo-url`).
fn adapter_for(kind: vibe_core::manifest::RegistryType, host: &str) -> Option<&'static str> {
    use vibe_core::manifest::RegistryType;
    match kind {
        RegistryType::Github | RegistryType::Gitverse => {
            adapter_for_host(host).or(Some(kind.as_str()))
        }
        RegistryType::Monorepo | RegistryType::Localdir => Some(kind.as_str()),
        RegistryType::Git => None,
    }
}

/// The host and organization segments a registry row is reported with.
/// Pure presentation: `vibe registry add` and `list` never refuse a row
/// because a publish adapter could not address it (PROP-002 `#publish`
/// derives its organization only when publishing). A local directory and a
/// repository on this machine's filesystem have neither segment.
fn display_segments(kind: vibe_core::manifest::RegistryType, url: &str) -> (String, String) {
    if kind == vibe_core::manifest::RegistryType::Localdir {
        return (String::from("(local directory)"), String::new());
    }
    if vibe_registry::git_backend::shell::native_admission::local_repository(url).is_some() {
        return (String::from("(local repository)"), String::new());
    }
    (
        vibe_publish::extract_host_segment(url).unwrap_or_else(|_| String::from("?")),
        vibe_publish::extract_org_segment(url).unwrap_or_else(|_| String::from("?")),
    )
}

/// The `vibe.toml` spelling of a naming convention.
fn naming_label(naming: NamingConvention) -> &'static str {
    match naming {
        NamingConvention::Fqdn => "fqdn",
        NamingConvention::KindName => "kind-name",
        NamingConvention::Name => "name",
        NamingConvention::KindSlashName => "kind/name",
    }
}

/// Parse the `--naming` CLI argument. Mirrors the serde `rename`s on
/// `NamingConvention` so what users type matches the `vibe.toml`
/// spelling exactly.
fn parse_naming(s: &str) -> Result<NamingConvention> {
    match s {
        "fqdn" => Ok(NamingConvention::Fqdn),
        "kind-name" => Ok(NamingConvention::KindName),
        "name" => Ok(NamingConvention::Name),
        "kind/name" => Ok(NamingConvention::KindSlashName),
        other => Err(anyhow!(
            "unknown naming convention `{other}` — must be one of `fqdn`, `kind-name`, `name`, `kind/name`"
        )),
    }
}
