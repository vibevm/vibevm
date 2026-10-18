use super::*;

impl BootDebuggerRoute {
    pub(super) fn dispatch(&self, state: &mut State, command: q::Command) -> Answer<r::Result> {
        use q::Command;
        match &command {
            Command::PreparationPlan(c) => return self.preparation(state, &c.plan_id, false, true),
            Command::Prepare(c) => return self.preparation(state, &c.plan_id, true, false),
            Command::PreparationRead(c) => {
                return self.preparation(state, &c.plan_id, false, false);
            }
            _ => {}
        }
        let id =
            match &command {
                Command::PreparationPlan(_) | Command::Prepare(_) | Command::PreparationRead(_) => {
                    unreachable!("handled above")
                }
                Command::Preview(c) => return self.preview(state, c),
                Command::Capture(_) => {
                    let capture = state.client.freeze().map_err(|_| {
                        fault(
                            e::ErrorCode::Unavailable,
                            "capture refused by document owner",
                        )
                    })?;
                    let id = capture.id().to_owned();
                    let snapshot = Arc::new(self.snapshot(capture));
                    let body = r::Body::Capture(Box::new(r::BodyCapture {
                        capture_id: id.clone(),
                        resources: snapshot.resources.clone(),
                    }));
                    state.snapshots.insert(id.clone(), snapshot);
                    return self.result(Some(id), body);
                }
                Command::Inspect(c) => &c.snapshot_id,
                Command::Tree(c) => &c.snapshot_id,
                Command::Explain(c) => &c.snapshot_id,
                Command::Cost(c) => &c.snapshot_id,
                Command::Compare(c) => &c.snapshot_id,
                Command::Chunk(c) => &c.snapshot_id,
                Command::Cancel(c) => {
                    let op = state.inspections.get_mut(&c.inspection_id).ok_or_else(|| {
                        fault(e::ErrorCode::UnknownInspection, "unknown inspection")
                    })?;
                    let terminal = op.terminal || op.stopped;
                    if !terminal {
                        op.stopped = true;
                        op.record(stream::Payload::State(Box::new(stream::PayloadState {
                            effect_state: stream::PayloadStateEffectState::NoEffects,
                            observed: stream::PayloadStateObserved::OwnerStopped,
                            result_ref: Some(op.result_id.clone()),
                        })))?;
                    }
                    return self.result(
                        Some(op.snapshot.clone()),
                        r::Body::Cancel(Box::new(r::BodyCancel {
                            inspection_id: c.inspection_id.clone(),
                            scope: r::BodyCancelScope::InspectionOnly,
                            build_cancelled: r::BodyCancelBuildCancelled::NotRequested,
                            status: if terminal {
                                r::BodyCancelStatus::AlreadyTerminal
                            } else {
                                r::BodyCancelStatus::ReachedSafeStop
                            },
                        })),
                    );
                }
                Command::Replay(c) => {
                    if !(1..=64).contains(&c.limit) {
                        return Err(fault(
                            e::ErrorCode::InvalidRequest,
                            "replay limit must be 1..64",
                        ));
                    }
                    let op = state.inspections.get(&c.inspection_id).ok_or_else(|| {
                        fault(e::ErrorCode::UnknownInspection, "unknown inspection")
                    })?;
                    let after = decimal(&c.after_sequence)?;
                    if after > op.sequence {
                        return Err(fault(
                            e::ErrorCode::StaleCursor,
                            "replay cursor exceeds stream tip",
                        ));
                    }
                    let oldest = op
                        .events
                        .front()
                        .map(|e| decimal(&e.sequence))
                        .transpose()?
                        .unwrap_or(op.sequence + 1);
                    let gap = (after + 1 < oldest).then(|| stream::Gap {
                        from_sequence: (after + 1).to_string(),
                        to_sequence: (oldest - 1).to_string(),
                        reason: stream::GapReason::Evicted,
                        terminal_ref: Some(op.result_id.clone()),
                    });
                    let events = op
                        .events
                        .iter()
                        .filter(|e| e.sequence.parse::<u64>().is_ok_and(|n| n > after))
                        .take(c.limit as usize)
                        .cloned()
                        .collect();
                    let message = stream::OperationStreamMessage {
                        schema: 1,
                        context_id: self.context.clone(),
                        target_id: None,
                        operation_id: OPERATION.into(),
                        run_id: c.inspection_id.clone(),
                        stream_id: c.inspection_id.clone(),
                        body: stream::Body::ReplayReply(Box::new(stream::BodyReplayReply {
                            after_sequence: c.after_sequence.clone(),
                            newest_sequence: Some(op.sequence.to_string()),
                            expired: false,
                            gap,
                            events,
                            terminal_ref: Some(op.result_id.clone()),
                        })),
                    };
                    stream_message::validate(&message, self.binding(&c.inspection_id))
                        .map_err(|_| internal())?;
                    return self.result(
                        Some(op.snapshot.clone()),
                        r::Body::Replay(Box::new(r::BodyReplay {
                            inspection_id: c.inspection_id.clone(),
                            stream_format_id: "machine-operation-stream-message".into(),
                            stream_json: json(&message)?,
                        })),
                    );
                }
            }
            .clone();
        let snapshot = state.snapshots.get(&id).cloned().ok_or_else(|| {
            fault(
                e::ErrorCode::UnknownSnapshot,
                "unknown or foreign immutable snapshot",
            )
        })?;
        let body = match command {
            Command::Inspect(_) => r::Body::Inspect(Box::new(r::BodyInspect {
                capture_id: id.clone(),
                resources: snapshot.resources.clone(),
                nodes: snapshot.nodes.clone(),
                sources: snapshot.sources.clone(),
                compiler_carrier_format_id: snapshot
                    .projection
                    .as_ref()
                    .and_then(|p| p.graph.carrier.as_ref())
                    .map(|_| "compiler-ir".into()),
                compiler_carrier_json: snapshot
                    .projection
                    .as_ref()
                    .and_then(|p| p.graph.carrier.as_ref())
                    .map(json)
                    .transpose()?,
                gaps: snapshot.gaps.clone(),
            })),
            Command::Explain(c) => {
                let chain = snapshot
                    .projection
                    .as_ref()
                    .and_then(|p| p.explain(c.contribution as usize, c.occurrence as usize));
                r::Body::Explain(Box::new(r::BodyExplain {
                    unavailable: chain
                        .is_none()
                        .then(|| "captured inclusion mapping unavailable".into()),
                    chain: chain
                        .unwrap_or_default()
                        .into_iter()
                        .map(|i| i as u32)
                        .collect(),
                }))
            }
            Command::Cost(_) => {
                let resource = snapshot
                    .capture
                    .resource(self.artifact)
                    .ok_or_else(internal)?;
                let payload = resource
                    .raw()
                    .map(measurement::measure_reader)
                    .transpose()
                    .map_err(|_| internal())?;
                let raw_bytes = payload
                    .as_ref()
                    .map(|p| exact(p.bytes))
                    .unwrap_or_else(|| unavailable("artifact bytes unavailable"));
                let scalars = match payload.map(|p| p.scalars) {
                    Some(measurement::Count::Exact { value }) => exact(value),
                    _ => unavailable("captured artifact is absent or not UTF8"),
                };
                r::Body::Cost(Box::new(r::BodyCost {
                    recipe: format!(
                        "{}; selected captured artifact only, not total boot or authored-source attribution",
                        measurement::RECIPE
                    ),
                    raw_bytes,
                    scalars,
                    tokens: unavailable("no tokenizer selected or executed"),
                    provider_billing: unavailable("provider billing not observed"),
                    ambient_tail: unavailable("unknown: ambient context not observed"),
                }))
            }
            Command::Compare(c) => {
                let other = state.snapshots.get(&c.other_snapshot_id).ok_or_else(|| {
                    fault(
                        e::ErrorCode::UnknownSnapshot,
                        "comparison snapshot not retained in this context",
                    )
                })?;
                // A successful M16 freeze retains bytes or proved absence; read
                // failures refuse freeze, so no unknown state becomes absence.
                let a: BTreeMap<_, _> = snapshot
                    .resources
                    .iter()
                    .filter_map(|r| r.digest.as_ref().map(|digest| (&r.path, digest)))
                    .collect();
                let b: BTreeMap<_, _> = other
                    .resources
                    .iter()
                    .filter_map(|r| r.digest.as_ref().map(|digest| (&r.path, digest)))
                    .collect();
                r::Body::Compare(Box::new(r::BodyCompare {
                    other_snapshot_id: c.other_snapshot_id,
                    added: b
                        .keys()
                        .filter(|k| !a.contains_key(*k))
                        .map(|s| (*s).clone())
                        .collect(),
                    removed: a
                        .keys()
                        .filter(|k| !b.contains_key(*k))
                        .map(|s| (*s).clone())
                        .collect(),
                    changed: a
                        .iter()
                        .filter(|(k, v)| b.get(*k).is_some_and(|w| w != *v))
                        .map(|(k, _)| (*k).clone())
                        .collect(),
                }))
            }
            Command::Chunk(c) => {
                if !(1..=2048).contains(&c.limit) {
                    return Err(fault(
                        e::ErrorCode::InvalidRequest,
                        "chunk limit must be 1..2048",
                    ));
                }
                let i = snapshot
                    .resources
                    .iter()
                    .position(|r| r.resource_id == c.resource_id)
                    .ok_or_else(|| {
                        fault(e::ErrorCode::UnknownResource, "resource not in snapshot")
                    })?;
                let (_, resource) = snapshot.capture.resources().nth(i).ok_or_else(internal)?;
                let bytes = resource.raw().ok_or_else(|| {
                    fault(e::ErrorCode::Unavailable, "captured bytes unavailable")
                })?;
                let offset = usize::try_from(decimal(&c.offset)?)
                    .map_err(|_| fault(e::ErrorCode::InvalidRequest, "offset overflow"))?;
                if offset > bytes.len() {
                    return Err(fault(
                        e::ErrorCode::StaleCursor,
                        "chunk offset exceeds captured bytes",
                    ));
                }
                let end = offset.saturating_add(c.limit as usize).min(bytes.len());
                r::Body::Chunk(Box::new(r::BodyChunk {
                    resource_id: c.resource_id,
                    digest: resource.content_digest().ok_or_else(internal)?,
                    basis: r::BodyChunkBasis::OriginalBytes,
                    offset: c.offset,
                    total: bytes.len().to_string(),
                    hex_bytes: bytes[offset..end]
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect(),
                    next_offset: (end < bytes.len()).then(|| end.to_string()),
                }))
            }
            Command::Tree(c) => {
                if !(1..=1024).contains(&c.limit)
                    || c.filter.len() > 1024
                    || c.filter.chars().any(char::is_control)
                {
                    return Err(fault(
                        e::ErrorCode::InvalidRequest,
                        "invalid page limit or path-prefix filter",
                    ));
                }
                let inspection_id = if let Some(i) = c.inspection_id {
                    i
                } else {
                    if c.cursor.is_some() {
                        return Err(fault(
                            e::ErrorCode::StaleCursor,
                            "cursor without inspection",
                        ));
                    }
                    let inspection_id = format!("{}:inspection:{}", self.context, state.next);
                    state.next = state.next.checked_add(1).ok_or_else(internal)?;
                    state.inspections.insert(
                        inspection_id.clone(),
                        self.pages(&snapshot, &id, &c.filter)?,
                    );
                    inspection_id
                };
                let op = state
                    .inspections
                    .get_mut(&inspection_id)
                    .ok_or_else(|| fault(e::ErrorCode::UnknownInspection, "unknown inspection"))?;
                if op.snapshot != id || op.filter != c.filter {
                    return Err(fault(
                        e::ErrorCode::MixedFilter,
                        "inspection snapshot/filter mismatch",
                    ));
                }
                if op.stopped {
                    return Err(fault(
                        e::ErrorCode::Cancelled,
                        "inspection stopped; build was not cancelled",
                    ));
                }
                let answer = op
                    .pages
                    .next_page(PageSessionRequest {
                        projection: text(&op.projection),
                        filter: text(&filter_identity(&op.filter)),
                        authorized_scope: text(&self.context),
                        axis_stamps: op.stamps.clone(),
                        cursor_token: c.cursor,
                        limit: c.limit as usize,
                    })
                    .map_err(|_| {
                        fault(
                            e::ErrorCode::StaleCursor,
                            "page owner refused cursor or terminal inspection",
                        )
                    })?;
                op.delivered += answer.value.rows.len() as u64;
                op.record(stream::Payload::Progress(Box::new(
                    stream::PayloadProgress {
                        current: op.delivered.to_string(),
                        total: None,
                        unit: stream::PayloadProgressUnit::WorkUnits,
                    },
                )))?;
                op.terminal = answer.value.next_cursor_id.is_none();
                if op.terminal {
                    op.record(stream::Payload::State(Box::new(stream::PayloadState {
                        observed: stream::PayloadStateObserved::ResultAvailable,
                        effect_state: stream::PayloadStateEffectState::NoEffects,
                        result_ref: Some(op.result_id.clone()),
                    })))?;
                }
                r::Body::Tree(Box::new(r::BodyTree {
                    inspection_id,
                    page_format_id: "machine-bound-page".into(),
                    page_json: json(&answer.value)?,
                }))
            }
            _ => return Err(internal()),
        };
        self.result(Some(id), body)
    }
    pub(super) fn binding<'a>(&'a self, id: &'a str) -> ExpectedBinding<'a> {
        ExpectedBinding {
            context_id: &self.context,
            target_id: None,
            operation_id: OPERATION,
            run_id: id,
            stream_id: id,
        }
    }
    fn pages(&self, snapshot: &Snapshot, id: &str, filter: &str) -> Answer<Inspection> {
        let projection = hash(json(&(id, filter, "boot-tree/1"))?.as_bytes());
        let composite = CompositeSnapshot::compose([
            AxisParticipation::Required(Some(AxisWitness {
                owner: text(OPERATION),
                capture: text(id),
                revision: text(id),
                observation: text(id),
                compatibility_claim: None,
                expected_revisions: vec![],
                freshness: AxisFreshness::DeclaredCurrent,
                completeness: AxisCompleteness::DeclaredComplete,
            })),
            AxisParticipation::NotParticipating,
            AxisParticipation::NotParticipating,
            AxisParticipation::NotParticipating,
        ])
        .map_err(|_| internal())?;
        let mut rows = vec![];
        let mut catalogue = vec![];
        for n in snapshot.nodes.iter().filter(|n| n.path.starts_with(filter)) {
            let occurrence = format!("{}:{}", n.contribution, n.occurrence);
            let reference = format!("{projection}:{occurrence}");
            let observed = entity::Observation {
                projection: projection.clone(),
                observation: id.into(),
            };
            catalogue.push(entity::EntityReference {
                schema: 1,
                reference_id: reference.clone(),
                logical: entity::Logical::ExtractionGap(Box::new(entity::LogicalExtractionGap {
                    reason: "captured occurrence is not a minted logical address".into(),
                })),
                captured: entity::Captured {
                    content_or_source: id.into(),
                    revision: id.into(),
                },
                observed: observed.clone(),
                occurrence: entity::Occurrence {
                    observed,
                    occurrence_id: occurrence.clone(),
                    contribution: n.contribution.to_string(),
                    ordinal: n.occurrence.to_string(),
                },
            });
            // Incumbent graph order may not be parent-first. Keep the captured
            // relation in inspect/explain; do not fabricate a page hierarchy.
            rows.push(page::Row {
                entity_reference_id: reference,
                occurrence_id: occurrence,
                parent_occurrence_id: None,
                child_state: page::ChildState::Unavailable(Box::new(page::ChildStateUnavailable {
                    reason: "flat occurrence projection; use captured explain relation".into(),
                })),
            });
        }
        let pages = PageSession::new(
            &composite,
            text(&projection),
            text(&filter_identity(filter)),
            text(&self.context),
            rows,
            catalogue,
            vec![SourceCaptureBinding {
                content_or_source: text(id),
                revision: text(id),
                observation: text(id),
                selected_capture: text(id),
            }],
            ProjectionExtent::Partial {
                reason: text("captured graph mappings may be unavailable"),
            },
        )
        .map_err(|_| internal())?;
        Ok(Inspection {
            snapshot: id.into(),
            filter: filter.into(),
            projection: projection.clone(),
            stamps: composite.stamps().to_vec(),
            pages,
            stopped: false,
            terminal: false,
            sequence: 0,
            delivered: 0,
            events: VecDeque::new(),
            result_id: projection,
        })
    }
}
fn filter_identity(filter: &str) -> String {
    hash(filter.as_bytes())
}
impl Inspection {
    fn record(&mut self, payload: stream::Payload) -> Answer<()> {
        self.sequence = self.sequence.checked_add(1).ok_or_else(internal)?;
        self.events.push_back(stream::EventRecord {
            sequence: self.sequence.to_string(),
            payload,
        });
        while self.events.len() > stream_message::MAX_REPLAY_EVENTS {
            self.events.pop_front();
        }
        Ok(())
    }
}
