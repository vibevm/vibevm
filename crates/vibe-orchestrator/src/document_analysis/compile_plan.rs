//! Existing pure registry collection and compiler lowering over captured facts.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#DOCUMENT-ANALYSIS-CAPTURE-LAW");
use super::*;
use vibe_core::{
    PackageName,
    manifest::{Lockfile, Manifest},
};
use vibe_extension_registry::{
    DependencyExtensionSource, DependencyProvider, DependencyProviderId, ExtensionWorld,
    HostExtensionSource, HostIdentity, HostProvider, collect_extensions,
};
impl AnalysisCapture {
    pub(super) fn compiler_plans(
        &self,
        primary: &Manifest,
        graph: &vibe_resolver::ResolvedGraph,
    ) -> Result<vibe_spec::CompilePlans, AnalysisDiagnostic> {
        let node = primary.consumer_node();
        let identity = match node.as_ref() {
            Some(node) => match &node.group {
                Some(group) => HostIdentity::coordinate(DependencyProviderId::new(
                    group.clone(),
                    PackageName::parse(&node.name).map_err(|e| {
                        self.diagnostic(self.primary, "compiler-owner", e.to_string())
                    })?,
                )),
                None => HostIdentity::ungrouped_project(&node.name),
            },
            None => HostIdentity::virtual_workspace(),
        };
        let parent = Path::new(self.resources[&self.primary].path())
            .parent()
            .unwrap_or(Path::new(""));
        let version = primary
            .package
            .as_ref()
            .map(|p| p.version.to_string())
            .or_else(|| primary.project.as_ref().map(|p| p.version.clone()))
            .unwrap_or_default();
        let host = HostExtensionSource {
            provider: HostProvider {
                identity,
                root: self.root.join(parent),
                version,
                kind: primary.package.as_ref().map(|p| p.kind),
                content_hash: None,
            },
            declarations: primary.extensions.clone(),
            controls: primary.extension_controls.clone(),
            mechanisms: primary.mechanism_decls.clone(),
        };
        let lock = self
            .resources
            .values()
            .filter(|r| r.declaration.role == ResourceRole::Lockfile)
            .filter_map(|r| std::str::from_utf8(r.raw()?).ok())
            .find_map(|text| toml::from_str::<Lockfile>(text).ok());
        let mut installed = vec![];
        let mut effective_stack = None;
        for selected in &graph.packages {
            let found = self
                .resources
                .iter()
                .filter(|(_, r)| r.declaration.role == ResourceRole::Manifest)
                .find_map(|(id, r)| {
                    let m = self.manifest(*id).ok()?;
                    let p = m.package.as_ref()?;
                    (p.group == selected.group
                        && p.name == selected.name
                        && p.version == selected.version)
                        .then_some((*id, r.path().to_owned(), m))
                });
            let Some((id, path, manifest)) = found else {
                return Err(self.diagnostic(
                    self.primary,
                    "compiler-world-unavailable",
                    "selected compiler provider manifest missing",
                ));
            };
            if manifest.extensions.is_empty() && manifest.mechanism_decls.is_empty() {
                continue;
            }
            let package = manifest.package.as_ref().unwrap();
            let locked=lock.as_ref().and_then(|l|l.packages.iter().find(|p|p.group==package.group&&p.name==package.name&&p.version==package.version)).ok_or_else(||self.diagnostic(id,"compiler-source-witness-unavailable","declaration has no matching captured selected lock pin; raw remains available"))?;
            let provider_id = DependencyProviderId::new(
                package.group.clone(),
                PackageName::parse(&package.name)
                    .map_err(|e| self.diagnostic(id, "compiler-owner", e.to_string()))?,
            );
            if package.kind == vibe_core::PackageKind::Stack {
                effective_stack = Some(provider_id.clone());
            }
            installed.push(DependencyExtensionSource {
                provider: DependencyProvider {
                    id: provider_id,
                    root: self
                        .root
                        .join(Path::new(&path).parent().unwrap_or(Path::new(""))),
                    version: package.version.to_string(),
                    kind: package.kind,
                    content_hash: locked.content_hash.clone(),
                },
                declarations: manifest.extensions.clone(),
                controls: manifest.extension_controls.clone(),
                mechanisms: manifest.mechanism_decls.clone(),
            });
        }
        let registry = collect_extensions(ExtensionWorld {
            installed,
            host,
            effective_stack,
        })
        .map_err(|e| self.diagnostic(self.primary, "compiler-collection", e.to_string()))?;
        let rows = registry.enabled_compile_rows();
        if rows
            .iter()
            .any(|r| r.declaration().handler.kind() != "builtin")
        {
            return Err(self.diagnostic(self.primary,"native-preparation-required","captured compiler configuration needs a native/effectful preparation; read analysis cannot invoke it"));
        }
        vibe_spec::lower_effective_compile_rows(&rows)
            .map_err(|e| self.diagnostic(self.primary, "compiler-lowering", e.to_string()))
    }
}
