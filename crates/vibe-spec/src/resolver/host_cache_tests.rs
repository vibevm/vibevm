//! Real compiler discovery must recheck retained host inputs before cache reuse.
#[cfg(test)]
mod cases {
    use crate::compiler::ir::{
        ArtifactContext, ArtifactFrame, ArtifactId, ArtifactInput, ArtifactPlan, ArtifactTarget,
        StaticCompileMode,
    };
    use crate::{
        FileResolver, FsSectionSource, ResolvedSource, ResolvedSourceMetadata, SectionSource,
        SelfCoordinate, SpecAddress,
    };
    use std::{fs, path::PathBuf};
    fn addr(doc: &str) -> SpecAddress {
        SpecAddress::parse(&format!("spec://org.host/host/{doc}#root")).unwrap()
    }
    fn manifest(interpretation: Option<&str>) -> String {
        let mut text = "[project]\ngroup='org.host'\nname='host'\nversion='1.0.0'\n".to_owned();
        if let Some(interpretation) = interpretation {
            let role = if interpretation == "vibe-spec" {
                "spec"
            } else {
                "resource"
            };
            text += &format!(
                "[[project.sources]]\ndirectory='spec'\nmedia='text/markdown'\ninterpretation='{interpretation}'\nrole='{role}'\nloading='explicit'\n"
            );
        }
        text
    }
    struct MutatingSource {
        inner: FsSectionSource,
        root: PathBuf,
        legacy: bool,
        enforce: bool,
    }
    impl SectionSource for MutatingSource {
        fn section_text(&self, address: &SpecAddress) -> Result<String, String> {
            self.inner.section_text(address)
        }
        fn source_metadata(
            &self,
            address: &SpecAddress,
            formats: &[String],
        ) -> Result<ResolvedSourceMetadata, String> {
            self.inner.source_metadata(address, formats)
        }
        fn read_resolved_source(
            &self,
            address: &SpecAddress,
            metadata: ResolvedSourceMetadata,
        ) -> Result<ResolvedSource, String> {
            let source = self.inner.read_resolved_source(address, metadata)?;
            if address.doc_path == "B" {
                if self.legacy {
                    fs::write(self.root.join("vibe.toml"), manifest(Some("commonmark"))).unwrap();
                } else {
                    fs::write(self.root.join("spec/A.md"), "# A {#root}\nCHANGED-A\n").unwrap();
                }
            }
            Ok(source)
        }
        fn admit_reference(&self, address: &SpecAddress) -> Result<(), String> {
            if self.enforce {
                self.inner.admit_reference(address)
            } else {
                Ok(())
            }
        }
    }
    fn plan() -> ArtifactPlan {
        let context = ArtifactContext::new(
            ArtifactId::new("static-md").unwrap(),
            ArtifactTarget::StaticMarkdown,
            ArtifactFrame::StaticLane {
                generated_path: "vibevm/vibespecs/boot/STATIC.md".into(),
                source_root: "vibevm/vibedeps".into(),
            },
            StaticCompileMode::QualifyPerNode,
        )
        .unwrap();
        ArtifactPlan::new(
            context,
            vec![
                ArtifactInput::normal("org.host/host", "spec/A.md", addr("A")).unwrap(),
                ArtifactInput::normal("org.host/host", "spec/B.md", addr("B")).unwrap(),
            ],
        )
        .unwrap()
    }
    #[test]
    fn actual_compiler_cached_host_reference_refuses_byte_or_legacy_policy_drift() {
        for legacy in [false, true] {
            for enforce in [false, true] {
                let root = tempfile::tempdir().unwrap();
                fs::create_dir(root.path().join("spec")).unwrap();
                fs::write(
                    root.path().join("vibe.toml"),
                    manifest((!legacy).then_some("vibe-spec")),
                )
                .unwrap();
                fs::write(root.path().join("spec/A.md"), "# A {#root}\nORIGINAL-A\n").unwrap();
                fs::write(
                    root.path().join("spec/B.md"),
                    "# B {#root}\n#use spec://org.host/host/A#root\nBODY-B\n",
                )
                .unwrap();
                let source = MutatingSource {
                    inner: FsSectionSource::new(FileResolver::new(
                        root.path(),
                        SelfCoordinate::new(Some("org.host".into()), "host".into()),
                    )),
                    root: root.path().into(),
                    legacy,
                    enforce,
                };
                let compiled = crate::compiler::builtin::compile_artifact(plan(), &source);
                if enforce {
                    let error = compiled.unwrap_err().to_string();
                    assert!(
                        error.contains("changed") || error.contains("legacy to declared"),
                        "legacy={legacy}: {error}"
                    );
                } else {
                    assert!(
                        compiled.is_ok(),
                        "counterfactual reference hook bypass must expose stale reuse: {compiled:?}"
                    );
                }
            }
        }
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn many_declared_documents_share_one_retained_root_under_low_fd_limit() {
        const CHILD: &str = "NST_HOST_READ_LOW_FD_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let output=std::process::Command::new("bash").args(["-c","ulimit -n 64; exec \"$@\"","host-read-test"])
                .arg(std::env::current_exe().unwrap()).args(["--exact","resolver::host_cache_tests::cases::many_declared_documents_share_one_retained_root_under_low_fd_limit","--test-threads=1"])
                .env(CHILD,"1").output().unwrap();
            assert!(
                output.status.success(),
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("spec")).unwrap();
        fs::write(root.path().join("vibe.toml"), manifest(Some("vibe-spec"))).unwrap();
        let source = FsSectionSource::new(FileResolver::new(
            root.path(),
            SelfCoordinate::new(Some("org.host".into()), "host".into()),
        ));
        for index in 0..160 {
            let doc = format!("D{index}");
            fs::write(
                root.path().join(format!("spec/{doc}.md")),
                format!("# D {{#root}}\nBODY-{index}\n"),
            )
            .unwrap();
            assert!(
                source
                    .section_text(&addr(&doc))
                    .unwrap()
                    .contains(&format!("BODY-{index}"))
            );
        }
        assert!(source.section_text(&addr("D0")).unwrap().contains("BODY-0"));
    }
}
