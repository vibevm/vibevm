//! The artifact drops and release source of this invocation, read from
//! the user config for the Locator (`PROP-066#drops`,
//! `PROP-019#release-sources`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-066#drops");

use std::path::PathBuf;

/// Read `[self]` and `[artifacts]` from the user config and resolve the
/// release source once (`PROP-019#release-sources`, `PROP-066#drops`). An
/// unreadable config or an unusable source is a warning for this run and
/// is recorded, so `vibe self` refuses with the reason instead of quietly
/// going back to the public host.
pub(super) fn artifact_drops(
    release_origin: Option<&str>,
    warnings: &mut Vec<String>,
) -> (
    Option<crate::commands::vvm::bundle::release_source::EffectiveReleaseSource>,
    Option<(PathBuf, String)>,
) {
    use crate::commands::vvm::bundle::release_source;
    let path = vibe_core::user_config::UserConfig::default_path();
    let config = match vibe_core::user_config::UserConfig::load() {
        Ok(config) => config,
        Err(error) => {
            warnings.push(error.to_string());
            vibe_core::user_config::UserConfig::default()
        }
    };
    let resolved = release_source::resolve(release_origin, &config, path.as_deref());
    if let Err(error) = &resolved {
        warnings.push(error.to_string());
    }
    release_source::install(resolved.clone());
    let declared_in = path
        .as_deref()
        .map_or_else(|| "config.toml".to_owned(), |p| p.display().to_string());
    let packages = match config.artifacts.packages {
        Some(drop) if drop.is_absolute() => Some((drop, declared_in)),
        Some(drop) => {
            warnings.push(format!(
                "`[artifacts] packages = \"{}\"` in `{declared_in}` is relative and was ignored \
                 (violates spec://org.vibevm.core/vibevm/common/PROP-066#drops; fix: give the \
                 drop as an absolute path)",
                drop.display()
            ));
            None
        }
        None => None,
    };
    (resolved.ok(), packages)
}
