//! Authoritative builtin proof over a C2 verified original capture.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-070#BUILTIN-DERIVATION-PROOF");

use std::path::Path;

use vibe_core::layout;
use vibe_core::manifest::{Manifest, SpecFormat};
use vibe_core::{ContentHash, machine_json_path};
use vibe_facts::{PackageOverlay, RegistrySnapshot};
use vibe_workspace::source_query::{SourceQueryUnavailable, WitnessedSourceWorld};
use vibe_workspace::vibedeps::{CONVERTER_RECIPE, SlotFileDisposition, replay_declared_captured};

use super::VerifiedSourceMembership;

/// Package overlay and its raw-byte witness from ONE validated registry load.
/// This is prepared consumer context, never selected by an editable slot row.
pub struct PreparedPackageOverlay {
    package: String,
    value: PackageOverlay,
    raw_hash: Option<ContentHash>,
}

impl PreparedPackageOverlay {
    pub fn from_registry_snapshot(
        snapshot: &RegistrySnapshot,
        group: &str,
        name: &str,
    ) -> Result<Self, SourceQueryUnavailable> {
        let package = format!("{group}/{name}");
        let relative = machine_json_path(
            &layout::current_vibefacts_root().join(format!("{}.toml", package.replace('/', "."))),
        );
        let mut matches = snapshot.witnesses.iter().filter(|row| row.path == relative);
        let raw_hash = matches
            .next()
            .map(|row| {
                ContentHash::parse(&row.digest)
                    .map_err(|_| SourceQueryUnavailable::DerivationUnavailable("overlay witness"))
            })
            .transpose()?;
        if matches.next().is_some() {
            return Err(SourceQueryUnavailable::DerivationUnavailable(
                "duplicate overlay witness",
            ));
        }
        let value = snapshot.registry.package_overlay(&package);
        if raw_hash.is_none() && !value.is_empty() {
            return Err(SourceQueryUnavailable::DerivationUnavailable(
                "overlay witness absent",
            ));
        }
        Ok(Self {
            package,
            value,
            raw_hash,
        })
    }
}

/// Distinct identities of one proved materialized representation. Neither
/// the package content hash nor a local slot digest substitutes for these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedMaterializationProof {
    source_path: String,
    output_path: String,
    original_raw_sha256: String,
    representation_sha256: String,
    converter_recipe: &'static str,
    overlay_hash: Option<ContentHash>,
}

impl DerivedMaterializationProof {
    pub fn source_path(&self) -> &str {
        &self.source_path
    }
    pub fn output_path(&self) -> &str {
        &self.output_path
    }
    pub fn original_raw_sha256(&self) -> &str {
        &self.original_raw_sha256
    }
    pub fn representation_sha256(&self) -> &str {
        &self.representation_sha256
    }
    pub fn converter_recipe(&self) -> &'static str {
        self.converter_recipe
    }
    pub fn overlay_hash(&self) -> Option<&ContentHash> {
        self.overlay_hash.as_ref()
    }
}

impl VerifiedSourceMembership {
    /// Replay the incumbent pure builtin converter on the verified ORIGINAL
    /// member, then compare its exact bytes to the retained selected output.
    /// No provider, materializer, filesystem converter or facts reload runs.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-070#BUILTIN-DERIVATION-PROOF"
    )]
    pub fn derive_builtin_output(
        &self,
        world: &WitnessedSourceWorld,
        group: &str,
        name: &str,
        output: &str,
        overlay: &PreparedPackageOverlay,
    ) -> Result<DerivedMaterializationProof, SourceQueryUnavailable> {
        let (pin, selected_hash) = world.selected_source_pin(group, name)?;
        if pin != self.package_pin || selected_hash != self.selected_hash {
            return Err(SourceQueryUnavailable::DerivationUnavailable(
                "selected original capture",
            ));
        }
        if overlay.package != format!("{group}/{name}") {
            return Err(SourceQueryUnavailable::DerivationUnavailable(
                "selected overlay package",
            ));
        }
        let recorded = world.recorded_output(group, name, output)?.ok_or(
            SourceQueryUnavailable::DerivationUnavailable("selected descriptor output"),
        )?;
        if recorded.slot_format == SpecFormat::Mixed
            || recorded.converter_recipe.as_deref() != Some(CONVERTER_RECIPE)
            || recorded.disposition != Some(SlotFileDisposition::Converted)
        {
            return Err(SourceQueryUnavailable::DerivationUnavailable(
                "builtin converter recipe or disposition",
            ));
        }
        if recorded.overlay_hash != overlay.raw_hash {
            return Err(SourceQueryUnavailable::DerivationUnavailable(
                "authorized overlay identity",
            ));
        }
        let manifest = self.index.snapshot(Manifest::FILENAME).ok_or(
            SourceQueryUnavailable::DerivationUnavailable("verified original manifest"),
        )?;
        let manifest = Manifest::parse_str(std::str::from_utf8(&manifest.bytes).map_err(|_| {
            SourceQueryUnavailable::DerivationUnavailable("verified original manifest")
        })?)
        .map_err(|_| SourceQueryUnavailable::DerivationUnavailable("verified original manifest"))?;
        let meta = manifest
            .require_package()
            .map_err(|_| SourceQueryUnavailable::DerivationUnavailable("source descriptor"))?;
        let original = self.index.snapshot(recorded.source_path).ok_or(
            SourceQueryUnavailable::DerivationUnavailable("verified source member"),
        )?;
        let replay = replay_declared_captured(
            meta,
            Path::new(recorded.source_path),
            recorded.slot_format,
            &overlay.value,
            &original.bytes,
        )
        .map_err(|_| SourceQueryUnavailable::DerivationUnavailable("builtin conversion"))?
        .ok_or(SourceQueryUnavailable::DerivationUnavailable(
            "selected conversion",
        ))?;
        if machine_json_path(&replay.0) != output || replay.1 != recorded.snapshot.bytes {
            return Err(SourceQueryUnavailable::DerivationUnavailable(
                "materialized output differs from builtin replay",
            ));
        }
        // This is finite detection, not a promise against a later ABA change.
        world.revalidate_current()?;
        let (after_pin, after_hash) = world.selected_source_pin(group, name)?;
        if after_pin != pin || after_hash != selected_hash {
            return Err(SourceQueryUnavailable::DerivationUnavailable(
                "selected world changed during replay",
            ));
        }
        Ok(DerivedMaterializationProof {
            source_path: recorded.source_path.to_owned(),
            output_path: output.to_owned(),
            original_raw_sha256: format!("sha256:{}", original.sha256),
            representation_sha256: format!("sha256:{}", recorded.snapshot.sha256),
            converter_recipe: CONVERTER_RECIPE,
            overlay_hash: overlay.raw_hash.clone(),
        })
    }
}
