//! Source-backed level-zero input; generated pages never authorize this read.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-LEVEL0-READ",
    r = 1
);

use crate::citations::{SpecSources, sources::CapturedPages};
use crate::error::{DocError, Result};
use crate::pages::{Page, PageSet, SPEC_ROOT, UnreadablePage};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use vibe_core::manifest::{SelectedSource, SourceInterpretation, select_source_rows};
use vibe_spec::source_policy::{
    CapturedHostInventory, PackagePolicyObservation, PackageSourceMode, observe_package_policy,
};

#[derive(Debug)]
pub(crate) struct SourcePages {
    pub(crate) set: PageSet,
    pub(crate) copies: Vec<(String, Vec<u8>)>,
    root: PathBuf,
    policy: PackagePolicyObservation,
    spec: Option<CapturedHostInventory>,
    doc: Option<CapturedPages>,
    legacy: Vec<(String, vibe_safefs::SourceObservation)>,
    absent_doc: Option<String>,
}

fn refused(root: &Path, reason: impl std::fmt::Display) -> DocError {
    DocError::Page {
        path: root.into(),
        message: format!(
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-LEVEL0-READ: {reason}; fix surface: selected native source policy and original root binding"
        ),
    }
}

impl SourcePages {
    pub(crate) fn capture(root: &Path, manifest: &str) -> Result<Self> {
        let policy = observe_package_policy(root)?;
        if policy.manifest_bytes() != Some(manifest.as_bytes()) {
            return Err(refused(
                root,
                "card and admitted source manifest observations differ",
            ));
        }
        // This caller has an explicit original root, not an independently
        // selected installed identity/hash/format. Do not manufacture one.
        if policy.materialized_record_present()? {
            return Err(refused(
                root,
                "materialized source requires independent installed binding",
            ));
        }
        let mut captured = Self {
            set: PageSet::default(),
            copies: Vec::new(),
            root: root.into(),
            policy,
            spec: None,
            doc: None,
            legacy: Vec::new(),
            absent_doc: None,
        };
        match captured.policy.mode() {
            PackageSourceMode::Explicit => captured.declared()?,
            PackageSourceMode::ExplicitEmpty => {}
            PackageSourceMode::Legacy | PackageSourceMode::MissingManifest => captured.legacy()?,
        }
        captured.recheck()?;
        Ok(captured)
    }

    fn declared(&mut self) -> Result<()> {
        let node = self
            .policy
            .manifest()
            .and_then(|manifest| manifest.consumer_node())
            .ok_or_else(|| refused(&self.root, "native source has no actual subject"))?;
        let coordinate = node
            .group
            .map(|group| format!("{group}/{}", node.name))
            .ok_or_else(|| refused(&self.root, "native source has no actual group"))?;
        let inventory = self
            .policy
            .capture_native_inventory(&coordinate, SourceInterpretation::VibeSpec)?;
        for resource in inventory.resources() {
            let SelectedSource::Descriptor { descriptor, .. } = select_source_rows(
                &coordinate,
                self.policy.source_rows().unwrap_or_default(),
                resource.relative(),
            )
            .map_err(|error| refused(&self.root, format!("{error:?}")))?
            else {
                return Err(refused(
                    &self.root,
                    "captured native descriptor disappeared",
                ));
            };
            let text = std::str::from_utf8(resource.bytes()).map_err(|error| error.to_string());
            let parsed = text.and_then(|text| match descriptor.media() {
                "application/xml" => vibe_specdoc::from_xml(text).map_err(|e| e.to_string()),
                "text/markdown" => vibe_specdoc::from_markdown(text).map_err(|e| e.to_string()),
                _ => Err("unsupported native Spec media".into()),
            });
            self.append(resource.relative(), resource.bytes(), parsed);
        }
        self.spec = Some(inventory);
        if self
            .policy
            .has_native_sources(&coordinate, SourceInterpretation::VibeDoc)?
        {
            let doc = SpecSources::new()
                .capture_pages(&self.root)
                .map_err(|error| refused(&self.root, error))?;
            self.set.pages.extend(doc.set().pages.iter().cloned());
            self.set
                .unreadable
                .extend(doc.set().unreadable.iter().cloned());
            for (relative, bytes) in doc.declared_bytes() {
                self.copies.push((
                    relative
                        .strip_prefix(&format!("{SPEC_ROOT}/"))
                        .unwrap_or(relative)
                        .into(),
                    bytes.into(),
                ));
            }
            self.doc = Some(doc);
        } else {
            self.absent_doc = Some(coordinate);
        }
        let mut identities = BTreeSet::new();
        let mut portable = BTreeSet::new();
        for (relative, _) in &self.copies {
            if !identities.insert(vibe_spec::canonical_doc_path(relative))
                || !portable.insert(vibe_safefs::path_identity_key(relative))
            {
                return Err(refused(
                    &self.root,
                    "selected native document identity is ambiguous across interpretations",
                ));
            }
        }
        Ok(())
    }

