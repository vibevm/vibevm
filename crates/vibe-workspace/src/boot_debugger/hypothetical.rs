//! Explicit computation over retained inputs. No filesystem or provider port.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#HYPOTHETICAL-AUTHORITY");

use super::{measurement, protocol};
use crate::{
    boot::{self, BootContribution, DependencyBoot},
    install::bootgen::captured,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
    sync::Mutex,
};
use vibe_core::{
    Group,
    manifest::{LinkType, Manifest, PackageFormat},
};
use vibe_spec::{ArtifactInput, ArtifactPlan, ArtifactTarget, DocumentProvider};

pub fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

/// A complete, finite host inventory supplied by the admitted capture owner.
/// Paths are original host-relative resource identities, never paths to open.
#[derive(Clone)]
pub struct HostCapture {
    pub id: String,
    pub manifest: Manifest,
    pub files: BTreeMap<String, Vec<u8>>,
}

/// Rows are supplied in the real resolver's selected order, with its exact
/// effective edges and manifests. The package-source composition owns binding.
#[derive(Clone)]
pub struct SelectedPackage {
    pub manifest: Manifest,
    pub requires: Vec<(Group, String)>,
    pub files: BTreeMap<String, Vec<u8>>,
    pub source_identity: String,
}

#[derive(Clone)]
pub struct FrozenWorld {
    host: HostCapture,
    packages: Vec<SelectedPackage>,
    catalog: String,
    identity: String,
    missing: Vec<String>,
    resolution_evidence: String,
}
impl FrozenWorld {
    /// Bind source-owner supplied resolution evidence once. This constructor
    /// does not discover, solve or compile; it cannot authorize live reads.
    pub fn from_resolution(
        host: HostCapture,
        packages: Vec<SelectedPackage>,
        catalog: String,
        request_and_choices: String,
        mut missing: Vec<String>,
    ) -> Result<Self, String> {
        let mut membership = HashSet::new();
        for row in &packages {
            let p = row.manifest.require_package().map_err(|e| e.to_string())?;
            if !membership.insert((p.group.clone(), p.name.to_string())) {
                return Err(format!(
                    "duplicate selected package membership: {}/{}",
                    p.group, p.name
                ));
            }
        }
        for row in &packages {
            for edge in &row.requires {
                if !membership.contains(edge) {
                    missing.push(format!(
                        "transitive selected source capture: {}/{}",
                        edge.0, edge.1
                    ));
                }
            }
        }
        for root in &host.manifest.requires.packages {
            if root
                .group
                .as_ref()
                .is_none_or(|g| !membership.contains(&(g.clone(), root.name.to_string())))
            {
                missing.push(format!("selected root source capture: {root}"));
            }
        }
        let rows: Vec<_> = packages
            .iter()
            .map(|p| (&p.manifest, &p.requires, &p.files, &p.source_identity))
            .collect();
        let encoded = serde_json::to_vec(&(
            "hypothetical-world/1",
            &host.id,
            &host.manifest,
            &host.files,
            rows,
            &catalog,
            &request_and_choices,
            &missing,
        ))
        .map_err(|e| e.to_string())?;
        Ok(Self {
            host,
            packages,
            catalog,
            identity: digest(&encoded),
            missing,
            resolution_evidence: request_and_choices,
        })
    }
    pub fn identity(&self) -> &str {
        &self.identity
    }
    pub fn catalog(&self) -> &str {
        &self.catalog
    }
    pub fn capture(&self) -> &str {
        &self.host.id
    }
}

