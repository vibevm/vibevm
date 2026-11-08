//! Read checkpoints around the retained source, never an analyzer-observer veto.
use vibe_spec::{ResolvedSource, ResolvedSourceMetadata, SectionSource, SpecAddress};
pub(super) struct ControlledSource<'a, S: SectionSource> {
    pub(super) source: &'a S,
    pub(super) control: Option<&'a vibe_core::cancellation::CancellationToken>,
}
impl<S: SectionSource> ControlledSource<'_, S> {
    fn check(&self) -> Result<(), String> {
        self.control.map_or(Ok(()), |control| {
            control.checkpoint().map_err(|error| error.to_string())
        })
    }
    fn checked<T>(&self, read: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
        self.check()?;
        let value = read()?;
        self.check()?;
        Ok(value)
    }
}
impl<S: SectionSource> SectionSource for ControlledSource<'_, S> {
    fn section_text(&self, address: &SpecAddress) -> Result<String, String> {
        self.checked(|| self.source.section_text(address))
    }
    fn admit_reference(&self, address: &SpecAddress) -> Result<(), String> {
        self.checked(|| self.source.admit_reference(address))
    }
    fn root_source_metadata(
        &self,
        address: &SpecAddress,
        formats: &[String],
    ) -> Result<ResolvedSourceMetadata, String> {
        self.checked(|| self.source.root_source_metadata(address, formats))
    }
    fn read_resolved_root_source(
        &self,
        address: &SpecAddress,
        metadata: ResolvedSourceMetadata,
    ) -> Result<ResolvedSource, String> {
        self.checked(|| self.source.read_resolved_root_source(address, metadata))
    }
    fn source_metadata(
        &self,
        address: &SpecAddress,
        formats: &[String],
    ) -> Result<ResolvedSourceMetadata, String> {
        self.checked(|| self.source.source_metadata(address, formats))
    }
    fn read_resolved_source(
        &self,
        address: &SpecAddress,
        metadata: ResolvedSourceMetadata,
    ) -> Result<ResolvedSource, String> {
        self.checked(|| self.source.read_resolved_source(address, metadata))
    }
    fn expand_pattern(&self, address: &SpecAddress) -> Result<Vec<SpecAddress>, String> {
        self.checked(|| self.source.expand_pattern(address))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct StopAfterRead {
        path: std::path::PathBuf,
        token: vibe_core::cancellation::CancellationToken,
    }
    impl SectionSource for StopAfterRead {
        fn section_text(&self, _: &SpecAddress) -> Result<String, String> {
            let text = std::fs::read_to_string(&self.path).map_err(|error| error.to_string())?;
            self.token.cancel();
            Ok(text)
        }
    }
    #[test]
    fn stop_during_source_read_reaches_the_real_compiler_before_emission() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("source.md");
        std::fs::write(&path, "# Root {#root}\n\nkept source\n").unwrap();
        let token = vibe_core::cancellation::CancellationToken::new();
        let inner = StopAfterRead {
            path: path.clone(),
            token: token.clone(),
        };
        let source = ControlledSource {
            source: &inner,
            control: Some(&token),
        };
        let plan = vibe_spec::ArtifactPlan::static_lane(
            vibe_spec::ArtifactTarget::StaticMarkdown,
            "vibevm/vibespecs/boot/STATIC.md",
            "vibevm/vibedeps",
            vec![
                vibe_spec::ArtifactInput::normal(
                    "org.demo/pkg",
                    "source.md",
                    vibe_spec::SpecAddress::parse("spec://org.demo/pkg/source#root").unwrap(),
                )
                .unwrap(),
            ],
        )
        .unwrap();
        let result = vibe_spec::compile_artifact(plan, &source);
        assert!(result.is_err());
        assert!(token.acknowledged());
        assert!(
            std::fs::read_to_string(path)
                .unwrap()
                .contains("kept source")
        );
    }
}
