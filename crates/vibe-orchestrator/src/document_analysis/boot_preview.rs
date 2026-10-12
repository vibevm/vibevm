//! Captured effective boot and builtin compilation, no publication.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#DOCUMENT-ANALYSIS-CAPTURE-LAW");
use super::*;
use vibe_core::manifest::Manifest;
use vibe_resolver::ResolvedGraph;
use vibe_specdoc::captured::{StructuredView, inspect_captured};
impl AnalysisCapture {
    pub(super) fn boot_preview(
        &self,
        primary: &Manifest,
        graph: &ResolvedGraph,
        result: &mut ConfigurationPreview,
    ) {
        use vibe_workspace::boot::{
            AuthoredBoot, DependencyBoot, NodeBootInputs, compute_effective_boot,
        };
        let mut dependencies = vec![];
        let installed: std::collections::HashSet<_> = graph
            .packages
            .iter()
            .map(|n| (n.group.clone(), n.name.clone()))
            .collect();
        let present_paths: std::collections::BTreeSet<_> = self
            .resources
            .values()
            .filter(|r| r.raw().is_some())
            .map(|r| r.path().to_owned())
            .collect();
        for node in &graph.packages {
            let found = self
                .resources
                .iter()
                .filter(|(_, r)| r.declaration.role == ResourceRole::Manifest)
                .find_map(|(id, r)| {
                    let m = self.manifest(*id).ok()?;
                    let p = m.package.as_ref()?;
                    (p.group == node.group && p.name == node.name && p.version == node.version)
                        .then_some((*id, r.path().to_owned(), m))
                });
            let Some((_id, path, manifest)) = found else {
                result.diagnostics.push(self.diagnostic(
                    self.primary,
                    "captured-resolution",
                    "selected manifest unavailable",
                ));
                return;
            };
            let package = manifest.package.as_ref().unwrap();
            let parent = Path::new(&path).parent().unwrap_or(Path::new(""));
            let snippet = manifest.boot_snippet.as_ref();
            let slot = vibe_core::machine_json_path(parent);
            let resolve = |source: &Path| {
                vibe_workspace::boot::resolve_snippet_source_with(&slot, source, |p| {
                    present_paths.contains(p)
                })
            };
            let main = snippet.and_then(|s| {
                vibe_workspace::boot::resolve_snippet_condition(s.when.as_ref(), &installed)
                    .map(|when| (resolve(&s.source), when))
            });
            let fragments = snippet
                .into_iter()
                .flat_map(|s| &s.fragments)
                .filter_map(|fragment| {
                    vibe_workspace::boot::resolve_snippet_condition(
                        fragment.when.as_ref(),
                        &installed,
                    )
                    .map(|when| vibe_workspace::boot::BootContribution {
                        path: resolve(&fragment.source),
                        when,
                    })
                })
                .collect();
            dependencies.push(DependencyBoot {
                kind: package.kind,
                group: package.group.clone(),
                name: package.name.clone(),
                boot_path: main.as_ref().map(|(path, _)| path.clone()),
                fragments,
                category: snippet.and_then(|s| s.category),
                declared_link: primary
                    .requires
                    .declared_link(&package.group, &package.name),
                suggested_link: snippet.and_then(|s| s.link),
                when: main.and_then(|(_, when)| when),
                requires: node
                    .dependencies
                    .iter()
                    .filter_map(|p| Some((p.group.clone()?, p.name.to_string())))
                    .collect(),
                format: package.format,
                unit_substituted: false,
            });
        }
        let primary_path = self.resources[&self.primary].path();
        let parent = Path::new(primary_path).parent().unwrap_or(Path::new(""));
        let node_rel = if parent.as_os_str().is_empty() {
            "."
        } else {
            parent.to_str().unwrap()
        };
        let present: Vec<_> = self
            .resources
            .values()
            .filter(|r| r.raw().is_some())
            .filter_map(|r| {
                Path::new(r.path())
                    .strip_prefix(parent)
                    .ok()
                    .map(Path::to_path_buf)
            })
            .collect();
        let own = match vibe_workspace::boot::authored_boot_from_inventory(
            &self.root.join(parent),
            node_rel,
            &present,
        ) {
            Ok(v) => v,
            Err(e) => {
                result.diagnostics.push(self.diagnostic(
                    self.primary,
                    "authored-boot",
                    e.to_string(),
                ));
                return;
            }
        };
        let mut inherited = Vec::<AuthoredBoot>::new();
        for (id, r) in &self.resources {
            if *id != self.primary && r.declaration.role == ResourceRole::Manifest {
                let ancestor = Path::new(r.path()).parent().unwrap_or(Path::new(""));
                if ancestor == parent || !parent.starts_with(ancestor) {
                    continue;
                }
                let relative = if ancestor.as_os_str().is_empty() {
                    "."
                } else {
                    ancestor.to_str().unwrap()
                };
                let files: Vec<_> = self
                    .resources
                    .values()
                    .filter(|r| r.raw().is_some())
                    .filter_map(|r| {
                        Path::new(r.path())
                            .strip_prefix(ancestor)
                            .ok()
                            .map(Path::to_path_buf)
                    })
                    .collect();
                match vibe_workspace::boot::authored_boot_from_inventory(
                    &self.root.join(ancestor),
                    relative,
                    &files,
                ) {
                    Ok(entries) => inherited.extend(entries.into_iter().filter(|b| {
                        b.category == Some(vibe_core::manifest::BootCategory::Foundation)
                    })),
                    Err(e) => {
                        result.diagnostics.push(self.diagnostic(
                            *id,
                            "inherited-boot",
                            e.to_string(),
                        ));
                        return;
                    }
                }
            }
        }
        inherited.sort_by(|a, b| {
            a.path
                .split('/')
                .count()
                .cmp(&b.path.split('/').count())
                .then(a.path.cmp(&b.path))
        });
        let effective = match compute_effective_boot(NodeBootInputs {
            own_boot: &own,
            inherited_foundation: &inherited,
            dependencies: &dependencies,
            default_link: primary.boot.default_link,
        }) {
            Ok(v) => v,
            Err(e) => {
                result.diagnostics.push(self.diagnostic(
                    self.primary,
                    "boot-composition",
                    e.to_string(),
                ));
                return;
            }
        };
        let mut docs = vec![];
        let mut by_path = BTreeMap::new();
        for (id, resource) in &self.resources {
            let ResourceRole::Source { manifest } = &resource.declaration.role else {
                continue;
            };
            let owner = self
                .resources
                .iter()
                .find(|(_, r)| r.path() == manifest)
                .map(|(id, _)| *id)
                .unwrap();
            let Ok(m) = self.manifest(owner) else {
                continue;
            };
            let Some(consumer) = m.consumer_node() else {
                continue;
            };
            let owner_parent = Path::new(manifest).parent().unwrap_or(Path::new(""));
            let inherited_owner = owner_parent != parent && parent.starts_with(owner_parent);
            if owner != self.primary
                && !inherited_owner
                && !m.package.as_ref().is_some_and(|p| {
                    graph
                        .packages
                        .iter()
                        .any(|n| n.group == p.group && n.name == p.name && n.version == p.version)
                })
            {
                continue;
            }
            let Some(raw) = resource.bytes.clone() else {
                continue;
            };
            let Ok(syntax) = self.syntax(*id) else {
                continue;
            };
            let parent = Path::new(manifest).parent().unwrap_or(Path::new(""));
            let relative = Path::new(resource.path()).strip_prefix(parent).unwrap();
            let logical = relative
                .strip_prefix(vibe_core::layout::current_specs_root())
                .unwrap_or(relative);
            let stem = vibe_spec::canonical_doc_path(&vibe_core::machine_json_path(logical));
            let mut address = match vibe_spec::SpecAddress::parse(&format!(
                "spec://{}/{stem}",
                consumer.coordinate()
            )) {
                Ok(a) => a,
                Err(_) => continue,
            };
            if owner != self.primary {
                if let (Some(p), vibe_spec::Authority::Package { version, .. }) =
                    (m.package.as_ref(), &mut address.authority)
                {
                    *version = Some(p.version.to_string());
                }
            }
            by_path.insert(
                resource.path().to_owned(),
                (address.clone(), raw.clone(), syntax),
            );
            docs.push(vibe_spec::CapturedSpecDocument {
                address,
                raw,
                syntax,
            });
        }
        let source = match vibe_spec::CapturedSectionSource::new(docs) {
            Ok(v) => v,
            Err(e) => {
                result
                    .diagnostics
                    .push(self.diagnostic(self.primary, "captured-source", e));
                return;
            }
        };
        let mut inputs = vec![];
        for entry in effective.static_entries() {
            let Some((address, raw, syntax)) = by_path.get(&entry.path) else {
                result.diagnostics.push(self.diagnostic(
                    self.primary,
                    "missing-boot-source",
                    format!("boot source not captured: {}", entry.path),
                ));
                return;
            };
            let input = if entry.format == vibe_core::manifest::PackageFormat::Normal {
                vibe_spec::ArtifactInput::normal(&entry.origin, &entry.path, address.clone())
            } else {
                let inspection = inspect_captured(raw, *syntax);
                let text = match inspection.view {
                    Some(StructuredView::Vibe { ir, .. }) => vibe_specdoc::to_markdown(&ir),
                    _ => match std::str::from_utf8(raw) {
                        Ok(v) => v.into(),
                        Err(_) => {
                            result.diagnostics.push(self.diagnostic(
                                self.primary,
                                "boot-encoding",
                                "boot source is not UTF8",
                            ));
                            return;
                        }
                    },
                };
                vibe_spec::ArtifactInput::simple(&entry.origin, &entry.path, text)
            };
            match input {
                Ok(v) => inputs.push(v),
                Err(e) => {
                    result.diagnostics.push(self.diagnostic(
                        self.primary,
                        "boot-input",
                        e.to_string(),
                    ));
                    return;
                }
            }
        }
        result.boot = Some(effective);
        let target = if primary.consumer_node().and_then(|n| n.spec_format)
            == Some(vibe_core::manifest::SpecFormat::Xml)
        {
            vibe_spec::ArtifactTarget::StaticXml
        } else {
            vibe_spec::ArtifactTarget::StaticMarkdown
        };
        let plans = match self.compiler_plans(primary, graph) {
            Ok(v) => v,
            Err(e) => {
                result.diagnostics.push(e);
                return;
            }
        };
        if inputs.is_empty() {
            return;
        }
        let plan = match vibe_spec::ArtifactPlan::static_lane(
            target.clone(),
            if target == vibe_spec::ArtifactTarget::StaticXml {
                "analysis/STATIC.xml"
            } else {
                "analysis/STATIC.md"
            },
            "vibevm/vibespecs",
            inputs,
        ) {
            Ok(v) => v,
            Err(e) => {
                result
                    .diagnostics
                    .push(self.diagnostic(self.primary, "boot-plan", e.to_string()));
                return;
            }
        };
        match vibe_spec::compile_artifact(plans.attach_to(plan), &source) {
            Ok(artifact) => result.compiled_boot = Some(artifact.into_bytes()),
            Err(e) => result.diagnostics.push(self.diagnostic(
                self.primary,
                "boot-compile",
                e.to_string(),
            )),
        }
    }
}
