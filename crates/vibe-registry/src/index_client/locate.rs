//! Where is this registry's index? — the location ladder (PROP-005
//! §2.2 `#form-factor`, B-083).
//!
//! Split from `mod.rs` along the responsibility seam when the ladder
//! pushed that file past the 600-line budget: everything here answers
//! one question — which base URL (if any) the index client should
//! probe for a given `[[registry]]` — and nothing here talks HTTP.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#form-factor");

use vibe_core::manifest::RegistrySection;

/// Resolve `<index_url>` for the named registry from environment.
/// Mirrors the `VIBEVM_INDEX_URL_<REGISTRY>` shape used by
/// `vibe-publish::post_hook`.
pub fn index_url_for(registry: &str) -> Option<String> {
    let suffix = registry_env_suffix(registry);
    if suffix.is_empty() {
        return None;
    }
    std::env::var(format!("VIBEVM_INDEX_URL_{suffix}"))
        .ok()
        .filter(|s| !s.trim().is_empty())
}

fn registry_env_suffix(registry: &str) -> String {
    let mut out = String::with_capacity(registry.len());
    for c in registry.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_uppercase());
        } else {
            out.push('_');
        }
    }
    out
}

/// The index-location ladder's answer for one registry (PROP-005 §2.2
/// `#form-factor`, B-083): the base URL the index client should probe,
/// or that the index is switched off for this registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexUrlResolution {
    /// Probe this base URL. `source` names the rung that produced it —
    /// a consumer that reports reachability needs the difference: a
    /// dead *explicit* URL (env or key) is an operator-visible problem,
    /// a dead *default* guess is just a registry without an index.
    Url {
        base: String,
        source: IndexUrlSource,
    },
    /// The exact value `none` on an explicit step (env or key): index
    /// lookup is switched off for this registry — no probe, no network,
    /// straight to the `git ls-remote` path.
    Disabled,
}

/// Which rung of the ladder produced the URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexUrlSource {
    /// `VIBEVM_INDEX_URL_<REGISTRY>` — the operator's per-run re-point.
    Env,
    /// The `[[registry]].index_url` manifest key — a project property.
    ManifestKey,
    /// `<registry-url>/index` — the unset-rung guess the spec
    /// prescribes trying (`##INDEX-URL-DEFAULT`).
    Default,
}

/// The index-location ladder for one `[[registry]]` (PROP-005 §2.2
/// `#form-factor`): env `VIBEVM_INDEX_URL_<REGISTRY>` → the
/// `index_url` manifest key → the default `<registry-url>/index`.
/// The env var is an operator's per-run re-point and wins; the key is
/// a project property; the default is what the resolver tries when
/// neither is set. The exact value `none` on either explicit step
/// returns [`IndexUrlResolution::Disabled`] — before this ladder
/// existed, `none` in the env var reached the probe and died in URL
/// parsing, which happened to look like "no index"; the ladder makes
/// that outcome deliberate and free of any parse accident.
///
/// The name normalization for the env step is [`index_url_for`]'s and
/// is unchanged. The same `RegistrySection` shape serves the project
/// `vibe.toml` and the machine-global `~/.vibe/registry.toml`, so the
/// key carries in both columns.
pub fn resolve_index_url(registry: &RegistrySection) -> IndexUrlResolution {
    resolve_index_url_with(index_url_for(&registry.name), registry)
}

/// The ladder's pure core: the env step is passed in (already read)
/// so the rung arithmetic is testable without mutating process env —
/// forbidden under this crate's `#![forbid(unsafe_code)]` on edition
/// 2024, where `std::env::set_var` is `unsafe`. The same split as
/// `IndexAuth::plan` behind `IndexAuth::for_registry`.
fn resolve_index_url_with(env: Option<String>, registry: &RegistrySection) -> IndexUrlResolution {
    if let Some(resolved) = explicit_step(env.as_deref(), IndexUrlSource::Env) {
        return resolved;
    }
    if let Some(resolved) =
        explicit_step(registry.index_url.as_deref(), IndexUrlSource::ManifestKey)
    {
        return resolved;
    }
    IndexUrlResolution::Url {
        base: format!("{}/index", registry.url.trim_end_matches('/')),
        source: IndexUrlSource::Default,
    }
}

