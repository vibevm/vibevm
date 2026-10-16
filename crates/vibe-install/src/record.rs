//! Recording toolkit — pin construction and the merge discipline for
//! `vibe.toml` / `vibe.lock`. Pure functions over core types; the
//! plan/apply phases drive them, and `vibe update` reuses the pin
//! helper.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#install-workflow-in-detail");

use vibe_core::manifest::{GitPackageDep, LockedPackage, Lockfile, Manifest, SourceKind};
use vibe_core::{ContentHash, PackageName, PackageRef, SourceUrl, VersionSpec};
use vibe_resolver::ResolvedNode;
use vibe_workspace::install::ResolvedDep;

use crate::fetched::Fetched;

/// A structural requirement over `version` with the given operator.
/// Built via typed `semver::Comparator`, never a string round-trip:
/// `VersionReq::parse("={version}")` rejects versions carrying build
/// metadata (a req has no build-metadata grammar), and build metadata
/// never participates in constraint matching anyway.
fn req_with_op(op: semver::Op, version: &semver::Version) -> semver::VersionReq {
    semver::VersionReq {
        comparators: vec![semver::Comparator {
            op,
            major: version.major,
            minor: Some(version.minor),
            patch: Some(version.patch),
            pre: version.pre.clone(),
        }],
    }
}

/// Build a `<group>/<name>@=<exact-version>` pkgref for fetching the
/// version the solver chose, regardless of how the user originally
/// constrained the package.
pub fn exact_pinned_pkgref(node: &ResolvedNode) -> PackageRef {
    PackageRef {
        kind: None,
        group: Some(node.group.clone()),
        name: PackageName::from_validated(node.name.clone()),
        version: VersionSpec::Req(req_with_op(semver::Op::Exact, &node.version)),
    }
}

/// Convert a caller-supplied root into the form that lands on disk in
/// `vibe.toml` `[requires].packages`. Three cases:
///
/// 1. `exact` set → always `=<resolved-version>`, ignoring whatever
///    constraint the user typed (matches npm `--save-exact` —
///    operator wants exact pinning, not the default).
/// 2. No version given (`flow:wal` → `VersionSpec::Latest`) → write
///    caret based on the resolved version (`^0.1.0`). Same default as
///    Cargo `cargo add`, npm `npm install`, Poetry `poetry add`.
/// 3. An explicit constraint (`@^0.1`, `@=0.2.0`, `@~0.3.1`,
///    `@>=0.2, <1.0`, …) → preserved verbatim. The user already
///    declared their intent; we don't second-guess.
pub fn finalize_pkgref_for_manifest(
    cli_pkgref: &PackageRef,
    resolved_version: &semver::Version,
    exact: bool,
) -> PackageRef {
    let version = if exact {
        VersionSpec::Req(req_with_op(semver::Op::Exact, resolved_version))
    } else if matches!(cli_pkgref.version, VersionSpec::Latest) {
        VersionSpec::Req(req_with_op(semver::Op::Caret, resolved_version))
    } else {
        cli_pkgref.version.clone()
    };
    PackageRef {
        kind: cli_pkgref.kind,
        group: cli_pkgref.group.clone(),
        name: cli_pkgref.name.clone(),
        version,
    }
}

/// Merge new root pkgrefs into `lockfile.meta.root_dependencies`,
/// deduplicating on `(group, name)` (idempotent re-installs don't grow
/// the list). Existing entries for the same `(group, name)` are
/// overwritten by the new pkgref so a constraint change in
/// `vibe install` updates the recorded root constraint.
pub fn merge_root_dependencies(lockfile: &mut Lockfile, roots: &[PackageRef]) {
    for r in roots {
        let pos = lockfile
            .meta
            .root_dependencies
            .iter()
            .position(|existing| existing.group == r.group && existing.name == r.name);
        match pos {
            Some(i) => lockfile.meta.root_dependencies[i] = r.clone(),
            None => lockfile.meta.root_dependencies.push(r.clone()),
        }
    }
}

