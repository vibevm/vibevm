//! Exact captured replay and explicit update admission. D1 remains the codec;
//! B2 and the selected source remain the only configuration/solve engines.

use crate::{Fetched, InstallRequest, InstallSource, Plan, PlanObserver};
use serde_json::{Value, json};
use std::path::Path;
use vibe_core::manifest::{Lockfile, Manifest, SpecFormat};
use vibe_resolver::resolvo_engine::request::{EdgeProvenance, NormalizedRequest};
use vibe_workspace::{Workspace, install::ResolvedDep};

fn error(message: impl Into<String>) -> crate::Error {
    vibe_resolver::SolveError::UnsupportedRoute(message.into()).into()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    ExactReuse,
    SemanticInputChange,
    Unavailable,
    ReconfigurationRequired,
}

/// Complete before/after values, not a digest-only claim of equivalence.
#[derive(Debug, Clone)]
pub struct EvolutionReport {
    pub decision: Decision,
    pub retained_request: Option<String>,
    pub current_request: Option<String>,
    pub differences: Value,
    pub needs: Vec<String>,
}
impl EvolutionReport {
    pub fn value(&self) -> Value {
        json!({"decision":format!("{:?}",self.decision),"retained_request":self.retained_request,
            "current_request":self.current_request,"differences":self.differences,"needs":self.needs})
    }
}

fn report(
    lock: &Lockfile,
    current: Option<&NormalizedRequest>,
) -> crate::error::Result<EvolutionReport> {
    let before = lock
        .meta
        .configuration
        .as_ref()
        .map(|r| vibe_wire::behaviour::lock_configuration::decode(r).map_err(error))
        .transpose()?;
    let now = current.map(crate::requested_record).transpose()?;
    let current_request = now
        .as_ref()
        .map(|r| {
            serde_json::to_string(r)
                .map(|s| vibe_core::manifest::LockedConfiguration::digest(&s))
                .map_err(|e| error(e.to_string()))
        })
        .transpose()?;
    Ok(EvolutionReport {
        decision: Decision::Unavailable,
        retained_request: lock
            .meta
            .configuration
            .as_ref()
            .map(|r| r.request_digest.clone()),
        current_request,
        differences: json!({"requested":{"before":before.as_ref().map(|b| &b.0),"after":now},
            "resolved":{"before":before.as_ref().map(|b| &b.1),"after":null},
            "packages":{"before":lock.packages,"after":null}}),
        needs: vec![],
    })
}

fn payload_observations(fetched: &[Fetched]) -> Value {
    json!(fetched.iter().map(|f| {
        let current=vibe_registry::compute_content_hash(&f.cached.cache_dir);
        json!({"package":format!("{}/{}@{}",f.cached.resolved.group,f.cached.resolved.name,f.cached.resolved.version),
            "expected":f.cached.content_hash,"current":current.as_ref().ok(),"unavailable":current.err().map(|e|e.to_string())})
    }).collect::<Vec<_>>())
}

