//! Input normalisation at the command boundary: the effective spec format,
//! the generator stamp, and the canonical project root.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::manifest::{Manifest, SpecFormat};
use vibe_core::user_config::UserConfig;

/// Effective PROP-045 setting: a project pin is reproducible and wins over
/// the operator default; absence at both layers preserves legacy `mixed`.
pub(crate) fn resolve_spec_format(manifest: &Manifest, user_config: &UserConfig) -> SpecFormat {
    manifest
        .consumer_node()
        .and_then(|node| node.spec_format)
        .or(user_config.install.spec_format)
        .unwrap_or_default()
}

/// The lockfile provenance stamp this binary writes.
pub(crate) fn generated_by() -> String {
    format!("vibe {}", env!("CARGO_PKG_VERSION"))
}

pub(crate) fn resolve_project_root(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("canonicalizing `{}`", path.display()))?;
    let stripped = crate::commands::init::strip_unc_public(canonical);
    if !stripped.join(Manifest::FILENAME).exists() {
        bail!(
            "no `vibe.toml` in `{}`; run `vibe init` first",
            stripped.display()
        );
    }
    Ok(stripped)
}

#[cfg(test)]
mod spec_format_tests {
    use super::*;

    fn manifest(project_setting: Option<SpecFormat>) -> Manifest {
        let mut manifest: Manifest =
            toml::from_str("[project]\nname = \"demo\"\nversion = \"0.1.0\"\n")
                .expect("valid manifest");
        manifest.project.as_mut().expect("project").spec_format = project_setting;
        manifest
    }

    #[test]
    fn package_rooted_spec_format_is_equipotent() {
        // PROP-024 ##MANIFEST-ROLES-ARE-EQUIPOTENT: a package-rooted
        // checkout pins its materialisation exactly as a project does.
        let manifest: Manifest = toml::from_str(
            "[package]
name = \"b\"
group = \"org.x\"
kind = \"flow\"
version = \"1.0.0\"
spec_format = \"xml\"
",
        )
        .expect("valid manifest");
        let user = UserConfig::default();
        assert_eq!(resolve_spec_format(&manifest, &user), SpecFormat::Xml);
    }

    #[test]
    fn project_spec_format_wins_over_user_default() {
        let mut user = UserConfig::default();
        user.install.spec_format = Some(SpecFormat::Markdown);
        assert_eq!(
            resolve_spec_format(&manifest(Some(SpecFormat::Xml)), &user),
            SpecFormat::Xml
        );
    }

    #[test]
    fn user_default_and_builtin_mixed_fill_absent_project_setting() {
        let mut user = UserConfig::default();
        user.install.spec_format = Some(SpecFormat::Markdown);
        assert_eq!(
            resolve_spec_format(&manifest(None), &user),
            SpecFormat::Markdown
        );
        assert_eq!(
            resolve_spec_format(&manifest(None), &UserConfig::default()),
            SpecFormat::Mixed
        );
    }
}