    fn append(
        &mut self,
        original: &str,
        bytes: &[u8],
        parsed: std::result::Result<vibe_specdoc::doc::SpecDoc, String>,
    ) {
        let rel = original
            .strip_prefix(&format!("{SPEC_ROOT}/"))
            .unwrap_or(original)
            .to_owned();
        let path = self.root.join(original);
        self.copies.push((rel.clone(), bytes.into()));
        match parsed {
            Ok(doc) => self.set.pages.push(Page { rel, path, doc }),
            Err(message) => self
                .set
                .unreadable
                .push(UnreadablePage { rel, path, message }),
        }
    }

    fn legacy(&mut self) -> Result<()> {
        for (relative, _) in crate::pages::walk_package(&self.root)? {
            let original = format!("{SPEC_ROOT}/{relative}");
            let observation = self
                .policy
                .read_source_observation(&original)?
                .ok_or_else(|| refused(&self.root, "legacy page disappeared"))?;
            let text = std::str::from_utf8(observation.bytes())
                .map_err(|error| refused(&self.root, error))?;
            // Preserve the accepted legacy vocabulary; declarations choose
            // their own vocabulary explicitly in the separate branch above.
            let parsed = vibe_specdoc::from_xml_with(text, vibe_specdoc::Vocabulary::Doc)
                .map_err(|e| e.to_string());
            self.append(&original, observation.bytes(), parsed);
            self.legacy.push((original, observation));
        }
        Ok(())
    }

    pub(crate) fn recheck(&self) -> Result<()> {
        self.policy.recheck_unchanged()?;
        if self.policy.materialized_record_present()? {
            return Err(refused(
                &self.root,
                "materialized source requires independent installed binding",
            ));
        }
        if let Some(inventory) = &self.spec {
            self.policy.recheck_host_inventory(inventory)?;
        }
        if let Some(doc) = &self.doc {
            doc.recheck().map_err(|e| refused(&self.root, e))?;
        }
        if let Some(coordinate) = &self.absent_doc
            && self
                .policy
                .has_native_sources(coordinate, SourceInterpretation::VibeDoc)?
        {
            return Err(refused(
                &self.root,
                "previously absent native Doc input appeared",
            ));
        }
        if matches!(
            self.policy.mode(),
            PackageSourceMode::Legacy | PackageSourceMode::MissingManifest
        ) {
            let current = crate::pages::walk_package(&self.root)?;
            let names: Vec<_> = current
                .iter()
                .map(|(rel, _)| format!("{SPEC_ROOT}/{rel}"))
                .collect();
            if names
                != self
                    .legacy
                    .iter()
                    .map(|(rel, _)| rel.clone())
                    .collect::<Vec<_>>()
            {
                return Err(refused(&self.root, "legacy page membership changed"));
            }
            for (relative, before) in &self.legacy {
                let now = self
                    .policy
                    .read_source_observation(relative)?
                    .ok_or_else(|| refused(&self.root, "legacy page disappeared"))?;
                if now.identity() != before.identity() || now.bytes() != before.bytes() {
                    return Err(refused(&self.root, "captured legacy page changed"));
                }
            }
        }
        self.policy.recheck_unchanged()?;
        Ok(())
    }
}
