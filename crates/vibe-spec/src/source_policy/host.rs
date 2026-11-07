//! Finite cited-source capture; canonical lookup never grants raw files meaning.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-HOST-SYNC",
    r = 1
);
use super::*;
use std::collections::BTreeSet;
use vibe_core::manifest::{
    SelectedSource, SourceInterpretation, SourceSelector, select_source_rows,
};

/// One concrete native source captured for an exact host citation. This is
/// finite observation data, not a full-tree census or a publication capability.
#[derive(Debug)]
pub struct CitedNativeSource {
    pub relative: String,
    pub media: String,
    pub interpretation: SourceInterpretation,
    pub observation: vibe_safefs::SourceObservation,
    parents: Vec<vibe_safefs::FileIdentity>,
    document: String,
    coordinate: String,
    point_aliases: bool,
}
impl PackagePolicyObservation {
    /// A held control-entry presence cue only; no record body or asserted hash is
    /// read. Absence does not authenticate an original source.
    pub fn materialized_record_present(&self) -> Result<bool, SourcePolicyError> {
        self.recheck_unchanged()?;
        let root = self.project.root_dir().map_err(|error| {
            self.cited_failure(SourcePolicyErrorKind::Unavailable, error.to_string())
        })?;
        let name = vibe_core::layout::SLOT_RECORD_FILENAME;
        let present = self
            .project
            .source_file_present_in(&root, name)
            .map_err(|error| {
                self.cited_failure(SourcePolicyErrorKind::Unavailable, error.to_string())
            })?
            || root
                .open_child_checked(name)
                .map_err(|error| {
                    self.cited_failure(SourcePolicyErrorKind::Unavailable, error.to_string())
                })?
                .is_some();
        self.recheck_unchanged()?;
        Ok(present)
    }