fn replay_failure<S: InstallSource + ?Sized>(
    lock: &Lockfile,
    source: &S,
    retained: &vibe_resolver::resolvo_engine::CatalogInspection,
    fetched: &[Fetched],
    reason: impl ToString,
) -> crate::Error {
    let reason = reason.to_string();
    let Ok(mut report) = report(lock, source.normalized_request()) else {
        return error(format!("reinstall unavailable: {reason}"));
    };
    let project = |catalog: &vibe_resolver::resolvo_engine::CatalogInspection| {
        json!(catalog.candidates.iter().map(|(package,id)|json!({"package":package,"source":id.source,"content":id.content,"definition":id.definition})).collect::<Vec<_>>())
    };
    let available = |catalog: &vibe_resolver::resolvo_engine::CatalogInspection| {
        lock.packages.iter().all(|locked| {
            catalog.candidates.iter().any(|(p, _)| {
                p.parse::<vibe_core::PackageRef>().is_ok_and(|p| {
                    p.group.as_ref() == Some(&locked.group)
                        && p.name == locked.name
                        && p.version.matches(&locked.version)
                })
            })
        })
    };
    let current = source.catalog_inspection();
    let observations = payload_observations(fetched);
    let complete = report.retained_request.is_some()
        && report.current_request.is_some()
        && available(retained)
        && current.is_some_and(available)
        && fetched.len() == lock.packages.len()
        && observations
            .as_array()
            .is_some_and(|rows| rows.iter().all(|r| r["current"].is_string()));
    let changed = report.current_request != report.retained_request
        || current.is_some_and(|c| project(c) != project(retained))
        || observations
            .as_array()
            .is_some_and(|rows| rows.iter().any(|r| r["expected"] != r["current"]));
    report.decision = if complete && changed {
        Decision::SemanticInputChange
    } else {
        Decision::Unavailable
    };
    report.differences["captures"] =
        json!({"before":project(retained),"after":current.map(project)});
    report.differences["payload_observations"] = observations;
    report.needs.push(reason);
    error(format!("reinstall refused: {}", report.value()))
}

/// An immutable replay proof over caller-prepared captures. No provider,
/// solver, fetch, replacement capture or default selection is invoked here.
/// Construct before continuation/lifecycle effects; revalidate at application.
#[derive(Debug)]
pub struct PreparedReinstall {
    lock: Lockfile,
    fetched: Vec<Fetched>,
    pub report: EvolutionReport,
}
impl PreparedReinstall {
    /// Retain the exact payloads and proof from a successful ordinary plan.
    /// The lock must be that plan's durable result; no solve happens here.
    pub fn from_planned<S: InstallSource + ?Sized>(
        lock: &Lockfile,
        source: &S,
        plan: &crate::PlannedInstall,
        retained_catalog: &vibe_resolver::resolvo_engine::CatalogInspection,
    ) -> crate::error::Result<Self> {
        let captured = plan.captured_catalog.as_ref().ok_or_else(|| {
            replay_failure(
                lock,
                source,
                retained_catalog,
                &plan.fetched,
                "reinstall unavailable: original plan capture missing",
            )
        })?;
        // The held plan, rather than a newly supplied catalog, is the retained
        // authority. Keep the explicit receipt parameter for hosts retaining
        // independent evidence, and require agreement on every locked row.
        for package in &lock.packages {
            let matches = |p: &String| {
                p.parse::<vibe_core::PackageRef>().is_ok_and(|p| {
                    p.group.as_ref() == Some(&package.group)
                        && p.name == package.name
                        && p.version.matches(&package.version)
                })
            };
            let original = captured.candidates.iter().find(|(p, _)| matches(p));
            if original.is_none()
                || !retained_catalog
                    .candidates
                    .iter()
                    .any(|row| Some(row) == original)
            {
                return Err(replay_failure(
                    lock,
                    source,
                    retained_catalog,
                    &plan.fetched,
                    "reinstall unavailable: original plan capture missing or substituted",
                ));
            }
        }
        Self::new(
            lock,
            source,
            plan.fetched.clone(),
            &plan.selected_provenance,
            captured,
        )
    }
    pub fn new<S: InstallSource + ?Sized>(
        lock: &Lockfile,
        source: &S,
        fetched: Vec<Fetched>,
        provenance: &[EdgeProvenance],
        retained_catalog: &vibe_resolver::resolvo_engine::CatalogInspection,
    ) -> crate::error::Result<Self> {
        let observations = fetched.clone();
        Self::check(lock, source, fetched, provenance, retained_catalog)
            .map_err(|reason| replay_failure(lock, source, retained_catalog, &observations, reason))
    }