/// One explicit rung of the ladder: an absent or whitespace-only value
/// means the rung is unset (fall to the next one); the exact value
/// `none` switches the index off; anything else is the base URL to
/// probe, passed through as written — URL parsing is the probe's job.
fn explicit_step(raw: Option<&str>, source: IndexUrlSource) -> Option<IndexUrlResolution> {
    let value = raw?.trim();
    if value.is_empty() {
        return None;
    }
    if value == "none" {
        return Some(IndexUrlResolution::Disabled);
    }
    Some(IndexUrlResolution::Url {
        base: value.to_string(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibe_core::manifest::{AuthKind, NamingConvention};

    #[test]
    fn registry_env_suffix_uppercases() {
        assert_eq!(registry_env_suffix("vibespecs"), "VIBESPECS");
        assert_eq!(
            registry_env_suffix("vibespecs-gitverse"),
            "VIBESPECS_GITVERSE"
        );
    }

    // The rung arithmetic is exercised through the pure core
    // (`resolve_index_url_with`) with the env step passed in: mutating
    // process env from libtest is off the table — multi-threaded
    // `set_var` is the exact UB that made it `unsafe` in edition 2024,
    // and this crate carries `#![forbid(unsafe_code)]`. The env rung's
    // real read (`index_url_for`) is one `std::env::var` shared with
    // the pre-ladder code path.

    fn ladder_section(name: &str, url: &str, index_url: Option<&str>) -> RegistrySection {
        RegistrySection {
            name: name.to_string(),
            url: url.to_string(),
            r#ref: "main".to_string(),
            naming: NamingConvention::Fqdn,
            auth: AuthKind::None,
            token_env: None,
            enabled: true,
            index_url: index_url.map(|s| s.to_string()),
        }
    }

    #[test]
    #[specmark::verifies(
        "spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#form-factor",
        r = 1
    )]
    fn ladder_env_beats_key_and_default() {
        let reg = ladder_section(
            "r",
            "https://github.com/vibespecs",
            Some("https://key.example"),
        );
        let got = resolve_index_url_with(Some("https://env.example".into()), &reg);
        assert_eq!(
            got,
            IndexUrlResolution::Url {
                base: "https://env.example".into(),
                source: IndexUrlSource::Env,
            }
        );
    }

    #[test]
    #[specmark::verifies(
        "spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#form-factor",
        r = 1
    )]
    fn ladder_key_beats_default() {
        let reg = ladder_section(
            "r",
            "https://github.com/vibespecs",
            Some("https://key.example"),
        );
        let got = resolve_index_url_with(None, &reg);
        assert_eq!(
            got,
            IndexUrlResolution::Url {
                base: "https://key.example".into(),
                source: IndexUrlSource::ManifestKey,
            }
        );
    }

    #[test]
    #[specmark::verifies(
        "spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#form-factor",
        r = 1
    )]
    fn ladder_defaults_to_registry_url_slash_index() {
        let reg = ladder_section("r", "https://github.com/vibespecs", None);
        let got = resolve_index_url_with(None, &reg);
        assert_eq!(
            got,
            IndexUrlResolution::Url {
                base: "https://github.com/vibespecs/index".into(),
                source: IndexUrlSource::Default,
            }
        );
        // A trailing slash on the registry URL must not double up.
        let reg = ladder_section("r", "https://github.com/vibespecs/", None);
        let got = resolve_index_url_with(None, &reg);
        assert_eq!(
            got,
            IndexUrlResolution::Url {
                base: "https://github.com/vibespecs/index".into(),
                source: IndexUrlSource::Default,
            }
        );
    }

    #[test]
    fn ladder_none_on_the_env_step_disables() {
        let reg = ladder_section(
            "r",
            "https://github.com/vibespecs",
            Some("https://key.example"),
        );
        let got = resolve_index_url_with(Some("none".into()), &reg);
        assert_eq!(got, IndexUrlResolution::Disabled);
    }

    #[test]
    fn ladder_none_on_the_key_step_disables() {
        let reg = ladder_section("r", "https://github.com/vibespecs", Some("none"));
        let got = resolve_index_url_with(None, &reg);
        assert_eq!(got, IndexUrlResolution::Disabled);
    }

    #[test]
    fn ladder_whitespace_only_rungs_fall_through() {
        // A whitespace-only env value is already filtered by
        // `index_url_for`; the core defends the same way for the key,
        // and an env set to spaces must not mask a real key.
        let reg = ladder_section(
            "r",
            "https://github.com/vibespecs",
            Some("https://key.example"),
        );
        let got = resolve_index_url_with(Some("   ".into()), &reg);
        assert_eq!(
            got,
            IndexUrlResolution::Url {
                base: "https://key.example".into(),
                source: IndexUrlSource::ManifestKey,
            }
        );

        let reg = ladder_section("r", "https://github.com/vibespecs", Some("   "));
        let got = resolve_index_url_with(None, &reg);
        assert_eq!(
            got,
            IndexUrlResolution::Url {
                base: "https://github.com/vibespecs/index".into(),
                source: IndexUrlSource::Default,
            }
        );
    }
}
