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
    /// Observe one authored input of a prepared node under the held
    /// workspace — the system-prompt template and the selected entrypoint,
    /// neither of which is a boot source nor a declared source row, so
    /// neither reaches this crate through any existing reader.
    ///
    /// `relative` is node-relative and forward-slashed. `None` means the
    /// file is absent, which is an answer rather than a failure: the
    /// template's absence is how a node declines the whole feature
    /// (PROP-076 `#SYS-OPT-IN`).
    ///
    /// The node's own guard is rechecked **before and after** the read, by
    /// the same before/after reasoning generated-input admission already
    /// uses: a body that changed while it was being observed must be
    /// refused, never witnessed. Nothing outside the node is reachable —
    /// the read is held under the node's own directory capability, so a
    /// path that climbs out of it cannot resolve
    /// (PROP-076 `#SYS-CAPTURE-REUSE`).
    pub(crate) fn node_input(
        &self,
        node_rel: &str,
        relative: &str,
    ) -> Result<Option<Vec<u8>>, WorkspaceError> {
        // A node that was never prepared is a hard error, not an absence:
        // reading an unwitnessed node would produce evidence nothing backs.
        self.node(node_rel)?;
        let guards = self.node_guards(node_rel);
        for guard in &guards {
            guard.recheck()?;
        }
        let components: Vec<&str> = if node_rel == "." {
            Vec::new()
        } else {
            node_rel.split('/').collect()
        };
        let directory = self
            .project
            .dir(&components, false)
            .map_err(|e| error(&self.root.join(node_rel), e))?;
        let bytes = self
            .project
            .read_source_bytes_in(&directory, relative)
            .map_err(|e| error(&self.root.join(node_rel).join(relative), e))?;
        for guard in &guards {
            guard.recheck()?;
        }
        Ok(bytes)
    }

    /// Observe one **workspace-root-relative** file under the held
    /// workspace — the spelling a generated `INDEX.md` row uses, and
    /// therefore the only spelling in which the continuation plan can name a
    /// dependency slot's boot file (`PROP-076#SYS-BANDS`, and the plan-path
    /// deviation recorded in `#SYS-SIDECAR`).
    ///
    /// It is the root node's own [`node_input`](Self::node_input) and adds
    /// no reader of its own: the same prepared guard, rechecked before and
    /// after, and the same no-follow capability under the workspace root.
    /// The agent lane's band-U bodies are read through it, which is why a
    /// lane body can only ever be a file the boot plan itself named
    /// (`#SYS-NO-HOME-EXPANSION`).
    pub(crate) fn root_input(&self, relative: &str) -> Result<Option<Vec<u8>>, WorkspaceError> {
        self.node_input(".", relative)
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