    fn check<S: InstallSource + ?Sized>(
        lock: &Lockfile,
        source: &S,
        fetched: Vec<Fetched>,
        provenance: &[EdgeProvenance],
        retained_catalog: &vibe_resolver::resolvo_engine::CatalogInspection,
    ) -> crate::error::Result<Self> {
        let current = source
            .normalized_request()
            .ok_or_else(|| error("reinstall unavailable: current authored authority missing"))?;
        if lock.meta.configuration.is_none() {
            return Err(error(
                "reinstall unavailable: retained configuration capture missing",
            ));
        }
        crate::record::validate_replay(lock, source, &[])?;
        let catalog = source
            .catalog_inspection()
            .ok_or_else(|| error("reinstall unavailable: captured catalog missing"))?;
        if fetched.len() != lock.packages.len() {
            return Err(error("reinstall unavailable: incomplete payload captures"));
        }
        for locked in &lock.packages {
            let matches: Vec<_> = fetched
                .iter()
                .filter(|f| {
                    f.cached.resolved.group == locked.group
                        && f.cached.resolved.name == locked.name.as_str()
                        && f.cached.resolved.version == locked.version
                })
                .collect();
            if matches.len() != 1 {
                return Err(error(
                    "reinstall unavailable: missing or duplicate payload capture",
                ));
            }
            let f = matches[0];
            let coordinate = format!("{}/{}@{}", locked.group, locked.name, locked.version);
            let metadata =
                vibe_registry::CandidateMetadata::read_local_registry(&f.cached.cache_dir)?;
            // Cached local metadata has its own source URI; compare exact
            // manifest/bytes, and use the admitted catalog for source identity.
            let id = catalog
                .candidates
                .iter()
                .find(|(p, _)| {
                    p.parse::<vibe_core::PackageRef>().is_ok_and(|p| {
                        p.group.as_ref() == Some(&locked.group)
                            && p.name == locked.name
                            && p.version.matches(&locked.version)
                    })
                })
                .map(|(_, id)| id)
                .ok_or_else(|| {
                    error(format!(
                        "reinstall unavailable: capture missing for {coordinate}"
                    ))
                })?;
            if !retained_catalog.candidates.iter().any(|(p, old)| {
                old == id
                    && p.parse::<vibe_core::PackageRef>().is_ok_and(|p| {
                        p.group.as_ref() == Some(&locked.group)
                            && p.name == locked.name
                            && p.version.matches(&locked.version)
                    })
            }) {
                return Err(error(format!(
                    "reinstall stale: retained source/content/definition capture missing for {coordinate}"
                )));
            }
            if f.cached.content_hash != locked.content_hash.as_str()
                || vibe_registry::compute_content_hash(&f.cached.cache_dir)?
                    != locked.content_hash.as_str()
                || metadata.manifest != f.cached.manifest
                || id.definition != metadata.identity.definition
                || f.meta.dependencies != locked.dependencies
                || f.feature_expansion
                    .active_features
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    != locked.features
                || f.cached.source_uri != locked.source_url.as_str()
                || f.cached.source_ref != locked.source_ref
                || f.cached.resolved_commit != locked.resolved_commit
                || f.cached.package_meta().kind != locked.kind
                || f.cached.package_meta().materialization != locked.materialization
                || f.embedded_sources != locked.embedded_sources
                || !(id.content == f.cached.content_hash
                    || id
                        .content
                        .strip_prefix("git:")
                        .is_some_and(|commit| Some(commit) == f.cached.resolved_commit.as_deref()))
            {
                return Err(error(format!(
                    "reinstall stale: payload/source/graph differs for {coordinate}"
                )));
            }
        }
        let record = crate::record::configuration_record(Some(current), provenance, &fetched)?;
        if record != lock.meta.configuration {
            return Err(error(
                "reinstall stale: resolved effects or definitions differ",
            ));
        }
        let mut report = report(lock, Some(current))?;
        report.decision = Decision::ExactReuse;
        report.differences["resolved"]["after"] = report.differences["resolved"]["before"].clone();
        report.differences["packages"]["after"] = report.differences["packages"]["before"].clone();
        Ok(Self {
            lock: lock.clone(),
            fetched,
            report,
        })
    }

