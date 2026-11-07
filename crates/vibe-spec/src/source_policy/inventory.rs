//! Finite declared original-host inventory, independent of publication payloads.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-HOST-INVENTORY",
    r = 1
);
use super::*;
use std::collections::{BTreeMap, BTreeSet};
use vibe_core::manifest::{SelectedSource, SourceSelector, select_source_rows};

/// Original bytes selected by the outer host policy; no nested manifest authority.
#[derive(Debug)]
pub struct CapturedHostResource {
    relative: String,
    observation: vibe_safefs::SourceObservation,
}
impl CapturedHostResource {
    pub fn relative(&self) -> &str {
        &self.relative
    }
    pub fn bytes(&self) -> &[u8] {
        self.observation.bytes()
    }
}
/// One finite selected observation, not a whole-repository census or a write token.
#[derive(Debug)]
pub struct CapturedHostInventory {
    resources: Vec<CapturedHostResource>,
    membership: Membership,
    coordinate: String,
    logical_root: PathBuf,
    root_identity: FileIdentity,
    manifest_bytes: Option<Vec<u8>>,
}
impl CapturedHostInventory {
    pub fn resources(&self) -> &[CapturedHostResource] {
        &self.resources
    }
}
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Membership {
    pub(super) files: BTreeSet<String>,
    pub(super) directories: BTreeMap<String, Option<FileIdentity>>,
}
fn git_control(path: &str) -> bool {
    path.split('/').any(|component| component == ".git")
}
impl PackagePolicyObservation {
    pub(super) fn inventory_error(&self, reason: impl Into<String>) -> SourcePolicyError {
        failure(
            &self.state.logical_root,
            SourcePolicyErrorKind::Unavailable,
            reason,
        )
    }
    fn inventory_admission(&self, coordinate: &str) -> Result<(), SourcePolicyError> {
        self.recheck_unchanged()?;
        if self.materialized_record_present()?
            || self
                .state
                .logical_root
                .ancestors()
                .any(crate::resolver::is_materialized_slot_root)
            || self
                .project
                .root_path()
                .ancestors()
                .any(crate::resolver::is_materialized_slot_root)
        {
            return Err(self.unsupported(
                "original host inventory of a materialized root without installed binding",
            ));
        }
        let actual = self
            .manifest()
            .and_then(|manifest| manifest.consumer_node())
            .and_then(|node| node.group.map(|group| format!("{group}/{}", node.name)));
        if actual.as_deref() != Some(coordinate) {
            return Err(self.unsupported("host inventory without matching self coordinate"));
        }
        let rows = self
            .source_rows()
            .ok_or_else(|| self.unsupported("declared host inventory without source policy"))?;
        for row in rows {
            if git_control(row.selector().path()) {
                return Err(
                    self.unsupported("explicit source scope inside reserved .git control subtree")
                );
            }
        }
        Ok(())
    }
    fn selected_inventory(&self, coordinate: &str) -> Result<Membership, SourcePolicyError> {
        self.inventory_admission(coordinate)?;
        let mut membership = Membership::default();
        for row in self
            .source_rows()
            .ok_or_else(|| self.unsupported("missing declared host policy"))?
        {
            match row.selector() {
                SourceSelector::File(path) => {
                    let (parent, name) = path
                        .rsplit_once('/')
                        .map_or(("", path.as_str()), |(parent, name)| (parent, name));
                    let directory = self
                        .inventory_directory(parent, &mut membership)?
                        .ok_or_else(|| {
                            self.inventory_error(format!("exact source parent is absent: {path}"))
                        })?;
                    if !self
                        .project
                        .source_file_present_in(&directory, name)
                        .map_err(|error| self.inventory_error(error.to_string()))?
                    {
                        return Err(self.inventory_error(format!(
                            "exact source is absent or not a regular file: {path}"
                        )));
                    }
                    self.inventory_select(coordinate, path, &mut membership)?;
                }
                SourceSelector::Directory(path) => {
                    if let Some(directory) = self.inventory_directory(path, &mut membership)? {
                        self.inventory_walk(coordinate, &directory, path, &mut membership)?;
                    }
                }
            }
        }
        let mut identities = BTreeSet::new();
        for path in &membership.files {
            if !identities.insert(vibe_safefs::path_identity_key(path)) {
                return Err(
                    self.inventory_error("selected host paths have a portable identity collision")
                );
            }
        }
        self.recheck_unchanged()?;
        Ok(membership)
    }
    fn inventory_select(
        &self,
        coordinate: &str,
        path: &str,
        membership: &mut Membership,
    ) -> Result<(), SourcePolicyError> {
        if vibe_core::layout::is_generated_boot_output(Path::new(path)) {
            return Ok(());
        }
        let rows = self
            .source_rows()
            .ok_or_else(|| self.unsupported("missing declared host policy"))?;
        match select_source_rows(coordinate, rows, path).map_err(|error| {
            self.inventory_error(format!("invalid selected source path: {error:?}"))
        })? {
            SelectedSource::Descriptor { .. } => {
                membership.files.insert(path.into());
            }
            SelectedSource::NeedsInput { .. } => {
                return Err(
                    self.inventory_error(format!("ambiguous host source interpretation: {path}"))
                );
            }
            SelectedSource::Unclassified { .. } => {}
        }
        Ok(())
    }
    pub(super) fn inventory_directory(
        &self,
        path: &str,
        membership: &mut Membership,
    ) -> Result<Option<Pinned>, SourcePolicyError> {
        let mut current = self
            .project
            .root_dir()
            .map_err(|error| self.inventory_error(error.to_string()))?;
        self.inventory_directory_id("", Some(&current), membership)?;
        let mut spelling = String::new();
        if !path.is_empty() {
            for component in path.split('/') {
                if !spelling.is_empty() {
                    spelling.push('/');
                }
                spelling.push_str(component);
                match current
                    .open_child_checked(component)
                    .map_err(|error| self.inventory_error(error.to_string()))?
                {
                    Some(next) => {
                        current = next;
                        self.inventory_directory_id(&spelling, Some(&current), membership)?;
                    }
                    None => {
                        self.inventory_directory_id(&spelling, None, membership)?;
                        return Ok(None);
                    }
                }
            }
        }
        Ok(Some(current))
    }
    fn inventory_directory_id(
        &self,
        path: &str,
        directory: Option<&Pinned>,
        membership: &mut Membership,
    ) -> Result<(), SourcePolicyError> {
        let identity = directory
            .map(Pinned::identity)
            .transpose()
            .map_err(|error| self.inventory_error(error.to_string()))?;
        if let Some(before) = membership.directories.insert(path.into(), identity)
            && before != identity
        {
            return Err(self.inventory_error("selected directory changed during inventory"));
        }
        Ok(())
    }
    fn inventory_walk(
        &self,
        coordinate: &str,
        directory: &Pinned,
        prefix: &str,
        membership: &mut Membership,
    ) -> Result<(), SourcePolicyError> {
        for name in self
            .project
            .child_names(directory)
            .map_err(|error| self.inventory_error(error.to_string()))?
        {
            if name == ".git" {
                continue;
            }
            vibe_core::manifest::declarant_path_component(&name)
                .map_err(|error| self.inventory_error(error.reason()))?;
            let relative = format!("{prefix}/{name}");
            if vibe_core::layout::is_generated_boot_output(Path::new(&relative)) {
                continue;
            }
            if self
                .project
                .source_file_present_in(directory, &name)
                .map_err(|error| self.inventory_error(error.to_string()))?
            {
                self.inventory_select(coordinate, &relative, membership)?;
            } else {
                let child = directory
                    .open_child_checked(&name)
                    .map_err(|error| self.inventory_error(error.to_string()))?
                    .ok_or_else(|| self.inventory_error("selected directory entry disappeared"))?;
                self.inventory_directory_id(&relative, Some(&child), membership)?;
                self.inventory_walk(coordinate, &child, &relative, membership)?;
            }
        }
        Ok(())
    }
    /// Observe only exact rows and declared directory scopes. Loading=explicit
    /// is included because this is a requested observation, not automatic boot.
    pub fn capture_host_inventory(
        &self,
        coordinate: &str,
    ) -> Result<CapturedHostInventory, SourcePolicyError> {
        let membership = self.selected_inventory(coordinate)?;
        let mut resources = Vec::with_capacity(membership.files.len());
        for path in &membership.files {
            let observation = self.read_source_observation(path)?.ok_or_else(|| {
                self.inventory_error(format!("selected source disappeared: {path}"))
            })?;
            resources.push(CapturedHostResource {
                relative: path.clone(),
                observation,
            });
        }
        let captured = CapturedHostInventory {
            resources,
            membership,
            coordinate: coordinate.into(),
            logical_root: self.state.logical_root.clone(),
            root_identity: self.state.identity,
            manifest_bytes: self.state.bytes.clone(),
        };
        self.recheck_host_inventory(&captured)?;
        Ok(captured)
    }
    /// Recheck selected membership/parents and each captured original byte buffer.
    pub fn recheck_host_inventory(
        &self,
        captured: &CapturedHostInventory,
    ) -> Result<(), SourcePolicyError> {
        if captured.logical_root != self.state.logical_root
            || captured.root_identity != self.state.identity
            || captured.manifest_bytes != self.state.bytes
        {
            return Err(
                self.inventory_error("host inventory belongs to another policy observation")
            );
        }
        if self.selected_inventory(&captured.coordinate)? != captured.membership {
            return Err(
                self.inventory_error("selected host membership or directory identity changed")
            );
        }
        for resource in &captured.resources {
            let current = self
                .read_source_observation(&resource.relative)?
                .ok_or_else(|| self.inventory_error("captured host source disappeared"))?;
            if current.identity() != resource.observation.identity()
                || current.bytes() != resource.bytes()
            {
                return Err(self.inventory_error("captured host source changed"));
            }
        }
        self.recheck_unchanged()
    }
}

#[cfg(test)]
mod tests;