pub struct PreparedScenario {
    baseline: FrozenWorld,
    candidate: FrozenWorld,
    target: String,
    identity: String,
}
impl PreparedScenario {
    pub fn baseline(&self) -> &FrozenWorld {
        &self.baseline
    }
    pub fn candidate(&self) -> &FrozenWorld {
        &self.candidate
    }
    pub fn target(&self) -> &str {
        &self.target
    }
    pub fn new(
        baseline: FrozenWorld,
        candidate: FrozenWorld,
        target: String,
    ) -> Result<Self, String> {
        if !matches!(
            target
                .parse::<vibe_core::manifest::WhenCondition>()
                .map_err(|e| e.to_string())?,
            vibe_core::manifest::WhenCondition::Os(_)
        ) {
            return Err("target conditions require a captured os target".into());
        }
        let identity = digest(
            &serde_json::to_vec(&(
                "hypothetical-scenario/1",
                baseline.identity(),
                candidate.identity(),
                &target,
            ))
            .map_err(|e| e.to_string())?,
        );
        Ok(Self {
            baseline,
            candidate,
            target,
            identity,
        })
    }
    pub fn identity(&self) -> &str {
        &self.identity
    }
    pub fn evaluate(
        &self,
        expected: &str,
        baseline: &str,
        candidate: &str,
        catalog: &str,
        overrides: &BTreeMap<String, String>,
    ) -> Result<Preview, String> {
        if expected != self.identity
            || baseline != self.baseline.identity()
            || candidate != self.candidate.identity()
            || catalog != self.candidate.catalog()
        {
            return Err("stale hypothetical world/catalog/source capture".into());
        }
        let baseline = compile_world(&self.baseline, &BTreeMap::new())?;
        let mut candidate = compile_world(&self.candidate, overrides)?;
        let comparison = comparison::compare(self, &baseline, &candidate)?;
        if baseline.status != "complete" {
            candidate.status = "incomplete".into();
            candidate
                .missing
                .extend(baseline.missing.iter().map(|m| format!("baseline: {m}")));
        }
        candidate.comparison = Some(comparison);
        Ok(candidate)
    }
}