    fn cited_failure(
        &self,
        kind: SourcePolicyErrorKind,
        reason: impl Into<String>,
    ) -> SourcePolicyError {
        failure(&self.state.logical_root, kind, reason)
    }
    fn candidates(
        &self,
        coordinate: &str,
        document: &str,
        point_aliases: bool,
    ) -> Result<Vec<(String, String, SourceInterpretation)>, SourcePolicyError> {
        self.recheck_unchanged()?;
        let actual = self
            .manifest()
            .and_then(|manifest| manifest.consumer_node())
            .and_then(|node| node.group.map(|group| format!("{group}/{}", node.name)));
        if actual.as_deref() != Some(coordinate) {
            return Err(self.unsupported("cited native lookup without matching self coordinate"));
        }
        vibe_core::manifest::declarant_path(Path::new(document))
            .map_err(|e| self.cited_failure(SourcePolicyErrorKind::Unsupported, e.reason()))?;
        let rows = self
            .source_rows()
            .ok_or_else(|| self.unsupported("declared cited source lookup without policy"))?;
        let matches = |path: &str| {
            crate::canonical_doc_path(path) == document
                || (point_aliases && crate::resolver::source_doc_spelling(path, &[]) == document)
        };
        let mut paths = BTreeSet::new();
        for row in rows {
            if let SourceSelector::File(path) = row.selector()
                && matches(path)
            {
                paths.insert(path.clone());
            }
        }
        let parent = Path::new(document)
            .parent()
            .unwrap_or_else(|| Path::new(""));
        // These are the same root spellings canonical_doc_path removes. Each
        // lookup inspects only the exact cited parent, never its descendants.
        let parents = [
            parent.to_path_buf(),
            vibe_core::layout::current_specs_root().join(parent),
            Path::new(vibe_core::layout::LEGACY_SPECS_DIR).join(parent),
        ];
        for relative in parents {
            if !rows.iter().any(|row| matches!(row.selector(),SourceSelector::Directory(path) if relative.starts_with(path))) {continue;}

            let mut directory = self.project.root_dir().map_err(|e| {
                self.cited_failure(SourcePolicyErrorKind::Unavailable, e.to_string())
            })?;
            let mut exists = true;
            for component in relative.iter() {
                let component = component.to_str().ok_or_else(|| {
                    self.cited_failure(SourcePolicyErrorKind::Unsupported, "non-UTF8 cited parent")
                })?;
                match directory.open_child_checked(component).map_err(|e| {
                    self.cited_failure(SourcePolicyErrorKind::Unavailable, e.to_string())
                })? {
                    Some(next) => directory = next,
                    None => {
                        exists = false;
                        break;
                    }
                }
            }
            if !exists {
                continue;
            }
            for name in self.project.child_names(&directory).map_err(|e| {
                self.cited_failure(SourcePolicyErrorKind::Unavailable, e.to_string())
            })? {
                // An unrepresentable physical name cannot own a portable
                // citation. Never normalize a literal backslash into traversal.
                if vibe_core::manifest::declarant_path_component(&name).is_err() {
                    continue;
                }
                let mut components = relative
                    .iter()
                    .map(|part| {
                        part.to_str().ok_or_else(|| {
                            self.cited_failure(
                                SourcePolicyErrorKind::Unsupported,
                                "unrepresentable cited parent",
                            )
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                components.push(&name);
                let path = components.join("/");
                if matches(&path) {
                    paths.insert(path);
                }
            }
        }
        let mut native = Vec::new();
        for path in paths {
            match select_source_rows(coordinate, rows, &path).map_err(|e| {
                self.cited_failure(
                    SourcePolicyErrorKind::Unsupported,
                    format!("invalid cited path: {e:?}"),
                )
            })? {
                SelectedSource::Descriptor { descriptor, .. }
                    if matches!(
                        descriptor.interpretation(),
                        SourceInterpretation::VibeSpec | SourceInterpretation::VibeDoc
                    ) =>
                {
                    native.push((path, descriptor.media().into(), descriptor.interpretation()))
                }
                SelectedSource::NeedsInput { .. } => {
                    return Err(self.cited_failure(
                        SourcePolicyErrorKind::Unsupported,
                        "ambiguous cited source interpretation",
                    ));
                }
                _ => {}
            }
        }
        self.recheck_unchanged()?;
        Ok(native)
    }
    fn cited_parents(
        &self,
        relative: &str,
    ) -> Result<Vec<vibe_safefs::FileIdentity>, SourcePolicyError> {
        let mut directory = self
            .project
            .root_dir()
            .map_err(|e| self.cited_failure(SourcePolicyErrorKind::Unavailable, e.to_string()))?;
        let mut identities =
            vec![directory.identity().map_err(|e| {
                self.cited_failure(SourcePolicyErrorKind::Unavailable, e.to_string())
            })?];
        for component in Path::new(relative)
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .iter()
        {
            directory = directory
                .open_child_checked(component.to_str().ok_or_else(|| {
                    self.cited_failure(SourcePolicyErrorKind::Unavailable, "non-UTF8 parent")
                })?)
                .map_err(|e| self.cited_failure(SourcePolicyErrorKind::Unavailable, e.to_string()))?
                .ok_or_else(|| {
                    self.cited_failure(
                        SourcePolicyErrorKind::Unavailable,
                        "cited parent disappeared",
                    )
                })?;
            identities.push(directory.identity().map_err(|e| {
                self.cited_failure(SourcePolicyErrorKind::Unavailable, e.to_string())
            })?);
        }
        Ok(identities)
    }
    /// Resolve only one cited canonical address through the shared selector.
    pub fn capture_cited_native(
        &self,
        coordinate: &str,
        document: &str,
    ) -> Result<CitedNativeSource, SourcePolicyError> {
        self.capture_native(coordinate, document, false)
    }
    /// Native point reads use the actual package's vocabulary. Projects never
    /// acquire documentation-package kind merely by declaring a row.
    pub fn capture_addressed_native(
        &self,
        coordinate: &str,
        document: &str,
    ) -> Result<CitedNativeSource, SourcePolicyError> {
        self.capture_native(coordinate, document, true)
    }
    fn capture_native(
        &self,
        coordinate: &str,
        document: &str,
        point_aliases: bool,
    ) -> Result<CitedNativeSource, SourcePolicyError> {
        let candidates = self.candidates(coordinate, document, point_aliases)?;
        let [(relative, media, interpretation)] = candidates.as_slice() else {
            return Err(self.cited_failure(
                if candidates.is_empty() {
                    SourcePolicyErrorKind::MissingNativeAddress
                } else {
                    SourcePolicyErrorKind::Unavailable
                },
                if candidates.is_empty() {
                    "cited native source is unavailable or classified as raw"
                } else {
                    "cited native source address is ambiguous"
                },
            ));
        };
        if !point_aliases && *interpretation != SourceInterpretation::VibeSpec {
            return Err(self.unsupported("host citation documentation vocabulary"));
        }
        if *interpretation == SourceInterpretation::VibeDoc
            && !self
                .package()
                .is_some_and(|package| package.kind == vibe_core::PackageKind::Doc)
        {
            return Err(self.unsupported("native documentation source outside a doc package"));
        }
        let parents = self.cited_parents(relative)?;
        let observation = self.read_source_observation(relative)?.ok_or_else(|| {
            self.cited_failure(
                SourcePolicyErrorKind::Unavailable,
                "cited native file is absent; absence is not anchor deletion",
            )
        })?;
        self.recheck_unchanged()?;
        if self.cited_parents(relative)? != parents {
            return Err(self.cited_failure(
                SourcePolicyErrorKind::Changed,
                "cited parent changed during capture",
            ));
        }
        Ok(CitedNativeSource {
            relative: relative.clone(),
            media: media.clone(),
            interpretation: *interpretation,
            observation,
            parents,
            document: document.into(),
            coordinate: coordinate.into(),
            point_aliases,
        })
    }
    /// Recheck the relevant native membership and the same finite source bytes.
    /// Unrelated raw directory entries do not invalidate this citation.
    pub fn recheck_cited_native(
        &self,
        source: &CitedNativeSource,
    ) -> Result<(), SourcePolicyError> {
        let current =
            self.capture_native(&source.coordinate, &source.document, source.point_aliases)?;
        if current.relative != source.relative
            || current.media != source.media
            || current.interpretation != source.interpretation
            || current.parents != source.parents
            || current.observation.identity() != source.observation.identity()
            || current.observation.bytes() != source.observation.bytes()
        {
            return Err(self.cited_failure(
                SourcePolicyErrorKind::Changed,
                "cited native source changed after capture",
            ));
        }
        Ok(())
    }
}
