//! Pure prospective facts-overlay validation through existing derivation preparation.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-FACT-COMMANDS",
    r = 1
);
use super::*;
/// Retained original-source observation from pure declared derivation preflight.
/// This is finite predictable-error admission, not a publication transaction.
pub struct DeclaredDerivationPreflight {
    policy: super::super::source_policy::SourcePolicy,
}
impl DeclaredDerivationPreflight {
    /// Check the held source observations again before registry publication.
    pub fn recheck(&self) -> Result<(), WorkspaceError> {
        self.policy.recheck()
    }
}

/// Check prospective adoption through the materializer's existing converter,
/// selection and collision preparation without publishing any output.
pub fn preflight_declared_derivation(
    content_src: &Path,
    manifest: &vibe_core::manifest::Manifest,
    format: SpecFormat,
    mode: CopyMode,
    overlay: &PackageOverlay,
) -> Result<DeclaredDerivationPreflight, WorkspaceError> {
    let policy =
        super::super::source_policy::SourcePolicy::observe(content_src, format, Some(manifest))?;
    if !policy.is_declared() {
        return Err(super::super::source_policy::refused(
            content_src,
            "declared derivation preflight requires declared source policy",
        ));
    }
    declared::prepare(&policy, format, mode, overlay)?;
    policy.recheck()?;
    Ok(DeclaredDerivationPreflight { policy })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prospective_preflight_refuses_collision_and_retains_source_drift_guard() {
        let source = tempfile::tempdir().unwrap();
        let manifest = vibe_core::manifest::Manifest::parse_str("[package]\ngroup='org.example'\nname='rules'\nkind='flow'\nversion='1.0.0'\n[[package.sources]]\nfile='law.native'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n").unwrap();
        fs::write(
            source.path().join("vibe.toml"),
            toml::to_string(&manifest).unwrap(),
        )
        .unwrap();
        fs::write(source.path().join("law.native"), "<spec xmlns=\"https://vibevm.org/spec/1\"><p><LAW fact=\"true\" status=\"impl/done\">law</LAW></p></spec>").unwrap();
        let registry = Registry::default();
        let overlay = registry
            .package_overlay_with_additions(
                "org.example/rules",
                &[vibe_facts::FactEntry {
                    address: "spec://org.example/rules/law.native#LAW".into(),
                    origin: vibe_facts::FactOrigin::Package,
                    package: Some("org.example/rules".into()),
                    status: Some(FactStatus::parse("spec/work").unwrap()),
                    comment: None,
                }],
            )
            .unwrap();
        let guard = preflight_declared_derivation(
            source.path(),
            &manifest,
            SpecFormat::Markdown,
            CopyMode::Copy,
            &overlay,
        )
        .unwrap();
        assert!(!source.path().join("law.md").exists());
        assert!(!source.path().join(SLOT_RECORD_FILENAME).exists());
        fs::write(source.path().join("law.md"), "raw collision").unwrap();
        assert!(
            preflight_declared_derivation(
                source.path(),
                &manifest,
                SpecFormat::Markdown,
                CopyMode::Copy,
                &overlay
            )
            .is_err()
        );
        fs::write(source.path().join("law.native"), "changed").unwrap();
        assert!(guard.recheck().is_err());
        assert_eq!(
            fs::read_to_string(source.path().join("law.md")).unwrap(),
            "raw collision"
        );
        assert!(registry.is_empty());
    }
}