    pub fn validate(&self, lock: &Lockfile) -> crate::error::Result<()> {
        if &self.lock != lock {
            return Err(self.refused(
                lock,
                Decision::SemanticInputChange,
                "reinstall stale: lock changed after preparation",
                Value::Null,
            ));
        }
        for f in &self.fetched {
            match vibe_registry::compute_content_hash(&f.cached.cache_dir) {
                Ok(current) if current == f.cached.content_hash => {}
                Ok(_) => {
                    return Err(self.refused(
                        lock,
                        Decision::SemanticInputChange,
                        "reinstall stale: prepared payload changed",
                        Value::Null,
                    ));
                }
                Err(_) => {
                    return Err(self.refused(
                        lock,
                        Decision::Unavailable,
                        "reinstall unavailable: prepared payload cannot be observed",
                        Value::Null,
                    ));
                }
            }
        }
        Ok(())
    }

    fn refused(
        &self,
        lock: &Lockfile,
        decision: Decision,
        reason: &str,
        slot: Value,
    ) -> crate::Error {
        let mut report = self.report.clone();
        report.decision = decision;
        report.differences["lock_observation"] = json!({"before":self.lock,"after":lock});
        report.differences["payload_observations"] = payload_observations(&self.fetched);
        report.differences["slot_observation"] = slot;
        report.needs.push(reason.into());
        error(format!("reinstall refused: {}", report.value()))
    }

    /// Plain regeneration consumes installed slots. Prove those slots with
    /// the existing materialization verifier before any lifecycle callback.
    pub fn validate_slots(&self, root: &Path, format: SpecFormat) -> crate::error::Result<()> {
        use vibe_workspace::install::{SlotCheck, SlotVerifier};
        let verifier = crate::slot_verify::RegistrySlotVerifier::from_fetched(&self.fetched);
        for dep in self.resolution() {
            let slot =
                vibe_workspace::vibedeps::slot_abs_path(root, &dep.group, &dep.name, &dep.version);
            let check = verifier.verify_slot_for_format(&dep, &slot, format);
            if !matches!(check, SlotCheck::Verified) {
                let decision = if matches!(check, SlotCheck::Unverifiable) {
                    Decision::Unavailable
                } else {
                    Decision::SemanticInputChange
                };
                return Err(self.refused(&self.lock,decision,
                    "reinstall stale: installed slot differs or is unavailable; use --force with the same prepared captures",
                    json!({"package":format!("{}/{}",dep.group,dep.name),"expected":dep.source_hash,"observed":format!("{check:?}")})));
            }
        }
        Ok(())
    }

    pub fn resolution(&self) -> Vec<ResolvedDep> {
        self.fetched
            .iter()
            .map(|f| {
                let locked = self
                    .lock
                    .find(&f.cached.resolved.group, &f.cached.resolved.name)
                    .expect("validated capture");
                ResolvedDep {
                    kind: locked.kind,
                    group: locked.group.clone(),
                    name: locked.name.to_string(),
                    version: locked.version.clone(),
                    content_dir: f.cached.cache_dir.clone(),
                    source_hash: Some(locked.content_hash.clone()),
                    embedded_sources: locked.embedded_sources.clone(),
                    manifest: f.cached.manifest.clone(),
                    requires: locked
                        .dependencies
                        .iter()
                        .filter_map(|p| p.group.clone().map(|g| (g, p.name.to_string())))
                        .collect(),
                    admitted_by: locked.admitted_by.clone(),
                    via_override: locked.via_override.clone(),
                    source_mutable: false,
                    in_place_changed: None,
                }
            })
            .collect()
    }
}

/// Adapters with no typed current authority must refuse before legacy update
/// can erase selections. Flags never constitute that authority.
pub fn require_update_authority(lock: &Lockfile) -> crate::error::Result<()> {
    if crate::record::read_configuration(lock)? {
        return Err(error(
            "update unavailable: current authored configuration authority and captures required; explicit reconfiguration must use the admitted update route",
        ));
    }
    Ok(())
}

