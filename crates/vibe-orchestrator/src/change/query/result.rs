//! Result assembly consumes existing captured/metadata/work owners only.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-READ-OBSERVATION-AXES");
use super::*;
use vibe_core::manifest::change::ChangeDocumentRole as D;
use vibe_specdoc::captured::StructuredView;
use wire::*;
fn gap(scope: GapScope, address: impl Into<String>, code: impl Into<String>) -> Gap {
    Gap {
        scope,
        address: address.into(),
        code: code.into(),
    }
}
fn convert<T: serde::de::DeserializeOwned>(value: &impl serde::Serialize) -> Result<T, Response> {
    serde_json::from_value(
        serde_json::to_value(value).map_err(|_| {
            wire::refused(Code::InternalInvariant, "metadata serialization", vec![])
        })?,
    )
    .map_err(|_| wire::refused(Code::InternalInvariant, "owned metadata shape", vec![]))
}
fn role_name(role: D) -> &'static str {
    match role {
        D::Intent => "intent",
        D::Refinement => "refinement",
        D::Acceptance => "acceptance",
        D::Work => "work",
        D::Closure => "closure",
    }
}
pub(super) fn produce(
    service: &CapturedChangeQuery,
    request: &Request,
) -> Result<Response, Response> {
    let view = super::super::analyze_change(
        &service.capture,
        service.node,
        service.spec_root.as_deref(),
        &service.id,
    );
    let parsed = view.analysis().map_err(|_| {
        wire::refused(
            Code::InvalidChange,
            "captured change grammar/inputs unavailable",
            vec![],
        )
    })?;
    let manifest = &parsed.manifest;
    let requested: Vec<_> = request
        .roles
        .iter()
        .map(|role| {
            serde_json::to_value(role)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    let available: Vec<_> = manifest
        .documents()
        .iter()
        .map(|d| role_name(d.role).to_string())
        .collect();
    for role in &requested {
        if !available.contains(role) {
            return Err(wire::refused(
                Code::UnknownRole,
                "role is not declared by captured change",
                available,
            ));
        }
    }
    let node = service
        .capture
        .manifest(service.node)
        .map_err(|_| wire::refused(Code::UnavailableContext, "node capture unavailable", vec![]))?;
    let context = ChangeContext::for_node(&node, ".", service.spec_root.as_deref())
        .map_err(|_| wire::refused(Code::UnavailableContext, "node context unavailable", vec![]))?;
    let id = ChangeId::parse(&service.id)
        .map_err(|e| wire::refused(Code::InvalidId, e.to_string(), vec![]))?;
    let mut roles = vec![];
    let mut gaps = vec![];
    let mut facts = vec![];
    let mut rows = vec![];
    let mut local_authored = vec![];
    let mut fact_witnesses = BTreeMap::new();
    let mut omitted_roles = vec![];
    for doc in manifest.documents() {
        let name = role_name(doc.role);
        if !requested.is_empty() && !requested.iter().any(|r| r == name) {
            omitted_roles.push(doc.path.clone());
            continue;
        }
        let path = manifest.directory().join(&doc.path);
        let path_text = path.to_str().unwrap();
        let resource = service.host.resource(path_text).ok_or_else(|| {
            wire::refused(
                Code::InternalInvariant,
                "role declaration not captured",
                vec![],
            )
        })?;
        let row = service.capture.resource(resource).unwrap();
        let raw = row.raw().ok_or_else(|| {
            wire::refused(
                Code::MissingChange,
                "declared role bytes unavailable",
                vec![doc.path.clone()],
            )
        })?;
        let address = context
            .document_address(&id, &doc.path)
            .map_err(|_| wire::refused(Code::InvalidChange, "role address refused", vec![]))?;
        let mut state = RoleState::Unclassified;
        if doc.role == D::Work {
            state = RoleState::Work;
        } else if doc.role == D::Closure {
            state = RoleState::Inert;
            gaps.push(gap(
                GapScope::Local,
                &address,
                "derived-closure-out-of-scope",
            ));
        } else {
            match service.capture.inspect(resource) {
                Ok(preview) => match preview.inspection.view {
                    Some(StructuredView::Vibe { ir, .. }) => {
                        state = RoleState::Native;
                        for fact in vibe_facts::authored_facts(&ir, &format!("{address}#")) {
                            fact_witnesses.insert(
                                fact.address.clone(),
                                (path_text.to_string(), row.content_digest().unwrap()),
                            );
                            local_authored.push(fact);
                        }
                    }
                    Some(StructuredView::Opaque) => {
                        gaps.push(gap(GapScope::Local, &address, "unclassified-local-facts"))
                    }
                    Some(StructuredView::Commonmark(_) | StructuredView::XmlData(_)) => {
                        state = RoleState::Inert;
                        gaps.push(gap(GapScope::Local, &address, "inert-local-facts"));
                    }
                    None => {
                        state = RoleState::Invalid;
                        gaps.push(gap(
                            GapScope::Local,
                            &address,
                            "invalid-local-structured-view",
                        ));
                    }
                },
                Err(_) => {
                    state = RoleState::Unavailable;
                    gaps.push(gap(
                        GapScope::Local,
                        &address,
                        "local-classification-unavailable",
                    ));
                }
            }
        }
        roles.push(Role {
            role: serde_json::from_value(serde_json::json!(name)).unwrap(),
            path: path_text.into(),
            address,
            sha256: row.content_digest().unwrap(),
            bytes: u32::try_from(raw.len())
                .map_err(|_| wire::refused(Code::InternalInvariant, "role byte bound", vec![]))?,
            buffer_version: row.buffer_version().map(|n| n.to_string()),
            basis: RoleBasis::OriginalBytes,
            state,
            raw_hex: if request.mode == RequestMode::Show {
                Some(raw.iter().map(|b| format!("{b:02x}")).collect())
            } else {
                None
            },
            raw_recipe: RoleRawRecipe::Hex,
        });
    }
    let local_rows = vibe_requirements::change::change_fact_rows(
        &service.registry.registry,
        manifest.coordinate(),
        false,
        &local_authored,
    )
    .map_err(|_| {
        wire::refused(
            Code::InternalInvariant,
            "local metadata join refused",
            vec![],
        )
    })?;
    for row in &local_rows {
        let (document, sha256) = &fact_witnesses[&row.address];
        facts.push(Fact {
            scope: FactScope::Local,
            metadata: convert(row)?,
            document: document.clone(),
            sha256: sha256.clone(),
            capture: service.capture.id().into(),
            basis: FactBasis::CapturedOriginalBytes,
            numeric_position_unavailable: true,
        });
    }
    rows.extend(local_rows);
    let mut origins = vec![];
    let mut provider_sources = vec![];
    let mut provider_roots = vec![];
    for (index, declared) in manifest.origins().iter().enumerate() {
        let addresses: Vec<_> = declared
            .documents
            .iter()
            .map(|d| d.address.clone())
            .collect();
        let mut origin = Origin {
            package: declared.package.clone(),
            declared_content: declared.selected_content_hash.clone(),
            observed_content: None,
            capture: None,
            state: OriginState::Unavailable,
            selected_addresses: addresses.clone(),
            observed_addresses: vec![],
            source_strengths: BTreeMap::new(),
        };
        if let Some(source) = service.origins.get(&declared.package) {
            origin.observed_content = Some(source.inventory().content_hash().as_str().into());
            origin.capture = Some(source.capture_id().into());
            match parsed.select_origin_facts(index, source) {
                Ok(selection) => {
                    for g in selection.gaps() {
                        gaps.push(gap(
                            GapScope::Origin,
                            &g.address,
                            format!("selected-fact-{:?}", g.kind),
                        ));
                    }
                    for g in selection.capture_gaps() {
                        gaps.push(gap(
                            GapScope::Origin,
                            &declared.package,
                            format!("captured-source-{:?}", g),
                        ));
                    }
                    let selected = selection.observed();
                    let authored: Vec<_> = selected.iter().map(|f| f.authored.clone()).collect();
                    let pin = source.inventory().pin();
                    let coordinate = format!("{}/{}", pin.group.as_ref().unwrap(), pin.name);
                    let origin_rows = vibe_requirements::change::change_fact_rows(
                        &service.registry.registry,
                        &coordinate,
                        true,
                        &authored,
                    )
                    .map_err(|_| {
                        wire::refused(
                            Code::InternalInvariant,
                            "origin metadata join refused",
                            vec![],
                        )
                    })?;
                    for row in &origin_rows {
                        let f = selected
                            .iter()
                            .find(|f| f.authored.address == row.address)
                            .unwrap();
                        facts.push(Fact {
                            scope: FactScope::Origin,
                            metadata: convert(row)?,
                            document: f.document.path.clone(),
                            sha256: f.document.digest.clone(),
                            capture: f.capture_id.clone(),
                            basis: FactBasis::CapturedOriginalBytes,
                            numeric_position_unavailable: true,
                        });
                    }
                    rows.extend(origin_rows);
                    origin.observed_addresses =
                        authored.iter().map(|f| f.address.clone()).collect();
                    origin.selected_addresses.sort();
                    origin.observed_addresses.sort();
                    origin.state = if selection.is_complete() {
                        OriginState::Matched
                    } else {
                        OriginState::Unavailable
                    };
                    for f in selected {
                        let doc = &source.inventory().documents()[&f.document.path];
                        origin.source_strengths.insert(
                            f.document.path.clone(),
                            match doc.provenance().strength {
                                vibe_workspace::source_query::SourceStrength::Observed => {
                                    "observed"
                                }
                                vibe_workspace::source_query::SourceStrength::RecordConsistent => {
                                    "record-consistent"
                                }
                                vibe_workspace::source_query::SourceStrength::SourceProven => {
                                    "source-proven"
                                }
                            }
                            .into(),
                        );
                    }
                }
                Err(error) => {
                    origin.state = if matches!(
                        error,
                        vibe_requirements::change::ChangeFactSelectionError::Unbound
                    ) {
                        OriginState::Unbound
                    } else {
                        OriginState::Drift
                    };
                    gaps.push(gap(
                        GapScope::Origin,
                        &declared.package,
                        format!("origin-binding-{:?}", error),
                    ));
                }
            }
            let package = vibe_core::manifest::Manifest::parse_str(
                std::str::from_utf8(&source.inventory().manifest().bytes).unwrap(),
            )
            .unwrap()
            .package
            .unwrap();
            provider_roots.push((
                format!("{}/{}", package.group, package.name),
                vibe_workspace::vibedeps::slot_abs_path(
                    &service.workspace_root,
                    &package.group,
                    &package.name,
                    &package.version,
                ),
                source.inventory().content_hash().as_str().to_string(),
            ));
        } else {
            gaps.push(gap(
                GapScope::Origin,
                &declared.package,
                "retained-selected-source-unavailable",
            ));
        }
        origins.push(origin);
    }
    provider_sources.push(vibe_requirements::ProviderSource {
        kind: vibe_wire::generated::requirements_report::RequirementSourceKind::Host,
        package: manifest.coordinate(),
        root: Some(&service.root),
        expected_content_hash: None,
    });
    for (coordinate, path, hash) in &provider_roots {
        provider_sources.push(vibe_requirements::ProviderSource {
            kind: vibe_wire::generated::requirements_report::RequirementSourceKind::Package,
            package: coordinate,
            root: Some(path),
            expected_content_hash: Some(hash),
        });
    }
    rows.sort_by(|a, b| a.address.cmp(&b.address));
    let addresses: Vec<_> = rows.iter().map(|r| r.address.clone()).collect();
    let provider = vibe_trace::SpecmapRelationProvider;
    let requested = request.mode == RequestMode::Coverage;
    let enrichment = vibe_requirements::change::change_relations(
        &mut rows,
        requested,
        if requested { Some(&provider) } else { None },
        &vibe_requirements::RelationRequest {
            selected_root: &service.root,
            workspace_root: &service.workspace_root,
            sources: &provider_sources,
            addresses: &addresses,
        },
    )
    .map_err(|_| {
        wire::refused(
            Code::InternalInvariant,
            "relation observation refused",
            vec![],
        )
    })?;
    for fact in &mut facts {
        if let Some(row) = rows.iter().find(|row| row.address == fact.metadata.address) {
            fact.metadata = convert(row)?;
        }
    }
    facts.sort_by(|a, b| a.metadata.address.cmp(&b.metadata.address));
    let work_items = parsed
        .identities
        .graph
        .items()
        .map(|item| WorkItem {
            id: item.id.clone(),
            predecessors: item.predecessors.clone(),
            touches: item.touches.clone(),
            needs: item
                .needs
                .iter()
                .map(|need| Need {
                    kind: serde_json::from_value(serde_json::json!(need.kind.as_str())).unwrap(),
                    target: need.target.clone(),
                })
                .collect(),
        })
        .collect();
    let scope_complete = gaps.is_empty() && origins.iter().all(|o| o.state == OriginState::Matched);
    let registry_witnesses = service
        .registry
        .witnesses
        .iter()
        .map(|w| Witness {
            path: w.path.clone(),
            sha256: w.digest.clone(),
            bytes: u32::try_from(w.bytes).unwrap_or(u32::MAX),
        })
        .collect();
    service.revalidate().map_err(|_| {
        wire::refused(
            Code::StaleObservation,
            "captured read inputs changed during observation",
            vec![],
        )
    })?;
    Ok(Response::Success(Box::new(ResponseSuccess {
        schema: 1,
        mode: convert(&request.mode)?,
        id: service.id.clone(),
        title: manifest.title().into(),
        slug: manifest.slug().into(),
        coordinate: manifest.coordinate().into(),
        selected_node: manifest.selected_node().into(),
        directory: manifest.directory().to_str().unwrap().into(),
        intent_recipe: parsed.identities.intent.recipe().into(),
        intent_hash: parsed.identities.intent.digest().into(),
        work_recipe: parsed.identities.work.recipe().into(),
        work_hash: parsed.identities.work.digest().into(),
        capture: service.capture.id().into(),
        roles,
        omitted_roles,
        origins,
        facts,
        work_items,
        registry_witnesses,
        relation_sources: enrichment.iter().map(convert).collect::<Result<_, _>>()?,
        gaps,
        scope_complete,
        terminal: ResponseSuccessTerminal::NotEvaluated,
        artifact_evidence: ResponseSuccessArtifactEvidence::NotObserved,
        source_authentication: ResponseSuccessSourceAuthentication::NotEstablished,
    })))
}