/// Replace the lock's root snapshot with the complete manifest-derived set.
///
/// A bare `vibe install` resolves the whole workspace union.  Unlike an
/// explicit `vibe install <pkg>` (which merges the newly finalized request
/// into the existing declarations), that union is authoritative: retaining a
/// removed root makes the freshly written lock immediately stale and can leave
/// a package named as a root even though it is absent from the resolved graph.
pub(crate) fn replace_root_dependencies(lockfile: &mut Lockfile, roots: &[PackageRef]) {
    lockfile.meta.root_dependencies = roots.to_vec();
}

/// Merge new root pkgrefs into `manifest.requires.packages`, same
/// dedup discipline as [`merge_root_dependencies`]. Returns `true` if
/// any entry was added or changed — caller writes the manifest only
/// when the in-memory shape actually diverged from disk.
///
/// Skips pkgrefs that are already declared as a git-source in
/// `manifest.requires.git_packages` — those were recorded earlier via
/// the `--git` declaration path and writing them again as
/// registry-resolved would create a `(group, name)` duplicate that
/// `try_from = "RequiresWire"` rejects on the next parse.
pub fn merge_manifest_requires(manifest: &mut Manifest, roots: &[PackageRef]) -> bool {
    let mut changed = false;
    for r in roots {
        if manifest
            .requires
            .git_packages
            .iter()
            .any(|g| Some(&g.group) == r.group.as_ref() && g.name == r.name)
        {
            // Already declared as git-source — leave untouched.
            continue;
        }
        let pos = manifest
            .requires
            .packages
            .iter()
            .position(|existing| existing.group == r.group && existing.name == r.name);
        match pos {
            Some(i) => {
                if manifest.requires.packages[i] != *r {
                    manifest.requires.packages[i] = r.clone();
                    changed = true;
                }
            }
            None => {
                manifest.requires.packages.push(r.clone());
                changed = true;
            }
        }
    }
    changed
}

/// Record a `--git` source declaration into `manifest.requires`:
/// replace any prior git-source entry for the same `(group, name)`
/// (updating an existing declaration) and drop a conflicting
/// registry-resolved entry from `requires.packages`, since M1.15 forbids
/// a `(group, name)` collision between the two tables — the
/// `RequiresWire` deserialiser rejects it on the next parse. Pure: it
/// mutates in memory and the caller persists, exactly as
/// [`merge_manifest_requires`] does (the CLI writes before it resolves,
/// so a panic mid-resolve cannot strand the declaration off disk). This
/// is the manifest-mutation discipline the CLI used to own inline; the
/// CLI now translates the `--git*` flags into the [`GitPackageDep`] and
/// hands it here.
pub fn record_git_source(manifest: &mut Manifest, dep: GitPackageDep) {
    // Drop any prior registry-resolved entry for the same pkgref.
    manifest
        .requires
        .packages
        .retain(|p| !(p.group.as_ref() == Some(&dep.group) && p.name == dep.name));
    // Replace any prior git-source entry for the same pkgref (same shape
    // as updating an existing constraint).
    manifest
        .requires
        .git_packages
        .retain(|g| !(g.group == dep.group && g.name == dep.name));
    manifest.requires.git_packages.push(dep);
}