#[derive(Debug, Serialize)]
pub struct Placement {
    pub origin: String,
    pub path: String,
    pub loading: String,
    pub when: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct Preview {
    pub status: String,
    pub missing: Vec<String>,
    pub placements: Vec<Placement>,
    pub artifact: Option<String>,
    pub carrier: Option<vibe_wire::generated::compiler_ir::e1::ir::Ir>,
    pub measurement: Option<measurement::Lane>,
    pub comparison: Option<comparison::Comparison>,
}
fn incomplete(missing: Vec<String>) -> Preview {
    Preview {
        status: "incomplete".into(),
        missing,
        placements: vec![],
        artifact: None,
        carrier: None,
        measurement: None,
        comparison: None,
    }
}
#[derive(Default)]
struct Carrier(Mutex<Option<Vec<u8>>>);
impl vibe_spec::CompileTraceSink for Carrier {
    fn record(&self, event: &vibe_spec::PassTraceEvent<'_>) {
        if event.pass() == "assemble" {
            *self.0.lock().expect("private trace") = event.snapshot().map(Vec::from);
        }
    }
}
fn compile_world(
    world: &FrozenWorld,
    overrides: &BTreeMap<String, String>,
) -> Result<Preview, String> {
    if !world.missing.is_empty() {
        return Ok(incomplete(world.missing.clone()));
    }
    let installed: HashSet<_> = world
        .packages
        .iter()
        .filter_map(|p| p.manifest.package.as_ref())
        .map(|p| (p.group.clone(), p.name.clone()))
        .collect();
    let mut files = world.host.files.clone();
    let mut dependencies = vec![];
    let mut documents = vec![];
    let mut addresses = BTreeMap::new();
    let mut missing = vec![];
    if world
        .host
        .manifest
        .requires
        .links
        .values()
        .any(|l| *l == LinkType::StaticTransitive)
    {
        missing.push("captured static-transitive unit world: host".into());
    }
    for row in &world.packages {
        let p = row.manifest.require_package().map_err(|e| e.to_string())?;
        let coordinate = format!("{}/{}", p.group, p.name);
        if !row.manifest.extensions.is_empty() || !row.manifest.mechanism_decls.is_empty() {
            missing.push(format!("{}: {coordinate}", owner_stage(&row.manifest)));
        }
        if !p.sources.is_empty() || p.bridge {
            missing.push(format!("captured descriptor derivation: {coordinate}"));
        }
        let slot = package_slot(p);
        for (path, bytes) in &row.files {
            files.insert(format!("{slot}/{path}"), bytes.clone());
        }
        if p.format == PackageFormat::Normal && p.sources.is_empty() {
            for (path, bytes) in &row.files {
                let Ok(relative) =
                    Path::new(path).strip_prefix(vibe_core::layout::current_specs_root())
                else {
                    continue;
                };
                let syntax = match relative.extension().and_then(|e| e.to_str()) {
                    Some("md") => vibe_specdoc::captured::CapturedSyntax::VibeMarkdown,
                    Some("xml") => vibe_specdoc::captured::CapturedSyntax::VibeXml(
                        vibe_specdoc::Vocabulary::Spec,
                    ),
                    _ => continue,
                };
                let stem =
                    vibe_spec::canonical_doc_path(&relative.to_string_lossy().replace('\\', "/"));
                let address = vibe_spec::SpecAddress::parse(&format!("spec://{coordinate}/{stem}"))
                    .map_err(|e| e.to_string())?;
                addresses.insert(format!("{slot}/{path}"), address.clone());
                documents.push(vibe_spec::CapturedSpecDocument {
                    address: vibe_spec::SpecAddress::parse(&format!(
                        "spec://{coordinate}@{}/{stem}",
                        p.version
                    ))
                    .map_err(|e| e.to_string())?,
                    raw: bytes.clone().into(),
                    syntax,
                });
            }
        }
        let snippet = row.manifest.boot_snippet.as_ref();
        let contribution = |path: &Path, when: Option<&vibe_core::manifest::WhenCondition>| {
            captured::resolve_snippet_condition(when, &installed).map(|when| BootContribution {
                path: captured::resolve_snippet_source_with(&slot, path, |p| files.contains_key(p)),
                when,
            })
        };
        let main = snippet.and_then(|s| contribution(&s.source, s.when.as_ref()));
        let fragments = snippet
            .into_iter()
            .flat_map(|s| &s.fragments)
            .filter_map(|f| contribution(&f.source, f.when.as_ref()))
            .collect();
        dependencies.push(DependencyBoot {
            kind: p.kind,
            group: p.group.clone(),
            name: p.name.clone(),
            boot_path: main.as_ref().map(|m| m.path.clone()),
            when: main.and_then(|m| m.when),
            fragments,
            category: snippet.and_then(|s| s.category),
            declared_link: world
                .host
                .manifest
                .requires
                .declared_link(&p.group, &p.name),
            suggested_link: snippet.and_then(|s| s.link),
            requires: row.requires.clone(),
            format: p.format,
            unit_substituted: false,
        });
        // Hybrid unit substitution/hoisting requires its own captured unit world.
        if row.manifest.requires.links.values().any(|l| {
            matches!(
                l,
                LinkType::Static | LinkType::StaticHard | LinkType::StaticTransitive
            )
        }) || (!row.requires.is_empty()
            && row
                .manifest
                .boot
                .default_link
                .is_some_and(|l| l != LinkType::Dynamic))
        {
            missing.push(format!("captured hybrid unit world: {coordinate}"));
        }
    }
    if !world.host.manifest.extensions.is_empty() || !world.host.manifest.mechanism_decls.is_empty()
    {
        missing.push(format!("{}: host", owner_stage(&world.host.manifest)));
    }
    if !missing.is_empty() {
        return Ok(incomplete(missing));
    }
    let inventory: Vec<PathBuf> = world.host.files.keys().map(PathBuf::from).collect();
    let own = captured::authored_boot_from_inventory(Path::new("."), ".", &inventory)
        .map_err(|e| e.to_string())?;
    let effective = boot::compute_effective_boot(boot::NodeBootInputs {
        own_boot: &own,
        inherited_foundation: &[],
        dependencies: &dependencies,
        default_link: world.host.manifest.boot.default_link,
    })
    .map_err(|e| e.to_string())?;
    for (origin, loading) in overrides {
        let rows: Vec<_> = effective
            .entries
            .iter()
            .filter(|e| &e.origin == origin)
            .collect();
        if rows.is_empty() || rows.iter().any(|e| loading_name(e.link) != *loading) {
            return Err(format!(
                "incompatible loading override for {origin}: {loading}; prepare lawful consumer intent instead"
            ));
        }
    }
    let placements = effective
        .entries
        .iter()
        .map(|e| Placement {
            origin: e.origin.clone(),
            path: e.path.clone(),
            loading: loading_name(e.link).into(),
            when: e.when.as_ref().map(ToString::to_string),
        })
        .collect();
    for entry in &effective.entries {
        if !files.contains_key(&entry.path) {
            return Ok(incomplete(vec![format!("source capture: {}", entry.path)]));
        }
    }
    let mut inputs = vec![];
    for entry in effective.static_entries() {
        let Some(bytes) = files.get(&entry.path) else {
            return Ok(incomplete(vec![format!("source capture: {}", entry.path)]));
        };
        let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
        let (markdown, _) = vibe_specdoc::project_spec_text(Path::new(&entry.path), text)
            .map_err(|e| e.to_string())?;
        let crate::boot::BootProvenance::Dependency { group, name } = &entry.provenance else {
            return Ok(incomplete(vec![format!(
                "host static provider: {}",
                entry.path
            )]));
        };
        let provider = DocumentProvider::Dependency {
            group: group.clone(),
            name: vibe_core::PackageName::parse(name).map_err(|e| e.to_string())?,
        };
        let input = if entry.format == PackageFormat::Normal {
            let Some(address) = addresses.get(&entry.path) else {
                return Ok(incomplete(vec![format!(
                    "normal source address capture: {}",
                    entry.path
                )]));
            };
            ArtifactInput::normal_declared_by(&entry.origin, &entry.path, address.clone(), provider)
        } else {
            ArtifactInput::simple_declared_by(&entry.origin, &entry.path, markdown, provider)
        };
        inputs.push(input.map_err(|e| e.to_string())?);
    }
    if inputs.is_empty() {
        return Ok(Preview {
            status: "complete".into(),
            missing: vec![],
            placements,
            artifact: None,
            carrier: None,
            measurement: None,
            comparison: None,
        });
    }
    let xml = world
        .host
        .manifest
        .consumer_node()
        .and_then(|n| n.spec_format)
        == Some(vibe_core::manifest::SpecFormat::Xml);
    let plan = ArtifactPlan::static_lane(
        if xml {
            ArtifactTarget::StaticXml
        } else {
            ArtifactTarget::StaticMarkdown
        },
        vibe_core::layout::current_boot_dir()
            .join(if xml { "STATIC.xml" } else { "STATIC.md" })
            .to_string_lossy(),
        crate::layout_paths::vibedeps(""),
        inputs,
    )
    .map_err(|e| e.to_string())?;
    let trace = Carrier::default();
    let source = vibe_spec::CapturedSectionSource::new(documents)?;
    let artifact = match vibe_spec::compile_artifact_traced(plan, &source, &trace) {
        Ok(a) => a,
        Err(error) => {
            return Ok(incomplete(vec![format!(
                "builtin compilation/captured closure: {error}"
            )]));
        }
    };
    let bytes = trace
        .0
        .into_inner()
        .map_err(|e| e.to_string())?
        .ok_or("missing compiler assemble capture")?;
    let mut projection =
        protocol::project(world.identity(), protocol::CapturedKind::Compiler, &bytes)?;
    let raw = projection
        .graph
        .sources
        .iter()
        .enumerate()
        .filter_map(|(index, source)| {
            let vibe_spec::SourceIdentity::StaticEntry { origin, path } = &source.identity else {
                return None;
            };
            if !effective
                .entries
                .iter()
                .any(|e| &e.origin == origin && &e.path == path)
            {
                return None;
            }
            files.get(path).map(|bytes| (index, bytes))
        })
        .map(|(index, bytes)| {
            measurement::measure_reader(bytes.as_slice()).map(|payload| (index, payload))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()
        .map_err(|e| e.to_string())?;
    projection.measurement =
        measurement::measure_graph(world.identity(), &projection.graph, &raw, &BTreeMap::new());
    projection.measurement.artifact =
        Some(measurement::measure_reader(artifact.bytes()).map_err(|e| e.to_string())?);
    Ok(Preview {
        status: "complete".into(),
        missing: vec![],
        placements,
        artifact: Some(String::from_utf8(artifact.into_bytes()).map_err(|e| e.to_string())?),
        carrier: projection.graph.carrier,
        measurement: Some(projection.measurement),
        comparison: None,
    })
}
fn loading_name(link: LinkType) -> &'static str {
    match link {
        LinkType::Static => "static",
        LinkType::Dynamic => "dynamic",
        LinkType::StaticHard => "static-hard",
        LinkType::StaticTransitive => "static-transitive",
    }
}
fn owner_stage(manifest: &Manifest) -> &'static str {
    if manifest
        .extensions
        .iter()
        .any(|row| row.handler.kind() == "native")
    {
        "native compiler preparation"
    } else {
        "owner-runtime plan capture"
    }
}

#[cfg(test)]
#[path = "m_06_d_1_cases.rs"]
mod m_06_d_1_cases;

pub mod comparison;

fn package_slot(p: &vibe_core::manifest::PackageMeta) -> String {
    if p.materialization.is_in_place() {
        crate::vibedeps::in_place_slot_rel_path(&p.group, &p.name)
    } else {
        crate::vibedeps::slot_rel_path(&p.group, &p.name, &p.version)
    }
}
