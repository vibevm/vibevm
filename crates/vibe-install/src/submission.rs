//! Submitted inputs enter one existing plan; private payload proofs authorize
//! exactly that plan's ordinary lock publication, never the generic writer.
use crate::{Error, Fetched, PlannedInstall, error::Result};
use specmark::spec;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use vibe_core::manifest::{Lockfile, Manifest, SubmittedInput};

#[derive(Debug)]
pub(crate) struct VerifiedSubmission {
    root: PathBuf,
    input: SubmittedInput,
    payloads: Vec<vibe_registry::local_preview::VerifiedPayload>,
    source_payload: vibe_registry::local_preview::VerifiedPayload,
    selected_rows: Vec<vibe_core::manifest::LockedPackage>,
    configurations: BTreeMap<String, vibe_core::manifest::PackageConfiguration>,
}
fn invalid(reason: impl Into<String>) -> Error {
    vibe_core::Error::InvalidLockfile {
        reason: reason.into(),
    }
    .into()
}

#[spec(implements = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail")]
pub(crate) fn check_root(root: &Path, manifest: &Manifest, input: &SubmittedInput) -> Result<()> {
    let project = vibe_safefs::Project::open(root).map_err(|e| invalid(e.to_string()))?;
    let captured = project
        .read_file_snapshot_bounded("vibe.toml", 64 * 1024)
        .map_err(|e| invalid(e.to_string()))?
        .ok_or_else(|| invalid("submitted root manifest missing"))?;
    let current = Manifest::parse_str(
        std::str::from_utf8(&captured.bytes).map_err(|_| invalid("submitted root is not UTF8"))?,
    )?;
    if current != *manifest
        || root
            .canonicalize()
            .map_err(|_| invalid("submitted root unavailable"))?
            != root
        || vibe_core::manifest::submitted_project_witness(root, &captured.bytes).map_err(invalid)?
            != input.project_witness
        || manifest.workspace.is_some()
        || manifest.requires.packages.len() != 1
        || manifest.requires.packages[0] != input.requirement
        || manifest
            .requires
            .optional
            .get(&input.requirement.qualified_name())
            .copied()
            .unwrap_or(false)
        || !manifest.requires.configurations.is_empty()
    {
        return Err(invalid(
            "submitted project or exact unanswered root declaration changed",
        ));
    }
    Ok(())
}
fn payloads(fetched: &[Fetched]) -> Result<Vec<vibe_registry::local_preview::VerifiedPayload>> {
    fetched
        .iter()
        .map(|f| {
            vibe_registry::local_preview::verify_pinned_payload(
                &f.cached.cache_dir,
                &f.cached.content_hash,
            )
            .map_err(|e| invalid(format!("{e:#}")))
        })
        .collect()
}
impl VerifiedSubmission {
    #[spec(implements = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail")]
    pub(crate) fn mint(planned: &mut PlannedInstall, input: SubmittedInput) -> Result<Self> {
        check_root(&planned.project_root, &planned.manifest, &input)?;
        let key = input.requirement.qualified_name();
        let f = planned
            .fetched
            .iter()
            .find(|f| format!("{}/{}", f.cached.resolved.group, f.cached.resolved.name) == key)
            .ok_or_else(|| invalid("submitted target not fetched"))?;
        if f.cached.registry_name.as_deref() != Some(input.expected_source.registry.as_str())
            || f.cached.content_hash != input.expected_source.content_hash.as_str()
            || !input
                .requirement
                .version
                .matches(&f.cached.resolved.version)
        {
            return Err(invalid(
                "submitted actual candidate differs from expected preview source",
            ));
        }
        let source_path = vibe_registry::localdir_path(&f.cached.source_uri)
            .map_err(|e| invalid(format!("{e:#}")))?;
        let source_payload = vibe_registry::local_preview::verify_pinned_payload(
            &source_path,
            input.expected_source.content_hash.as_str(),
        )
        .map_err(|e| invalid(format!("{e:#}")))?;
        let proofs = payloads(&planned.fetched)?;
        for f in &mut planned.fetched {
            if format!("{}/{}", f.cached.resolved.group, f.cached.resolved.name) == key {
                let record = f
                    .configuration
                    .as_mut()
                    .ok_or_else(|| invalid("submitted target configuration missing"))?;
                if record.consumers.len() != 1
                    || record.consumers.get("./vibe.toml") != Some(&input.original)
                {
                    return Err(invalid("submitted effective consumer differs"));
                }
                record.consumers.clear();
                record
                    .submitted_inputs
                    .insert("./vibe.toml".into(), input.clone());
                record.validate(&f.cached.manifest)?;
            }
        }
        for row in &mut planned.resolution {
            if format!("{}/{}", row.group, row.name) == key {
                row.configuration = planned
                    .fetched
                    .iter()
                    .find(|f| {
                        f.cached.resolved.group == row.group && f.cached.resolved.name == row.name
                    })
                    .ok_or_else(|| invalid("submitted target not fetched"))?
                    .configuration
                    .clone();
            }
        }
        let language = planned
            .language_chain
            .first()
            .map(String::as_str)
            .filter(|language| *language != "en");
        let selected_rows = planned
            .fetched
            .iter()
            .map(|f| {
                let resolved = planned
                    .resolution
                    .iter()
                    .find(|row| {
                        row.group == f.cached.resolved.group && row.name == f.cached.resolved.name
                    })
                    .ok_or_else(|| invalid("fetched package has no resolved plan row"))?;
                Ok(crate::record::locked_package_from_fetched(
                    f, resolved, language,
                ))
            })
            .collect::<Result<_>>()?;
        let configurations = planned
            .fetched
            .iter()
            .filter_map(|f| {
                f.configuration.clone().map(|record| {
                    (
                        format!("{}/{}", f.cached.resolved.group, f.cached.resolved.name),
                        record,
                    )
                })
            })
            .collect();
        Ok(Self {
            root: planned.project_root.clone(),
            input,
            payloads: proofs,
            source_payload,
            selected_rows,
            configurations,
        })
    }
    pub(crate) fn recheck(&self, manifest: &Manifest) -> Result<()> {
        check_root(&self.root, manifest, &self.input)?;
        self.source_payload
            .recheck()
            .map_err(|e| invalid(format!("{e:#}")))?;
        for proof in &self.payloads {
            proof.recheck().map_err(|e| invalid(format!("{e:#}")))?;
        }
        Ok(())
    }
    pub(crate) fn encode(self, lock: &mut Lockfile, previous: Option<&str>) -> Result<String> {
        let key = self.input.requirement.qualified_name();
        let record = lock
            .configurations
            .get(&key)
            .ok_or_else(|| invalid("submitted publication lost record"))?;
        if lock.packages != self.selected_rows
            || lock.configurations != self.configurations
            || lock.meta.root_dependencies != vec![self.input.requirement.clone()]
            || record.submitted_inputs
                != BTreeMap::from([("./vibe.toml".into(), self.input.clone())])
        {
            return Err(invalid("submitted publication differs from verified plan"));
        }
        lock.meta.schema_version = if self.input.original.profile.is_some()
            && vibe_core::manifest::integer_profile_shape(&record.definition)
        {
            20
        } else {
            16
        };
        lock.validate_configurations()?;
        let text = toml::to_string_pretty(lock).map_err(|e| invalid(e.to_string()))?;
        Ok(vibe_core::manifest::decorate_toml(&text, previous))
    }
}

/// Source-free replay verifies every actual full store payload, never a sidecar.
#[spec(implements = "spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail")]
pub fn verify_submitted_replay(root: &Path, manifest: &Manifest, lock: &Lockfile) -> Result<()> {
    if !lock
        .configurations
        .values()
        .any(|r| !r.submitted_inputs.is_empty())
    {
        return Ok(());
    }
    lock.validate_configurations()?;
    let store = vibe_registry::store::store_root()?;
    for record in lock.configurations.values() {
        for input in record.submitted_inputs.values() {
            check_root(root, manifest, input)?;
        }
    }
    for row in &lock.packages {
        let path =
            vibe_registry::store::entry_dir(&store, &row.group, row.name.as_str(), &row.version);
        vibe_registry::local_preview::verify_pinned_payload(&path, row.content_hash.as_str())
            .map_err(|e| invalid(format!("{e:#}")))?;
    }
    Ok(())
}