/// Build a [`LockedPackage`] from a fetched node. The lockfile records
/// the resolution provenance; the materialised footprint is the
/// `vibedeps/` slot — deterministic from `(kind, name, version)` — so
/// `files_written` stays empty and the `NN-` `boot_snippet` filename
/// is retired.
pub(crate) fn locked_package_from_fetched(
    f: &Fetched,
    resolved: &ResolvedDep,
    language: Option<&str>,
) -> LockedPackage {
    let c = &f.cached;
    let source_kind = if c.overridden {
        SourceKind::Override
    } else if c.is_path_source {
        SourceKind::Path
    } else if c.is_git_source {
        SourceKind::Git
    } else if c.is_local {
        SourceKind::Local
    } else if c.is_embedded {
        SourceKind::Embedded
    } else {
        SourceKind::Registry
    };
    LockedPackage {
        kind: c.package_meta().kind,
        group: c.resolved.group.clone(),
        name: PackageName::from_validated(c.resolved.name.clone()),
        version: c.resolved.version.clone(),
        bridge: c.package_meta().bridge,
        authors: c.package_meta().authors.clone(),
        registry: c.registry_name.clone(),
        source_url: SourceUrl::new(c.source_uri.clone()),
        source_ref: c.source_ref.clone(),
        resolved_commit: c.resolved_commit.clone(),
        content_hash: ContentHash::from_validated(c.content_hash.clone()),
        embedded_sources: f.embedded_sources.clone(),
        boot_snippet: None,
        files_written: Vec::new(),
        dependencies: f.meta.dependencies.clone(),
        admitted_by: resolved.admitted_by.clone(),
        via_override: resolved.via_override.clone(),
        overridden: c.overridden,
        source_kind: Some(source_kind),
        via_redirect: c.via_redirect.clone(),
        features: f
            .feature_expansion
            .active_features
            .iter()
            .cloned()
            .collect(),
        subskills_active: Vec::new(),
        describes: c.package_meta().describes.as_ref().map(|p| p.to_string()),
        language: language.map(str::to_string),
        // Record the package's declared materialization so destructive ops
        // and the guard (PROP-022 §2.6) recognise an in-place slot.
        materialization: c.package_meta().materialization,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_root_snapshot_drops_removed_dependencies() {
        let mut lockfile = Lockfile::empty("vibe test", "2026-08-29T00:00:00Z");
        lockfile.meta.root_dependencies = vec![
            "flow:org.example/kept@^1".parse().unwrap(),
            "flow:org.example/removed@^1".parse().unwrap(),
        ];
        let current = vec!["flow:org.example/kept@^1".parse().unwrap()];

        replace_root_dependencies(&mut lockfile, &current);

        assert_eq!(lockfile.meta.root_dependencies, current);
    }

    #[test]
    fn explicit_root_merge_preserves_other_declared_dependencies() {
        let mut lockfile = Lockfile::empty("vibe test", "2026-08-29T00:00:00Z");
        lockfile.meta.root_dependencies = vec!["flow:org.example/kept@^1".parse().unwrap()];
        let added: PackageRef = "flow:org.example/added@^2".parse().unwrap();

        merge_root_dependencies(&mut lockfile, std::slice::from_ref(&added));

        assert_eq!(lockfile.meta.root_dependencies.len(), 2);
        assert_eq!(lockfile.meta.root_dependencies[1], added);
    }
}

fn lock_error(reason: impl Into<String>) -> crate::Error {
    vibe_resolver::SolveError::UnsupportedRoute(format!("configuration lock: {}", reason.into()))
        .into()
}
fn identity(i: &vibe_registry::CandidateIdentity) -> serde_json::Value {
    serde_json::json!({"source":i.source,"content":i.content,"definition":i.definition})
}
fn config_value(v: &vibe_core::manifest::feature_effects::ConfigValue) -> serde_json::Value {
    use vibe_core::manifest::feature_effects::ConfigValue::*;
    match v {
        Bool(v) => serde_json::json!({"bool":v}),
        Integer(v) => serde_json::json!({"integer":v}),
        String(v) => serde_json::json!({"string":v}),
    }
}
/// Lossless, explicitly tagged projection of the admitted B1/B2 definition.
/// It is evidence compared against current authored inputs, never an authority
/// reconstructed from an outcome or an alternate solver input.
fn definition(
    d: &vibe_resolver::features::configuration::ConfigurationDefinition,
) -> crate::error::Result<String> {
    use serde_json::{Value, json};
    use vibe_core::manifest::{feature_constraints::*, feature_effects::*};
    let guard = |g: &Guard| match g {
        Guard::Group(n) => json!({"group":n.as_str()}),
        Guard::Member(n) => json!({"member":n.as_str()}),
    };
    let mut dependencies = serde_json::Map::new();
    for (id, dep) in &d.declarations.dependencies {
        let source = match &dep.source {
            DependencySource::Registry(p) => json!({"registry":p.to_string()}),
            _ => {
                return Err(lock_error(
                    "configured persistence requires admitted registry declarations",
                ));
            }
        };
        dependencies.insert(
            id.as_str().into(),
            json!({"source":source,"optional":dep.optional,
            "link":dep.link,"access":dep.access,"friend":dep.friend,"exclude":dep.exclude}),
        );
    }
    let features: serde_json::Map<String,Value> = d.program.features.iter().map(|(n,effects)| {
        (n.as_str().into(), Value::Array(effects.iter().map(|e| match e {
            FeatureEffect::Local(f) => json!({"local":f.as_str()}),
            FeatureEffect::ActivateDependency(dep) => json!({"dependency":dep.as_str()}),
            FeatureEffect::TargetFeature{dependency,feature,strength} => json!({"dependency":dependency.as_str(),"feature":feature.as_str(),"strength":match strength { FeatureStrength::Strong => "strong", FeatureStrength::Weak => "weak" }}),
            FeatureEffect::ExportedSubskill(p) => json!({"subskill":p}),
            FeatureEffect::ConfigValue(n) => json!({"config":n.as_str()}),
        }).collect()))
    }).collect();
    let groups: serde_json::Map<String,Value> = d.constraints.groups.iter().map(|(n,g)|
        (n.as_str().into(),json!({"cardinality":match g.cardinality {Cardinality::ZeroOrMore=>"zero-or-more",Cardinality::OneOrMore=>"one-or-more",Cardinality::ZeroOrOne=>"zero-or-one",Cardinality::ExactlyOne=>"exactly-one"},
            "members":g.members.iter().map(|m|m.as_str()).collect::<Vec<_>>(),
            "guards":g.guards.iter().map(&guard).collect::<Vec<_>>(),
            "defaults":g.defaults.as_ref().map(|v|v.iter().map(|m|m.as_str()).collect::<Vec<_>>())}))).collect();
    let i = &d.instance;
    let domain = &i.domain;
    let package = match &i.package {
        PackageIdentity::Vibe(p) => json!({"vibe":p.to_string()}),
        PackageIdentity::Native {
            backend,
            package,
            version,
        } => {
            json!({"native":{"backend":backend.as_str(),"package":package.as_str(),"version":version}})
        }
    };
    let value = json!({
        "instance":{"identity":i.identity.as_str(),"configuration":i.configuration.as_str(),"package":package,
            "domain":{"identity":domain.identity.as_str(),
                "definition":{"reference":domain.definition.reference().as_str(),"producer":domain.definition.producer().as_str()},
                "target":{"reference":domain.target.reference().as_str(),"producer":domain.target.producer().as_str()},
                "target_role":match domain.target_role {vibe_core::environment::LockTargetRole::Executor=>"executor",vibe_core::environment::LockTargetRole::ProducedToolRuntime=>"produced_tool_runtime",vibe_core::environment::LockTargetRole::EmittedCodeTarget=>"emitted_code_target"},
                "lock":domain.lock.as_ref().map(|l|json!({"reference":l.reference().as_str(),"producer":l.producer().as_str()}))}},
        "program":{"features":features,"exclusive":d.program.exclusive.iter().map(|(n,v)|(n.clone(),json!(v.iter().map(|f|f.as_str()).collect::<Vec<_>>()))).collect::<serde_json::Map<String,Value>>()},
        "declarations":{"dependencies":dependencies,"aliases":d.declarations.aliases.iter().map(|(a,b)|(a.as_str().into(),json!(b.as_str()))).collect::<serde_json::Map<String,Value>>(),
            "exported_subskills":d.declarations.exported_subskills,"config_values":d.declarations.config_values.iter().map(|(n,v)|(n.as_str().into(),config_value(v))).collect::<serde_json::Map<String,Value>>()},
        "constraints":{"groups":groups,"member_guards":d.constraints.member_guards.iter().map(|(n,v)|(n.as_str().into(),json!(v.iter().map(&guard).collect::<Vec<_>>()))).collect::<serde_json::Map<String,Value>>()}
    });
    serde_json::to_string(&value).map_err(|e| lock_error(e.to_string()))
}

fn requested_record(
    request: &vibe_resolver::resolvo_engine::request::NormalizedRequest,
) -> crate::error::Result<vibe_wire::generated::lock_configuration::e1::requested::Requested> {
    use serde_json::json;
    let mut definitions = Vec::new();
    let mut defaults = Vec::new();
    if let Some(c) = &request.configuration {
        for d in &c.definitions {
            definitions.push(json!({"package":d.package.to_string(),"version":d.version.to_string(),"identity":identity(&d.identity),"definition":definition(&d.definition)?}));
        }
        for b in &c.defaults {
            let (edge, id, shown, r) = b.receipt();
            defaults.push(json!({"edge":edge,"identity":identity(id),"definition":definition(shown)?,"request":serde_json::to_string(r).map_err(|e|lock_error(e.to_string()))?}));
        }
    }
    let mut value = json!({"epoch":"1","roots":request.roots.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "origins":request.root_origins.iter().map(|r|json!({"constraint":r.constraint.to_string(),"consumer":r.consumer,"declaration":r.declaration})).collect::<Vec<_>>(),
        "features":request.features.explicit,"no_defaults":request.features.no_defaults,"all_features":request.features.all,
        "definitions":definitions,"defaults":defaults});
    if let Some(c) = &request.configuration {
        value["configuration"] =
            json!(serde_json::to_string(&c.request).map_err(|e| lock_error(e.to_string()))?);
    }
    serde_json::from_value(value).map_err(|e| lock_error(e.to_string()))
}

/// Build once before apply effects, from the exact confirmed C3 evidence.
pub(crate) fn configuration_record(
    request: Option<&vibe_resolver::resolvo_engine::request::NormalizedRequest>,
    provenance: &[vibe_resolver::resolvo_engine::request::EdgeProvenance],
    fetched: &[Fetched],
) -> crate::error::Result<Option<vibe_core::manifest::LockedConfiguration>> {
    use serde_json::json;
    let Some(request) = request else {
        return Ok(None);
    };
    let requested = requested_record(request)?;
    let mut configurations = Vec::new();
    if let Some(c) = &request.configuration {
        let admitted = vibe_wire::behaviour::consumer_configuration::admit_request(&c.request)
            .map_err(lock_error)?;
        for d in &c.definitions {
            if !fetched.iter().any(|f| {
                d.package.group.as_ref() == Some(&f.cached.resolved.group)
                    && d.package.name == f.cached.resolved.name
                    && d.version == f.cached.resolved.version
            }) {
                continue;
            }
            let indices: Vec<_> = admitted
                .iter()
                .enumerate()
                .filter(|(_, e)| e.instance == d.definition.instance)
                .map(|(i, _)| i)
                .collect();
            if indices.is_empty() {
                continue;
            }
            for &index in &indices {
                let edge = &admitted[index];
                let omitted_default = |definition: &vibe_resolver::features::configuration::ConfigurationDefinition| {
                    definition.constraints.groups.iter().any(|(name,group)| group.defaults.is_some()
                        && edge.selections.get(name).is_none_or(|s|matches!(s.selection,vibe_core::manifest::feature_constraints::Selection::Omitted)))
                };
                let governed = edge.policy
                    == vibe_core::manifest::feature_constraints::DefaultPolicy::UseDeclaredDefaults
                    && (omitted_default(&d.definition)
                        || c.defaults.iter().any(|b| {
                            let (i, _, shown, _) = b.receipt();
                            i == index && omitted_default(shown)
                        }));
                if governed
                    && !c.defaults.iter().any(|b| {
                        let (i, id, shown, r) = b.receipt();
                        i == index && id == &d.identity && shown == &d.definition && r == &c.request
                    })
                {
                    return Err(lock_error(
                        "missing exact shown-definition default authorization",
                    ));
                }
            }
            if provenance.iter().any(|p| {
                p.selected.parent.group == d.package.group
                    && p.selected.parent.name == d.package.name
                    && p.configured_definition.as_ref() != Some(&d.definition)
            }) {
                return Err(lock_error(
                    "confirmed configured definition differs from selected source receipt",
                ));
            }

            let local = vibe_resolver::resolvo_engine::request::consumer_wire::Request {
                epoch: c.request.epoch.clone(),
                edges: indices
                    .iter()
                    .map(|&i| c.request.edges[i].clone())
                    .collect(),
            };
            let outcome = vibe_resolver::features::configuration::resolve_consumer_configuration(
                &local,
                std::slice::from_ref(&d.definition),
            )
            .map_err(lock_error)?;
            if outcome.result.outcomes.iter().any(|o|o.status != vibe_wire::generated::consumer_configuration::e1::result::OutcomeStatus::Resolved) {
                return Err(lock_error("unresolved configuration at record boundary"));
            }
            let fetched_effects = &fetched
                .iter()
                .find(|f| {
                    d.package.group.as_ref() == Some(&f.cached.resolved.group)
                        && d.package.name == f.cached.resolved.name
                        && d.version == f.cached.resolved.version
                })
                .expect("selected definition")
                .feature_expansion;
            if outcome.expansions.first().and_then(Option::as_ref) != Some(fetched_effects) {
                return Err(lock_error(
                    "selected feature closure differs from admitted definition and intent",
                ));
            }
            // Keep dormant intent per original consumer edge and group.
            let selected = &outcome.result.outcomes[0].selected;
            let mut dormant = Vec::new();
            for &i in &indices {
                for (group, selection) in &c.request.edges[i].selections {
                    if let Some(members) = &selection.members {
                        let members: Vec<_> = members
                            .iter()
                            .filter(|m| selected.get(group).is_none_or(|v| !v.contains(m)))
                            .cloned()
                            .collect();
                        if !members.is_empty() {
                            dormant.push(json!({"edge":i,"group":group,"members":members}));
                        }
                    }
                }
            }
            configurations.push(json!({"package":format!("{}@{}",d.package.qualified_name(),d.version),"result":serde_json::to_string(&outcome.result).map_err(|e|lock_error(e.to_string()))?,"dormant":dormant}));
        }
    }
    let packages: Vec<_> = fetched.iter().map(|f| {
        let e=&f.feature_expansion;
        json!({"package":format!("{}/{}@{}",f.cached.resolved.group,f.cached.resolved.name,f.cached.resolved.version),
            "content":f.cached.content_hash,"feature_closure":e.active_features,
            "effects":json!({"dependencies":e.active_deps,"dependency_features":e.dep_features,"weak_dependency_features":e.weak_dep_features,"subskills":e.active_subskills,
                "config_values":e.config_values.iter().map(|(n,v)|(n.clone(),config_value(v))).collect::<serde_json::Map<_,_>>()}).to_string()})
    }).collect();
    let mut edges = Vec::new();
    for p in provenance {
        edges.push(json!({"parent":p.selected.parent.to_string(),"declaration":p.selected.declaration,"alternative":p.selected.alternative,
            "provider":p.selected.provider.to_string(),"owner_identity":identity(&p.declaration_identity),"provider_identity":identity(&p.provider_identity),
            "definition":p.configured_definition.as_ref().map(definition).transpose()?.unwrap_or_default(),"request_digest":"pending"}));
    }
    let resolved = serde_json::from_value(json!({"epoch":"1","request_digest":"pending","packages":packages,"edges":edges,"configurations":configurations})).map_err(|e|lock_error(e.to_string()))?;
    vibe_wire::behaviour::lock_configuration::encode(requested, resolved)
        .map(Some)
        .map_err(lock_error)
}

#[cfg(test)]
mod m_05_d_1_cases;

/// Read generated payloads before any legacy freshness or migration branch.
pub(crate) fn read_configuration(lock: &Lockfile) -> crate::error::Result<bool> {
    match &lock.meta.configuration {
        None => Ok(false),
        Some(record) => Ok(vibe_wire::behaviour::lock_configuration::decode(record)
            .map_err(lock_error)?
            .0
            .configuration
            .is_some()),
    }
}

/// Compare against current caller-admitted authored authority, never against
/// defaults or consumer edges inferred from the retained derived outcome.
pub(crate) fn validate_replay<S: crate::InstallSource + ?Sized>(
    lock: &Lockfile,
    source: &S,
    explicit: &[PackageRef],
) -> crate::error::Result<()> {
    let Some(record) = &lock.meta.configuration else {
        return Ok(());
    };
    let (old, resolved) =
        vibe_wire::behaviour::lock_configuration::decode(record).map_err(lock_error)?;
    if old.configuration.is_none() {
        return Ok(());
    }
    let current = source.normalized_request().ok_or_else(|| {
        lock_error("current authored configuration authority unavailable; exact replay refused")
    })?;
    let now = requested_record(current)?;
    if now.configuration.is_none() {
        return Err(lock_error(
            "current authored configuration authority unavailable; refusing to strip configured lock",
        ));
    }
    if explicit.is_empty() {
        if now != old {
            return Err(lock_error(
                "authored request, default receipt or same-version definition changed; exact replay refused",
            ));
        }
    } else {
        // Named version resolution is authorized, not loss of consumer intent.
        if now.configuration != old.configuration
            || now.features != old.features
            || now.no_defaults != old.no_defaults
            || now.all_features != old.all_features
        {
            return Err(lock_error(
                "explicit roots do not authorize changing other configuration intent",
            ));
        }
        let named = |p: &str| {
            p.parse::<PackageRef>().ok().is_some_and(|p| {
                explicit
                    .iter()
                    .any(|e| e.group == p.group && e.name == p.name)
            })
        };
        for d in &old.definitions {
            if !named(&d.package) && !now.definitions.contains(d) {
                return Err(lock_error(
                    "regeneration changed an unnamed configured definition",
                ));
            }
        }
    }
    let catalog = source
        .catalog_inspection()
        .ok_or_else(|| lock_error("exact replay requires current captured candidate identities"))?;
    for edge in resolved.edges {
        for (reference, source_id, content, definition_id) in [
            (
                &edge.parent,
                &edge.owner_identity.source,
                &edge.owner_identity.content,
                &edge.owner_identity.definition,
            ),
            (
                &edge.provider,
                &edge.provider_identity.source,
                &edge.provider_identity.content,
                &edge.provider_identity.definition,
            ),
        ] {
            let p: PackageRef = reference
                .parse()
                .map_err(|e| lock_error(format!("invalid retained edge: {e}")))?;
            if explicit
                .iter()
                .any(|e| e.group == p.group && e.name == p.name)
            {
                continue;
            }
            let present = catalog.candidates.iter().any(|(coordinate, id)| {
                let Ok(candidate) = coordinate.parse::<PackageRef>() else {
                    return false;
                };
                candidate.group == p.group
                    && candidate.name == p.name
                    && id.source == *source_id
                    && id.content == *content
                    && id.definition == *definition_id
            });
            if !present {
                return Err(lock_error(
                    "selected declaration/provider capture changed; exact replay refused",
                ));
            }
        }
    }
    Ok(())
}

/// Exact replay also binds the actual derived result to the current solve.
pub(crate) fn validate_replayed_result(
    lock: &Lockfile,
    current: Option<&vibe_core::manifest::LockedConfiguration>,
    explicit: &[PackageRef],
) -> crate::error::Result<()> {
    if read_configuration(lock)? {
        let previous = lock.meta.configuration.as_ref().expect("configured lock");
        let current = current
            .ok_or_else(|| lock_error("current authored configuration authority unavailable"))?;
        let (old, _) =
            vibe_wire::behaviour::lock_configuration::decode(previous).map_err(lock_error)?;
        let (now, _) =
            vibe_wire::behaviour::lock_configuration::decode(current).map_err(lock_error)?;
        if now.configuration != old.configuration
            || now.features != old.features
            || now.no_defaults != old.no_defaults
            || now.all_features != old.all_features
        {
            return Err(lock_error(
                "regeneration cannot strip or replace confirmed configuration intent",
            ));
        }
        if explicit.is_empty() && current != previous {
            return Err(lock_error(
                "resolved choices, closure or selected providers changed; exact replay refused",
            ));
        }
    }
    Ok(())
}
