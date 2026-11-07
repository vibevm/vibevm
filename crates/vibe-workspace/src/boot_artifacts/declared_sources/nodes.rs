//! Retained original node inputs beside installed source bindings.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-HOST-BOOT",
    r = 1
);
use super::*;
impl DeclaredBootSources {
    pub(crate) fn prepare_with_nodes(
        workspace: &crate::Workspace,
        resolution: &[ResolvedDep],
        input_format: Option<SpecFormat>,
    ) -> Result<Option<Self>, WorkspaceError> {
        workspace.validate_authored_projection()?;
        let mut nodes = BTreeMap::new();
        for (relative, _) in workspace.iter_nodes() {
            let manifest = workspace
                .authored_node(relative)
                .ok_or_else(|| error(&workspace.root, "retained authored node input missing"))?;
            nodes.insert(
                relative.to_owned(),
                Arc::new(crate::install::bootgen::PreparedNodeBoot::prepare(
                    &workspace.node_abs_path(relative),
                    relative,
                    manifest,
                )?),
            );
        }
        let mut sources = Self::prepare_inner(&workspace.root, resolution, input_format, true)?;
        if let Some(sources) = sources.as_mut() {
            sources.nodes = Arc::new(nodes);
        }
        Ok(sources)
    }
    pub(crate) fn node(
        &self,
        relative: &str,
    ) -> Result<&crate::install::bootgen::PreparedNodeBoot, WorkspaceError> {
        self.nodes
            .get(relative)
            .map(Arc::as_ref)
            .ok_or_else(|| error(&self.root, "node was not prepared"))
    }
    pub(crate) fn node_guards(
        &self,
        relative: &str,
    ) -> Vec<Arc<crate::install::bootgen::PreparedNodeBoot>> {
        let mut guards = self
            .nodes
            .get(relative)
            .cloned()
            .into_iter()
            .collect::<Vec<_>>();
        if relative != "."
            && let Some(root) = self.nodes.get(".")
        {
            guards.push(root.clone());
        }
        guards
    }
    pub(crate) fn recheck_node(&self, node_dir: &Path) -> Result<(), WorkspaceError> {
        let relative = node_dir
            .strip_prefix(&self.root)
            .map_err(|e| error(node_dir, e))?;
        let relative = if relative.as_os_str().is_empty() {
            ".".into()
        } else {
            crate::path_to_slash(relative)
        };
        for node in self.node_guards(&relative) {
            node.recheck()?;
        }
        Ok(())
    }
}
