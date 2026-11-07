//! Original discovery inputs remain separate from effective version-expanded manifests.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-HOST-BOOT");
use crate::{Workspace, WorkspaceError};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use vibe_core::manifest::Manifest;

/// Compare all authored wire fields. TOML tables are unordered; arrays remain ordered.
///
/// ```
/// let a = vibe_core::manifest::Manifest::parse_str("[project]\nname='host'\nversion='1.0.0'\n[requires.packages]\n'org.example/a'='1'\n'org.example/z'='1'\n").unwrap();
/// let mut b = a.clone();
/// b.requires.packages.reverse();
/// assert!(vibe_workspace::authored_manifests_equal(&a, &b).unwrap());
/// b.requires.packages[0].version = vibe_core::VersionSpec::parse("2").unwrap();
/// assert!(!vibe_workspace::authored_manifests_equal(&a, &b).unwrap());
/// ```
pub fn authored_manifests_equal(a: &Manifest, b: &Manifest) -> Result<bool, String> {
    let a = toml::Value::try_from(a).map_err(|e| e.to_string())?;
    let b = toml::Value::try_from(b).map_err(|e| e.to_string())?;
    Ok(a == b)
}
pub(crate) use authored_manifests_equal as same_manifest;
fn refused(root: &Path, reason: impl std::fmt::Display) -> WorkspaceError {
    WorkspaceError::SpecMaterialization {
        path: root.into(),
        reason: format!(
            "violates REQ spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-HOST-BOOT-LAW: {reason}; fix surface: retain the original authored node inputs across discovery and owned selected-manifest publication"
        ),
    }
}
impl Workspace {
    /// The original parsed node input, without reconstructing version variables.
    ///
    /// ```
    /// let root = tempfile::tempdir().unwrap();
    /// std::fs::write(root.path().join("vibe.toml"), "[project]\nname='host'\nversion='1.0.0'\n").unwrap();
    /// let workspace = vibe_workspace::Workspace::load(root.path()).unwrap();
    /// assert!(workspace.authored_node(".").is_some());
    /// ```
    pub fn authored_node(&self, relative: &str) -> Option<&Manifest> {
        self.authored_nodes.get(relative)
    }

    /// Advance only an explicitly supplied before/after selected-node delta.
    /// The caller updates effective semantics and owns guarded publication;
    /// this method reads and writes no filesystem state.
    ///
    /// ```
    /// let root = tempfile::tempdir().unwrap();
    /// std::fs::write(root.path().join("vibe.toml"), "[project]\nname='host'\nversion='1.0.0'\n").unwrap();
    /// let mut workspace = vibe_workspace::Workspace::load(root.path()).unwrap();
    /// let before = workspace.root_manifest.clone();
    /// let selected = workspace.root.clone();
    /// workspace.advance_authored_selected(&selected, &before, &before).unwrap();
    /// ```
    pub fn advance_authored_selected(
        &mut self,
        selected: &Path,
        before: &Manifest,
        after: &Manifest,
    ) -> Result<(), WorkspaceError> {
        let relative = self
            .node_rel_of(selected)
            .ok_or_else(|| refused(selected, "selected node is outside the retained workspace"))?
            .to_string();
        let retained = self
            .authored_nodes
            .get(&relative)
            .ok_or_else(|| refused(selected, "selected authored input is missing"))?;
        if !same_manifest(retained, before).map_err(|e| refused(selected, e))? {
            return Err(refused(
                selected,
                "selected delta does not start at the retained authored input",
            ));
        }
        let mut candidate = self.authored_nodes.clone();
        candidate.insert(relative, after.clone());
        self.validate_projection_from(&candidate)?;
        self.authored_nodes = candidate;
        Ok(())
    }

    /// Check the new override epoch against original inputs before publication.
    /// Only the selected node may differ, and only by the supplied known value.
    ///
    /// ```
    /// let root = tempfile::tempdir().unwrap();
    /// std::fs::write(root.path().join("vibe.toml"), "[project]\nname='host'\nversion='1.0.0'\n").unwrap();
    /// let before = vibe_workspace::Workspace::load(root.path()).unwrap();
    /// let after = vibe_workspace::Workspace::load(root.path()).unwrap();
    /// before.validate_authored_transition(&after, &before.root, &before.root_manifest).unwrap();
    /// ```
    pub fn validate_authored_transition(
        &self,
        after: &Self,
        selected: &Path,
        selected_after: &Manifest,
    ) -> Result<(), WorkspaceError> {
        self.validate_authored_projection()?;
        after.validate_authored_projection()?;
        if self.root != after.root || self.authored_nodes.keys().ne(after.authored_nodes.keys()) {
            return Err(refused(
                &self.root,
                "authored workspace node set changed after admission",
            ));
        }
        let relative = self
            .node_rel_of(selected)
            .ok_or_else(|| refused(selected, "selected node is outside the retained workspace"))?;
        for (key, original) in &self.authored_nodes {
            let expected = if key == relative.as_str() {
                selected_after
            } else {
                original
            };
            let observed = after
                .authored_nodes
                .get(key)
                .ok_or_else(|| refused(&self.root, "authored node disappeared"))?;
            if !same_manifest(expected, observed).map_err(|e| refused(&self.root, e))? {
                return Err(refused(
                    &self.node_abs_path(key),
                    "authored node changed across the selected override epoch",
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn validate_authored_projection(&self) -> Result<(), WorkspaceError> {
        self.validate_projection_from(&self.authored_nodes)
    }
    fn validate_projection_from(
        &self,
        authored: &BTreeMap<String, Manifest>,
    ) -> Result<(), WorkspaceError> {
        let effective_keys = self
            .iter_nodes()
            .map(|(key, _)| key)
            .collect::<BTreeSet<_>>();
        let authored_keys = authored.keys().map(String::as_str).collect::<BTreeSet<_>>();
        if effective_keys != authored_keys || authored.len() != self.members.len() + 1 {
            return Err(refused(
                &self.root,
                "effective and retained authored node sets differ",
            ));
        }
        let mut root = authored
            .get(".")
            .ok_or_else(|| refused(&self.root, "authored root is missing"))?
            .clone();
        let mut members = self.members.clone();
        for member in &mut members {
            member.manifest = authored
                .get(member.rel_path.as_str())
                .ok_or_else(|| refused(&self.root, "authored member is missing"))?
                .clone();
        }
        // Reuse the one pure expansion owner; never recover raw data from effective values.
        crate::expand::finalize_versions(&mut root, &mut members)?;
        if !same_manifest(&root, &self.root_manifest).map_err(|e| refused(&self.root, e))? {
            return Err(refused(
                &self.root,
                "effective root differs from retained authored projection",
            ));
        }
        for member in members {
            let effective = self
                .member_by_rel_path(member.rel_path.as_str())
                .ok_or_else(|| refused(&self.root, "effective member is missing"))?;
            if !same_manifest(&member.manifest, &effective.manifest)
                .map_err(|e| refused(&self.root, e))?
            {
                return Err(refused(
                    &self.member_abs_path(&member),
                    "effective member differs from retained authored projection",
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
