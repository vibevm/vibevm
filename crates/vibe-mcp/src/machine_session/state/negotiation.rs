//! Versioned session negotiation and request-envelope admission.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SESSION-NEGOTIATION");

use super::*;
impl MachineSession {
    fn reply(&self, request_id: Option<String>, body: Body) -> SessionDecision {
        SessionDecision::Reply(SessionFrame {
            schema: 1,
            session_id: (self.phase == SessionPhase::Initialized).then(|| self.session_id.clone()),
            request_id,
            body,
        })
    }

    fn refused(&self, request_id: Option<String>, reason: BodyRefusalReason) -> SessionDecision {
        self.reply(
            request_id,
            Body::Refusal(Box::new(BodyRefusal {
                reason,
                envelope_json: None,
                outcome: Outcome {
                    effect_state: OutcomeEffectState::NoEffects,
                    completeness: OutcomeCompleteness::Unavailable,
                    provenance_id: None,
                },
            })),
        )
    }

    fn initialization_refused(
        &self,
        request_id: Option<String>,
        reason: BodyInitializationRefusedReason,
    ) -> SessionDecision {
        SessionDecision::Reply(SessionFrame {
            schema: 1,
            session_id: None,
            request_id,
            body: Body::InitializationRefused(Box::new(BodyInitializationRefused {
                reason,
                supported_api_versions: vec![API_VERSION.to_owned()],
                supported_schemas: self.schemas.clone(),
                supported_position_encodings: vec![POSITION_ENCODING.to_owned()],
            })),
        })
    }

    /// Initialize or check outer assigned IDs only. A successful request is
    /// explicitly unvalidated and must pass to its separate domain owner.
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-080#MACHINE-SESSION-NEGOTIATION",
        r = 1
    )]
    pub fn receive(&mut self, frame: SessionFrame) -> SessionDecision {
        let SessionFrame {
            schema,
            session_id,
            request_id,
            body,
        } = frame;
        if schema != 1 {
            return self.refused(request_id, BodyRefusalReason::UnsupportedVersion);
        }
        if self.phase == SessionPhase::Closed {
            return self.refused(request_id, BodyRefusalReason::Unavailable);
        }
        match body {
            Body::InitializeOffer(offer) => {
                if self.phase != SessionPhase::Uninitialized
                    || session_id.is_some()
                    || !request_id.as_deref().is_some_and(valid_id)
                {
                    return self.initialization_refused(
                        request_id,
                        BodyInitializationRefusedReason::InvalidOffer,
                    );
                }
                if !offer.api_versions.iter().any(|value| value == API_VERSION) {
                    return self.initialization_refused(
                        request_id,
                        BodyInitializationRefusedReason::NoCommonApiVersion,
                    );
                }
                if self
                    .schemas
                    .iter()
                    .filter(|required| {
                        required.format_id != FormatId::MachineProfiledDeployActionRequest.id()
                    })
                    .any(|required| !offer.schema_offers.contains(required))
                {
                    return self.initialization_refused(
                        request_id,
                        BodyInitializationRefusedReason::NoCommonSchema,
                    );
                }
                if !offer
                    .position_encodings
                    .iter()
                    .any(|value| value == POSITION_ENCODING)
                {
                    return self.initialization_refused(
                        request_id,
                        BodyInitializationRefusedReason::NoCommonPositionEncoding,
                    );
                }
                let Some(request_id) = request_id else {
                    return self.initialization_refused(
                        None,
                        BodyInitializationRefusedReason::InvalidOffer,
                    );
                };
                if !self.reserve_request_id(&request_id) {
                    return self.initialization_refused(
                        Some(request_id),
                        BodyInitializationRefusedReason::InvalidOffer,
                    );
                }
                self.selected_schemas = self
                    .schemas
                    .iter()
                    .filter(|schema| offer.schema_offers.contains(schema))
                    .cloned()
                    .collect();
                self.phase = SessionPhase::Initialized;
                self.reply(
                    Some(request_id),
                    Body::Initialized(Box::new(BodyInitialized {
                        api_version: API_VERSION.to_owned(),
                        selected_schemas: self.selected_schemas.clone(),
                        position_encoding: POSITION_ENCODING.to_owned(),
                        actual_capabilities: self.capabilities.clone(),
                        contexts: self
                            .contexts
                            .values()
                            .map(|bound| bound.descriptor.clone())
                            .collect(),
                        legacy_mcp_observed_version: PROTOCOL_VERSION.to_owned(),
                    })),
                )
            }
            Body::Request(request) => self.receive_request(session_id, request_id, *request),
            _ => self.refused(request_id, BodyRefusalReason::InvalidRequest),
        }
    }

    fn receive_request(
        &mut self,
        session_id: Option<String>,
        request_id: Option<String>,
        request: BodyRequest,
    ) -> SessionDecision {
        if self.phase != SessionPhase::Initialized {
            return self.refused(request_id, BodyRefusalReason::NotInitialized);
        }
        if session_id.as_deref() != Some(&self.session_id)
            || !request_id.as_deref().is_some_and(valid_id)
        {
            return self.refused(request_id, BodyRefusalReason::InvalidRequest);
        }
        let Some(request_id) = request_id else {
            return self.refused(None, BodyRefusalReason::InvalidRequest);
        };
        if !self.reserve_request_id(&request_id) {
            return self.refused(Some(request_id), BodyRefusalReason::InvalidRequest);
        }
        let Some(bound) = self.contexts.get(&request.context_id) else {
            return self.refused(Some(request_id), BodyRefusalReason::UnknownContext);
        };
        if bound.descriptor.availability != ContextAvailability::Available {
            return self.refused(Some(request_id), BodyRefusalReason::Unavailable);
        }
        if (self.require_target && request.target_id.is_none())
            || request
                .target_id
                .as_ref()
                .is_some_and(|id| !bound.descriptor.target_ids.contains(id))
        {
            return self.refused(Some(request_id), BodyRefusalReason::UnknownTarget);
        }
        if let Ok(envelope) = serde_json::from_str::<OperationEnvelope>(&request.envelope_json)
            && let EnvelopeBody::Input(input) = envelope.body
            && self
                .schemas
                .iter()
                .any(|schema| schema.format_id == input.format_id)
            && !self
                .selected_schemas
                .iter()
                .any(|schema| schema.format_id == input.format_id)
        {
            return self.refused(Some(request_id), BodyRefusalReason::UnsupportedVersion);
        }
        SessionDecision::UnvalidatedRequest(UnvalidatedRequest {
            session_id: self.session_id.clone(),
            request_id,
            context_id: request.context_id,
            target_id: request.target_id,
            envelope_json: request.envelope_json,
        })
    }
}
