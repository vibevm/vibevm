//! Private initial generated seeds; ordinary references never enter this lane.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-INSTALLED-READ",
    r = 1
);
use super::*;
use crate::boot::EffectiveBoot;
use sha2::{Digest, Sha256};
use vibe_spec::{
    Authority, FsSectionSource, ResolvedSource, ResolvedSourceMetadata, SectionSource,
};

#[derive(Clone)]
struct GeneratedInput {
    path: PathBuf,
    group: String,
    name: String,
    relative: String,
    bytes: Arc<[u8]>,
    digest: [u8; 32],
    media: &'static str,
    overlay: bool,
    policy: Arc<vibe_spec::source_policy::PackagePolicyCheckpoint>,
}
/// Operation-private compiler reader. Generated entries are available only as
/// initial typed seeds, never as authored source rows or reference targets.
pub(crate) struct BootSectionSource {
    ordinary: FsSectionSource,
    context: Option<DeclaredBootSources>,
    self_coord: SelfCoordinate,
    generated: BTreeMap<(String, String, String), GeneratedInput>,
}
impl BootSectionSource {
    pub(crate) fn has_generated(&self, entry: &BootEntry) -> bool {
        self.context.as_ref().is_some_and(|context| {
            self.generated
                .values()
                .any(|value| value.path == context.root.join(&entry.path))
        })
    }
    pub(crate) fn new(
        root: &Path,
        self_coord: &SelfCoordinate,
        boot: &EffectiveBoot,
        render_format: SpecFormat,
        context: Option<&DeclaredBootSources>,
        overlay: Option<&BTreeMap<PathBuf, Arc<[u8]>>>,
    ) -> Result<Self, WorkspaceError> {
        let resolver = match context {
            Some(context) => context.resolver(self_coord)?,
            None => FileResolver::new(root, self_coord.clone()),
        };
        let ordinary = match overlay {
            Some(values) => FsSectionSource::with_overlay(resolver, values.clone()),
            None => FsSectionSource::new(resolver),
        };
        let mut result = Self {
            ordinary,
            context: context.cloned(),
            self_coord: self_coord.clone(),
            generated: BTreeMap::new(),
        };
        if let Some(context) = context {
            for entry in boot
                .static_entries()
                .filter(|entry| entry.unit_substituted && !entry.elided && !entry.use_ref)
            {
                let BootProvenance::Dependency { group, name } = &entry.provenance else {
                    return Err(error(
                        Path::new(&entry.path),
                        "generated unit lacks dependency provenance",
                    ));
                };
                let selected = context
                    .selected
                    .get(&(group.to_string(), name.clone()))
                    .ok_or_else(|| {
                        error(Path::new(&entry.path), "generated unit is not selected")
                    })?;
                let absolute = root.join(&entry.path);
                let relative = absolute
                    .strip_prefix(&selected.root)
                    .map_err(|_| error(&absolute, "generated unit escapes selected slot"))?;
                let expected = vibe_core::layout::current_boot_dir()
                    .join(crate::boot_artifacts::static_file(render_format));
                if relative != expected {
                    return Err(error(
                        &absolute,
                        "generated unit path differs from selected render target",
                    ));
                }
                let seed = super::super::normal::normal_seed(&entry.origin, &entry.path)
                    .ok_or_else(|| error(&absolute, "generated unit has no internal seed"))?;
                let key = key(&seed).ok_or_else(|| {
                    error(&absolute, "generated unit seed has no package identity")
                })?;
                if key.0 != group.as_str() || key.1 != *name {
                    return Err(error(
                        &absolute,
                        "generated unit seed differs from typed provider",
                    ));
                }
                let supplied = overlay.and_then(|values| values.get(&absolute));
                let (bytes, policy) = context.checked_generated_observation(
                    group.as_str(),
                    name,
                    &crate::path_to_slash(relative),
                    supplied.map(|v| v.as_ref()),
                    self_coord,
                )?;
                let digest = Sha256::digest(&bytes).into();
                let relative = crate::path_to_slash(relative);
                let input = GeneratedInput {
                    path: absolute,
                    group: group.to_string(),
                    name: name.clone(),
                    relative,
                    bytes: bytes.into(),
                    digest,
                    media: if render_format == SpecFormat::Xml {
                        "application/xml"
                    } else {
                        "text/markdown"
                    },
                    overlay: supplied.is_some(),
                    policy: Arc::new(policy),
                };
                if let Some(prior) = result.generated.insert(key, input.clone())
                    && (prior.path != input.path || prior.bytes != input.bytes)
                {
                    return Err(error(&input.path, "ambiguous generated initial seed"));
                }
            }
        }
        Ok(result)
    }
    fn generated(&self, address: &SpecAddress) -> Option<&GeneratedInput> {
        self.generated.get(&key(address)?)
    }
    fn bytes(&self, input: &GeneratedInput) -> Result<Vec<u8>, String> {
        let context = self
            .context
            .as_ref()
            .ok_or_else(|| "generated read lost selected context".to_string())?;
        input
            .policy
            .recheck_unchanged()
            .map_err(|e| e.to_string())?;
        let (bytes, _) = context
            .checked_generated_observation(
                &input.group,
                &input.name,
                &input.relative,
                input.overlay.then_some(input.bytes.as_ref()),
                &self.self_coord,
            )
            .map_err(|e| e.to_string())?;
        let digest: [u8; 32] = Sha256::digest(&bytes).into();
        if digest != input.digest || bytes.as_slice() != input.bytes.as_ref() {
            return Err("generated artifact changed after its typed input observation".into());
        }
        input
            .policy
            .recheck_unchanged()
            .map_err(|e| e.to_string())?;
        Ok(bytes)
    }
    fn checked_generated(&self, address: &SpecAddress) -> Result<Option<&GeneratedInput>, String> {
        let Some(input) = self.generated(address) else {
            return Ok(None);
        };
        if address.pinned_r.is_some() {
            return Err("generated initial artifact has no authored revision".into());
        }
        if let Authority::Package {
            version: Some(version),
            ..
        } = &address.authority
        {
            let selected = self
                .context
                .as_ref()
                .and_then(|context| {
                    context
                        .selected
                        .get(&(input.group.clone(), input.name.clone()))
                })
                .ok_or_else(|| "generated package lost selection".to_owned())?;
            if version.trim_start_matches('=') != selected.version {
                return Err("generated initial seed version differs from selection".into());
            }
        }
        Ok(Some(input))
    }
    fn projection(
        input: &GeneratedInput,
        bytes: &[u8],
    ) -> Result<(String, vibe_specdoc::SourceKind), String> {
        let text = std::str::from_utf8(bytes)
            .map_err(|error| format!("generated artifact is not UTF-8: {error}"))?;
        if input.media == "application/xml" {
            vibe_spec::project_emitted_xml_to_markdown(text)
                .map(|text| (text, vibe_specdoc::SourceKind::XmlProjected))
                .map_err(|error| error.to_string())
        } else {
            Ok((text.to_owned(), vibe_specdoc::SourceKind::Markdown))
        }
    }
    pub(crate) fn simple_generated(
        &self,
        entry: &BootEntry,
    ) -> Result<Option<String>, WorkspaceError> {
        let Some(input) = self.generated.values().find(|value| {
            value.path
                == self
                    .context
                    .as_ref()
                    .map_or_else(|| PathBuf::from(&entry.path), |c| c.root.join(&entry.path))
        }) else {
            return Ok(None);
        };
        let bytes = self.bytes(input).map_err(|e| error(&input.path, e))?;
        Self::projection(input, &bytes)
            .map(|(text, _)| Some(text))
            .map_err(|e| error(&input.path, e))
    }
}
fn key(address: &SpecAddress) -> Option<(String, String, String)> {
    let Authority::Package { group, name, .. } = &address.authority else {
        return None;
    };
    Some((group.clone(), name.clone(), address.doc_path.clone()))
}
impl SectionSource for BootSectionSource {
    fn admit_reference(&self, address: &SpecAddress) -> Result<(), String> {
        if self.generated(address).is_some() {
            return Err("generated unit artifact is an internal initial seed, not an authored reference target".into());
        }
        self.ordinary.admit_reference(address)
    }
    fn section_text(&self, address: &SpecAddress) -> Result<String, String> {
        self.admit_reference(address)?;
        self.ordinary.section_text(address)
    }
    fn source_metadata(
        &self,
        address: &SpecAddress,
        formats: &[String],
    ) -> Result<ResolvedSourceMetadata, String> {
        self.admit_reference(address)?;
        self.ordinary.source_metadata(address, formats)
    }
    fn read_resolved_source(
        &self,
        address: &SpecAddress,
        metadata: ResolvedSourceMetadata,
    ) -> Result<ResolvedSource, String> {
        self.admit_reference(address)?;
        self.ordinary.read_resolved_source(address, metadata)
    }
    fn root_source_metadata(
        &self,
        address: &SpecAddress,
        formats: &[String],
    ) -> Result<ResolvedSourceMetadata, String> {
        match self.checked_generated(address)? {
            Some(input) => {
                ResolvedSourceMetadata::new(&input.path, "markdown".into(), "STATIC".into())
            }
            None => self.ordinary.root_source_metadata(address, formats),
        }
    }
    fn read_resolved_root_source(
        &self,
        address: &SpecAddress,
        metadata: ResolvedSourceMetadata,
    ) -> Result<ResolvedSource, String> {
        if let Some(input) = self.checked_generated(address)? {
            if metadata.origin() != input.path
                || metadata.format() != "markdown"
                || metadata.physical_stem() != "STATIC"
            {
                return Err("generated root locator differs from typed artifact".into());
            }
            let bytes = self.bytes(input)?;
            // This is an artifact stream, not one authored XML document. The
            // stream owner preserves framing/literals; anchor offsets below
            // belong to its canonical Markdown projection, never raw bytes.
            let (projected, kind) = Self::projection(input, &bytes)?;
            return vibe_spec::resolve_captured_declared_source(
                address,
                metadata,
                projected.as_bytes(),
                "text/markdown",
                SourceInterpretation::VibeSpec,
            )
            .map_err(|error| match kind.projection_notice() {
                Some(notice) => format!("{error} {notice}"),
                None => error,
            });
        }
        self.ordinary.read_resolved_root_source(address, metadata)
    }
    fn expand_pattern(&self, address: &SpecAddress) -> Result<Vec<SpecAddress>, String> {
        self.ordinary.expand_pattern(address).map(|rows| {
            rows.into_iter()
                .filter(|row| self.generated(row).is_none())
                .collect()
        })
    }
}