pub struct UpdatePlan {
    pub report: EvolutionReport,
    pub plan: Option<Plan>,
}

/// Explicit update over current admitted definitions. All original consumer
/// choices are checked before the incumbent solver may run. Reconfiguration
/// returns every gap and no applicable plan.
#[allow(clippy::too_many_arguments)]
pub fn plan_update<S: InstallSource + ?Sized>(
    source: &S,
    root: &Path,
    manifest: &mut Manifest,
    workspace: &mut Workspace,
    request: InstallRequest,
    format: SpecFormat,
    observer: &dyn PlanObserver,
) -> crate::error::Result<UpdatePlan> {
    if source
        .normalized_request()
        .and_then(|r| r.configuration.as_ref())
        .is_none()
        && workspace
            .iter_nodes()
            .any(|(_, node)| !node.consumer_configuration.is_empty())
    {
        let bound = crate::bind_authored_request(source, workspace, &request.features)?;
        return plan_update(
            &*bound, root, manifest, workspace, request, format, observer,
        );
    }
    let lock = Lockfile::read(workspace.root.join(Lockfile::FILENAME))?;
    let current = source.normalized_request();
    let mut report = report(&lock, current)?;
    let (Some(record), Some(current)) = (&lock.meta.configuration, current) else {
        report
            .needs
            .push("retained request or current admitted authority unavailable".into());
        return Ok(UpdatePlan { report, plan: None });
    };
    let (old, retained_resolved) =
        vibe_wire::behaviour::lock_configuration::decode(record).map_err(error)?;
    let now = crate::requested_record(current)?;
    let mut candidate_needs = std::collections::BTreeMap::<String, Vec<String>>::new();
    let mut unavailable = false;
    if old.configuration != now.configuration
        || old.features != now.features
        || old.no_defaults != now.no_defaults
        || old.all_features != now.all_features
        || old.roots != now.roots
        || old.origins != now.origins
    {
        report
            .needs
            .push("authored consumer intent changed; explicit reconfiguration required".into());
    }
    if let Some(c) = &current.configuration {
        let admitted = vibe_wire::behaviour::consumer_configuration::admit_request(&c.request)
            .map_err(error)?;
        for (i, edge) in admitted.iter().enumerate() {
            let edge_start = report.needs.len();
            let mut viable = false;
            let definitions: Vec<_> = c
                .definitions
                .iter()
                .filter(|d| d.definition.instance == edge.instance)
                .collect();
            if definitions.is_empty() {
                unavailable = true;
                report
                    .needs
                    .push(format!("edge {i}: current definition unavailable"));
            }
            for d in definitions {
                let start = report.needs.len();
                for (name, group) in &d.definition.constraints.groups {
                    let existed = old
                        .definitions
                        .iter()
                        .filter(|old| old.package == d.package.to_string())
                        .filter(|old| {
                            old.package.parse::<vibe_core::PackageRef>().is_ok_and(|p| {
                                retained_resolved.packages.iter().any(|selected| {
                                    selected.package
                                        == format!("{}@{}", p.qualified_name(), old.version)
                                })
                            })
                        })
                        .any(|old| {
                            serde_json::from_str::<Value>(&old.definition)
                                .ok()
                                .is_some_and(|v| {
                                    v["constraints"]["groups"].get(name.as_str()).is_some()
                                })
                        });
                    if !existed && !group.cardinality.accepts(0) {
                        report.needs.push(format!(
                            "edge {i}: new required group {} requires explicit reconfiguration",
                            name.as_str()
                        ));
                    }
                }
                for (name, selected) in &edge.selections {
                    match d.definition.constraints.groups.get(name) {
                        None => report
                            .needs
                            .push(format!("edge {i}: removed group {}", name.as_str())),
                        Some(group) => {
                            if let vibe_core::manifest::feature_constraints::Selection::Explicit(
                                members,
                            ) = &selected.selection
                            {
                                for member in members {
                                    if !group.members.contains(member) {
                                        report.needs.push(format!(
                                            "edge {i}: removed member {}/{}",
                                            name.as_str(),
                                            member.as_str()
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
                let local = vibe_resolver::resolvo_engine::request::consumer_wire::Request {
                    epoch: c.request.epoch.clone(),
                    edges: c
                        .request
                        .edges
                        .iter()
                        .enumerate()
                        .filter(|(j, _)| admitted[*j].instance == edge.instance)
                        .map(|(_, e)| e.clone())
                        .collect(),
                };
                match vibe_resolver::features::configuration::resolve_consumer_configuration(
                    &local,
                    std::slice::from_ref(&d.definition),
                ) {
                    Ok(outcome) => {
                        for o in &outcome.result.outcomes {
                            if o.status != vibe_wire::generated::consumer_configuration::e1::result::OutcomeStatus::Resolved {
                            report.needs.push(format!("edge {i}: {}", serde_json::to_string(o).map_err(|e| error(e.to_string()))?));
                        }
                        }
                    }
                    Err(e) => report.needs.push(format!("edge {i}: {e}")),
                }
                // A prior shown-default receipt remains governed even when
                // the new definition removes that group/default entirely.
                let previous_default = old.defaults.iter().any(|b| b.edge == i as u32);
                let uses_defaults = edge.policy
                    == vibe_core::manifest::feature_constraints::DefaultPolicy::UseDeclaredDefaults
                    && (previous_default
                        || d.definition.constraints.groups.iter().any(|(n, g)| {
                            g.defaults.is_some()
                                && edge.selections.get(n).is_none_or(|s| {
                                    matches!(
                                    s.selection,
                                    vibe_core::manifest::feature_constraints::Selection::Omitted
                                )
                                })
                        }));
                if uses_defaults
                    && !c.defaults.iter().any(|b| {
                        let (j, id, shown, r) = b.receipt();
                        j == i && id == &d.identity && shown == &d.definition && r == &c.request
                    })
                {
                    report.needs.push(format!(
                        "edge {i}: exact shown-definition default authorization required for {}@{}",
                        d.package, d.version
                    ));
                }
                if report.needs.len() == start {
                    viable = true;
                } else {
                    candidate_needs
                        .entry(format!("{}@{}", d.package.qualified_name(), d.version))
                        .or_default()
                        .extend_from_slice(&report.needs[start..]);
                }
            }
            // Candidate rejection remains the incumbent resolver's finite
            // search. An invalid newer alternative cannot prohibit a valid
            // older definition from satisfying the same retained intent.
            if viable {
                report.needs.truncate(edge_start);
            }
        }
    } else if old.configuration.is_some() {
        report
            .needs
            .push("configured consumer authority missing".into());
    }
    if !report.needs.is_empty() {
        report.decision = if unavailable {
            Decision::Unavailable
        } else {
            Decision::ReconfigurationRequired
        };
        return Ok(UpdatePlan { report, plan: None });
    }
    let plan = crate::plan::plan_prepared_mode(
        source, root, manifest, workspace, request, format, observer, true,
    )?;
    if let Plan::Ready(p) = &plan {
        let language = p
            .language_chain
            .first()
            .filter(|l| l.as_str() != "en")
            .map(String::as_str);
        let mut packages = Vec::new();
        for fetched in &p.fetched {
            let resolved = p
                .resolution
                .iter()
                .find(|r| {
                    r.group == fetched.cached.resolved.group
                        && r.name == fetched.cached.resolved.name
                })
                .ok_or_else(|| error("update unavailable: payload has no graph witness"))?;
            packages.push(crate::record::locked_package_from_fetched(
                fetched, resolved, language,
            ));
        }
        report.differences["packages"]["after"] =
            serde_json::to_value(packages).map_err(|e| error(e.to_string()))?;
        let after = crate::record::configuration_record(
            p.normalized_request.as_ref(),
            &p.selected_provenance,
            &p.fetched,
        )?;
        let after = after
            .as_ref()
            .ok_or_else(|| error("update unavailable: resolved evidence missing"))?;
        let (_, resolved) =
            vibe_wire::behaviour::lock_configuration::decode(after).map_err(error)?;
        report.differences["resolved"]["after"] =
            serde_json::to_value(resolved).map_err(|e| error(e.to_string()))?;
        for fetched in &p.fetched {
            let key = format!(
                "{}/{}@{}",
                fetched.cached.resolved.group,
                fetched.cached.resolved.name,
                fetched.cached.resolved.version
            );
            if let Some(needs) = candidate_needs.get(&key) {
                report.needs.extend_from_slice(needs);
            }
        }
        if !report.needs.is_empty() {
            report.decision = Decision::ReconfigurationRequired;
            return Ok(UpdatePlan { report, plan: None });
        }
    }
    report.decision = if report.differences["requested"]["before"]
        == report.differences["requested"]["after"]
        && report.differences["resolved"]["before"] == report.differences["resolved"]["after"]
        && report.differences["packages"]["before"] == report.differences["packages"]["after"]
    {
        Decision::ExactReuse
    } else {
        Decision::SemanticInputChange
    };
    Ok(UpdatePlan {
        report,
        plan: Some(plan),
    })
}

/// Plan an explicitly supplied new authored configuration. Unlike update,
/// this entrypoint is the caller's reconfiguration request; it never derives
/// intent or consent from the retained resolved state. The unchanged update
/// and exact replay paths continue to require retained intent.
pub fn plan_reconfiguration<S: InstallSource + ?Sized>(
    source: &S,
    root: &Path,
    request: InstallRequest,
    format: SpecFormat,
    observer: &dyn PlanObserver,
) -> crate::error::Result<crate::PlannedInstall> {
    if !request.roots.is_empty() {
        return Err(error(
            "reconfiguration cannot replace workspace declarations with CLI install roots",
        ));
    }
    if source
        .normalized_request()
        .and_then(|r| r.configuration.as_ref())
        .is_none()
    {
        let workspace = Workspace::discover(root)?;
        if workspace
            .iter_nodes()
            .any(|(_, node)| !node.consumer_configuration.is_empty())
        {
            let bound = crate::bind_authored_request(source, &workspace, &request.features)?;
            return plan_reconfiguration(&*bound, root, request, format, observer);
        }
    }
    let admitted = source
        .normalized_request()
        .filter(|r| r.configuration.is_some())
        .ok_or_else(|| {
            error(
                "NeedsSelection: reconfiguration requires current admitted authored configuration",
            )
        })?;
    let mut manifest = Manifest::read(root.join(Manifest::FILENAME))?;
    let mut workspace = Workspace::discover(root)?;
    let Plan::Ready(mut plan) = crate::plan::plan_prepared_mode(
        source,
        root,
        &mut manifest,
        &mut workspace,
        request,
        format,
        observer,
        true,
    )?
    else {
        return Err(error(
            "reconfiguration requires a complete selected desired world",
        ));
    };
    let canonical = |roots: &[vibe_core::PackageRef]| {
        let mut roots = roots.iter().map(ToString::to_string).collect::<Vec<_>>();
        roots.sort();
        roots
    };
    if plan.captured_catalog.is_none() {
        return Err(error("reconfiguration requires exact captured catalog"));
    }
    if canonical(&admitted.roots) != canonical(&plan.roots) {
        return Err(error(format!(
            "reconfiguration requires complete workspace root union: admitted={:?}, workspace={:?}",
            canonical(&admitted.roots),
            canonical(&plan.roots)
        )));
    }
    plan.authored_reconfiguration = true;
    Ok(*plan)
}
