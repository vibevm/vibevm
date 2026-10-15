fn require_criterion_decisions(review:&serde_json::Value,candidate:&CandidateResult)->Result<()> {
 let raw=serde_json::to_value(&candidate.criteria)?;let original=raw.as_array().ok_or("candidate criteria")?;let decisions=review["criterion_assessments"].as_array().ok_or("Root criterion decisions required")?;
 if decisions.len()!=original.len(){return Err("Root must review every original criterion".into())}
 for c in original {let d=decisions.iter().find(|d|d["requirement"]==c["requirement"]).ok_or("Root criterion missing")?;if d["producer_disposition"]!=c["disposition"]||d["review_disposition"]!="satisfied"||d["rationale"].as_str().is_none_or(|s|s.trim().is_empty()){return Err("Root independent criterion rationale missing/misstates original".into())}}
 Ok(())
}
fn require_retirement(root:&std::path::Path)->Result<()> {
 let v:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("ROOT-RETIREMENT.json"))?)?;
 let after=v["after"].as_array().ok_or("Root retirement after-observations missing")?;
 if after.len()!=5{return Err("Root exact original five-process retirement required".into())}
 let mut ids=std::collections::BTreeSet::new();
 for expected in [4100565u64,4100569,4100677,4100703] {if !after.iter().any(|row|row["pid"].as_u64()==Some(expected)){return Err("Retirement does not bind original M08B1 tree".into())}}
 for row in after{let pid=row["pid"].as_u64().ok_or("retired PID")?;if pid==0||!ids.insert(pid)||row["exists"]!=false||std::path::Path::new(&format!("/proc/{pid}")).exists(){return Err("Root retirement not exact/absent now".into())}}
 Ok(())
}
use zap_app::{ApplicationService,ApplicationStoreMode,ManagedRuntimeConfig};
use zap_api::*;use zap_core::*;use zap_wire::*;
use zap_domain::{acceptance::*,control::{WorkTransitioned,WorkTransitionedSchema,work_transition_basis_request},seams::{WorkState,EvidenceApplicability,EvidenceDisposition,EvidenceObservation,EvidenceResult,SourceCapture,VerificationMethod}};
use zap_domain::seams::MaturityStage;
use std::{path::Path,collections::BTreeSet};
type Result<T>=std::result::Result<T,Box<dyn std::error::Error>>;
#[derive(serde::Deserialize)]#[serde(deny_unknown_fields)]struct Review{approved:bool,summary:BoundedText<4096>,scope:BoundedText<4096>,limitations:Vec<BoundedText<4096>>,observations:serde_json::Value,fresh_distinct_cases:u32,reused_distinct_cases:u32,actual_environment:BoundedText<4096>,evidence_files:serde_json::Value,criterion_assessments:serde_json::Value}
fn root()->&'static Path{Path::new("/fast/zap-managed/2026-10-15-m08b1-review-correction")}
fn local(app:&ApplicationService,request:&MachineRequest)->Result<MachineResponse>{
 match request {
  MachineRequest::RuntimeInspect{request}=>Ok(MachineResponse::Runtime(app.runtime_inspect(request)?)),
  MachineRequest::PrepareEffectBundle{request}=>Ok(MachineResponse::PreparedEffectBundle(app.prepare_effect_bundle(request.clone())?)),
  MachineRequest::Command{command}=>{
   let mut secret=std::fs::read(root().join("service/coordinator.secret"))?;
   let result=app.submit_credential(&CredentialId::parse("coordinator.vibevm-m08b1-review-correction")?,&secret,command.canonical()?);secret.fill(0);
   Ok(MachineResponse::Command(result?))
  },
  _=>Err("unsupported local operator operation".into())
 }
}
fn claim()->Result<RuntimeJobClaimRecord>{Ok(serde_json::from_slice(&std::fs::read(root().join("ready-capture.json"))?)?)}
fn outcome_id()->Result<OutcomeId>{let p:zap_domain::initial_planning::InitialPlanCandidate=serde_json::from_slice(&std::fs::read(root().join("actual-candidate.json"))?)?;Ok(p.outcome.outcome_id)}
fn payload<T:serde::Serialize>(value:&T)->Result<QueryInput>{Ok(QueryInput{codec:CodecEpoch::CURRENT,canonical_json:CanonicalOutput::encode_json(CodecEpoch::CURRENT,value)?.as_bytes().to_vec()})}
fn job(app:&ApplicationService)->Result<zap_runtime::RuntimeJobRecord>{
 let response=local(app,&MachineRequest::RuntimeInspect{request:RuntimeInspectRequest{job_id:Some(claim()?.job_id),limit:1}})?;
 let MachineResponse::Runtime(view)=response else{return Err("unexpected runtime inspection response".into())};
 let mut jobs:Vec<zap_runtime::RuntimeJobRecord>=serde_json::from_slice(&view.detail)?;if jobs.len()!=1{return Err("exact job absent".into())}Ok(jobs.remove(0))
}
fn basis(app:&ApplicationService,phase:&str,request:BasisRequest)->Result<PreparedEffectBundleView>{
 let response=local(app,&MachineRequest::PrepareEffectBundle{request:PrepareBundleRequest{at:PreparationRead::Current,actor:None,draft:EffectBundleDraftInput{alternative_id:ChangeAlternativeId::parse(&format!("review-basis.m08b1.{phase}"))?,committed_prefix:vec![],effects:vec![],no_op_basis:Some(request)}}})?;
 let MachineResponse::PreparedEffectBundle(view)=response else{return Err("unexpected exact basis response".into())};Ok(view)
}
fn send<P:CommandPayload+serde::Serialize>(app:&ApplicationService,phase:&str,value:&P,basis_request:BasisRequest,review:&Review)->Result<()> {
 let current=basis(app,phase,basis_request)?;let command_id=CommandId::parse(&format!("root-review.m08b1.{phase}"))?;
 let command=ProtectedCommand{frame:CommandFrameInput{header:CommandHeader::new(CommandHeaderInput{protocol:ProtocolEpoch::new(2)?,store_id:current.store.store_id,campaign_id:current.store.campaign_id,base_id:current.store.base_id,command_id:command_id.clone(),event_id:EventId::parse(&format!("event:{command_id}"))?,expected_revision:current.observed_revision,kind:EventKind::parse(P::KIND)?,causes:vec![],basis:BasisBinding::Exact(current.request.initial_basis)})?,reason:CommandReason::new(CommandReasonInput{summary:review.summary.clone(),evidence:vec![],decision:None,change:None})?,payload:payload(value)?}};
 let response=local(app,&MachineRequest::Command{command})?;
 std::fs::write(root().join(format!("accept-{phase}-receipt.json")),serde_json::to_vec_pretty(&response)?)?;
 println!("{}",serde_json::to_string(&response)?);Ok(())
}
fn acceptance_basis(kind:&str,work:&WorkId,obligations:&[ObligationId],evidence:&[EvidenceId])->Result<BasisRequest>{let mut roots=BTreeSet::from([SubjectRef::Work(work.clone()),SubjectRef::Outcome(outcome_id()?)]);roots.extend(obligations.iter().cloned().map(SubjectRef::Obligation));roots.extend(evidence.iter().cloned().map(SubjectRef::Evidence));Ok(BasisRequest::new(BasisRequestInput{purpose:BasisPurpose::Mutation(EventKind::parse(kind)?),roots:roots.into_iter().collect(),policy:ContextRequirement::NotApplicable,capacity:ContextRequirement::NotApplicable,closure:ClosureRequirement::KnownGraph})?)}
fn main()->Result<()> {
 let args:Vec<String>=std::env::args().collect();let phase=args.get(1).ok_or("usage: accept-candidate inspect | open-review/evidence/stage/work ROOT_REVIEW_JSON")?;
 require_retirement(root())?;
 let mut config:ManagedRuntimeConfig=serde_json::from_slice(&std::fs::read(root().join("service/managed.json"))?)?;
 config.application.service.store_mode=ApplicationStoreMode::Open;
 let app=ApplicationService::open_filesystem(&root().join("service"),config.application.service)?;
 let job=job(&app)?;
 let current_work=app.store().read(ReadAt::Current)?.get_typed::<zap_domain::control::WorkRecord>(&job.work_id)?.ok_or("canonical work missing")?;
 eprintln!("revision={} work_state={:?} work_revision={}",app.store().head()?.get(),current_work.state,current_work.revision.get());
 if phase=="inspect"{std::fs::write(root().join("accept-current-job.json"),serde_json::to_vec_pretty(&job)?)?;println!("execution={:?} acceptance={:?} candidate={:?}",job.execution,job.acceptance,job.candidate_id);return Ok(())}
 let review:Review=serde_json::from_slice(&std::fs::read(args.get(2).ok_or("Root review input required")?)?)?;
 if !review.approved||review.fresh_distinct_cases!=37||review.reused_distinct_cases!=18{return Err("Root approval and actual passing bounded test observation required".into())}
 for entry in review.evidence_files.as_object().ok_or("Root evidence hashes required")?.values(){let path=entry["path"].as_str().ok_or("evidence path")?;if PayloadDigest::hash(&std::fs::read(path)?).to_string()!=entry["sha256"].as_str().ok_or("evidence digest")?{return Err("Root evidence bytes changed".into())}}
 let approved_bytes=std::fs::read(args.get(2).ok_or("Root input")?)?;
 let verifier=job.intent.safe_stop.verifier.as_ref().ok_or("safe verifier")?;
 let safe=app.store().read(ReadAt::Current)?.get_typed::<zap_runtime::VerificationRecord>(verifier)?.ok_or("safe verification missing")?;
 if safe.job_id!=job.job_id||safe.state!=zap_runtime::VerificationState::Passed||!safe.artifacts.contains(&ArtifactDigest::hash(&approved_bytes)){return Err("exact Root report not retained by safe verification".into())}
 let candidate:CandidateResult=serde_json::from_slice(&std::fs::read(root().join("candidate-result.json"))?)?;
 require_criterion_decisions(&serde_json::json!({"criterion_assessments": &review.criterion_assessments}),&candidate)?;
 if job.candidate_id.as_ref()!=Some(&candidate.candidate_id)||job.producer!=candidate.producer||job.contract_digest!=candidate.contract_digest||job.relevant_basis!=candidate.relevant_basis{return Err("Candidate file is not the exact collected canonical job candidate".into())}
 let claim:RuntimeJobClaimRecord=serde_json::from_slice(&std::fs::read(root().join("ready-capture.json"))?)?;
 require_observations(&serde_json::json!({"observations":&review.observations}),&claim)?;
 let mut evidence_ids=claim.work.checks.iter().map(|p|EvidenceId::parse(&format!("evidence.{}.root-review",p.verification_id))).collect::<std::result::Result<Vec<_>,_>>()?;evidence_ids.sort();if evidence_ids.len()!=8{return Err("All eight captured checks required".into())}let stage=StageAcceptanceId::parse("stage.m08b1.functional")?;
 let command_id=CommandId::parse(&format!("root-review.m08b1.{phase}"))?;
 if let Some((_,receipt))=app.store().lookup_commit(&command_id)? {
  println!("already_committed phase={} revision={} work_state={:?}",phase,receipt.revision().get(),current_work.state);return Ok(())
 }
 let snapshot=app.store().read(ReadAt::Current)?;
 let retained=match phase.as_str(){
 "open-review" if matches!(current_work.state,WorkState::Candidate|WorkState::Accepted)=>snapshot.get_typed::<CandidateReviewRecord>(&candidate.candidate_id)?.is_some(),
 "evidence"=>evidence_ids.iter().map(|id|snapshot.get_typed::<EvidenceAdjudicationRecord>(id)).collect::<std::result::Result<Vec<_>,_>>()?.iter().all(|row|row.as_ref().is_some_and(|r|r.candidate_id==candidate.candidate_id)),
 "stage"=>snapshot.get_typed::<StageAcceptanceRecord>(&stage)?.is_some_and(|r|r.candidate_id==candidate.candidate_id),
 "work"=>snapshot.get_typed::<WorkAcceptanceRecord>(&WorkAcceptanceId::parse("acceptance.m08b1.root")?)?.is_some_and(|r|r.candidate_id==candidate.candidate_id),
 _=>false};
 drop(snapshot);
 if retained {println!("already_retained phase={} work_state={:?}",phase,current_work.state);return Ok(())}

 match phase.as_str(){
 "open-review"=>{let p=WorkTransitioned{schema:WorkTransitionedSchema::V1,work_id:candidate.work_id.clone(),from_state:WorkState::Active,to_state:WorkState::Candidate,successor_ids:vec![]};let b=work_transition_basis_request(&p)?;send(&app,phase,&p,b,&review)?;},
 "evidence"=>{
 for plan in &claim.work.checks {
 let row=review.observations.as_array().ok_or("actual observations")?.iter().find(|r|r["verification_id"].as_str()==Some(plan.verification_id.as_str())).ok_or("actual method absent")?;
 let actual_argv:Vec<String>=serde_json::from_value(row["actual_argv"].clone())?;
 let actual_cases:Vec<String>=serde_json::from_value(row["cases"].clone())?;

 let evidence=EvidenceId::parse(&format!("evidence.{}.root-review",plan.verification_id))?;
 if let Some(prior)=app.store().read(ReadAt::Current)?.get_typed::<EvidenceAdjudicationRecord>(&evidence)?{if prior.candidate_id!=candidate.candidate_id{return Err("Evidence ID belongs to another candidate".into())}continue;}
 let report=candidate.artifacts.iter().find(|a|a.kind==ArtifactKind::Report).ok_or("actual candidate report artifact absent")?;
 let captures:Vec<SourceCapture>=claim.work.sources.iter().map(|s|SourceCapture{source_id:s.source_id.clone(),digest:s.digest}).collect();
 let mut limitations=review.limitations.clone();for note in &candidate.unresolved{if !limitations.contains(note){limitations.push(note.clone());}}
 let p=EvidenceAdjudicated{schema:EvidenceAdjudicatedSchema::V1,evidence_id:evidence.clone(),candidate_id:candidate.candidate_id.clone(),verification_id:plan.verification_id.clone(),expected_revision:Revision::GENESIS,disposition:EvidenceDisposition::Accepted,applies_to:EvidenceApplicability{outcome_id:outcome_id()?,obligation_ids:claim.work.obligation_ids.clone(),work_ids:vec![candidate.work_id.clone()],stage:Some(MaturityStage::Functional),scope:review.scope.clone()},source_captures:captures.clone(),method:VerificationMethod{argv:actual_argv.iter().map(|a|BoundedText::parse(a)).collect::<std::result::Result<Vec<_>,_>>()?,target:BoundedText::parse(plan.target.as_str())?,toolchain:BoundedText::parse(plan.toolchain.as_str())?,environment:review.actual_environment.clone(),subjects:plan.subjects.clone(),cases:actual_cases.iter().map(|a|BoundedText::parse(a)).collect::<std::result::Result<Vec<_>,_>>()?},limitations,observation:EvidenceObservation{evidence_id:evidence.clone(),result:EvidenceResult::ObservedPass,artifact:report.digest,work_ids:vec![candidate.work_id.clone()],source_ids:captures.iter().map(|s|s.source_id.clone()).collect()}};
 let mut roots:BTreeSet<_>=p.method.subjects.iter().cloned().collect();roots.insert(SubjectRef::Outcome(p.applies_to.outcome_id.clone()));roots.extend(p.applies_to.obligation_ids.iter().cloned().map(SubjectRef::Obligation));roots.extend(p.applies_to.work_ids.iter().cloned().map(SubjectRef::Work));roots.extend(captures.iter().map(|s|SubjectRef::Source(s.source_id.clone())));
 let b=BasisRequest::new(BasisRequestInput{purpose:BasisPurpose::Verification(p.verification_id.clone()),roots:roots.into_iter().collect(),policy:ContextRequirement::NotApplicable,capacity:ContextRequirement::NotApplicable,closure:ClosureRequirement::KnownGraph})?;send(&app,&format!("evidence.{}",plan.verification_id),&p,b,&review)?;
 }
 },
 "stage"=>{let p=StageAccepted{schema:StageAcceptedSchema::V1,stage_acceptance_id:stage,candidate_id:candidate.candidate_id.clone(),work_id:candidate.work_id.clone(),stage:MaturityStage::Functional,outcome_id:outcome_id()?,evidence_ids:evidence_ids.clone(),obligation_ids:claim.work.obligation_ids.clone(),scope:review.scope.clone(),summary:review.summary.clone()};let b=acceptance_basis(StageAccepted::KIND,&p.work_id,&p.obligation_ids,&p.evidence_ids)?;send(&app,phase,&p,b,&review)?;},
 "work"=>{let p=WorkAccepted{schema:WorkAcceptedSchema::V1,acceptance_id:WorkAcceptanceId::parse("acceptance.m08b1.root")?,candidate_id:candidate.candidate_id.clone(),work_id:candidate.work_id.clone(),outcome_id:outcome_id()?,stage_acceptance_id:stage,evidence_ids:evidence_ids.clone(),obligation_ids:claim.work.obligation_ids.clone(),integration_acceptance_ids:vec![],summary:review.summary.clone()};let b=acceptance_basis(WorkAccepted::KIND,&p.work_id,&p.obligation_ids,&p.evidence_ids)?;send(&app,phase,&p,b,&review)?;},
 _=>return Err("unknown phase".into())
 };Ok(())
}

fn require_observations(review:&serde_json::Value,claim:&RuntimeJobClaimRecord)->Result<()> {
 let rows=review["observations"].as_array().ok_or("Root actual check observations required")?;
 if rows.len()!=8||claim.work.checks.len()!=8{return Err("All eight exact captured verification plans required".into())}
 let mut ids=std::collections::BTreeSet::new();
 for plan in &claim.work.checks {
 let row=rows.iter().find(|r|r["verification_id"].as_str()==Some(plan.verification_id.as_str())).ok_or("Captured verification observation missing")?;
 if !ids.insert(plan.verification_id.as_str())||row["exit"]!=0||row["count"].as_u64().unwrap_or(0)==0{return Err("Nonzero passing actual check required".into())}
 let captured:Vec<String>=std::iter::once(plan.program.as_str().to_owned()).chain(plan.arguments.iter().map(|x|x.as_str().to_owned())).collect();
 if serde_json::to_value(&captured)?!=row["captured_argv"]{return Err("Root method differs from captured contract".into())}
 let actual:Vec<String>=serde_json::from_value(row["actual_argv"].clone())?;let mut measured=captured.clone();measured.extend(["--".into(),"--nocapture".into()]);
 if actual!=captured && !(plan.verification_id.as_str()=="verification.m08b1"&&actual==measured){return Err("Only measured lifecycle nocapture suffix may normalize".into())}
 let bytes=std::fs::read(row["log"].as_str().ok_or("actual check log")?)?;
 if PayloadDigest::hash(&bytes).to_string()!=row["log_sha256"].as_str().ok_or("check log hash")?{return Err("Actual check log changed".into())}
 let count=row["count"].as_u64().ok_or("count")?;let text=std::str::from_utf8(&bytes)?;
 if !text.contains(&format!("test result: ok. {count} passed; 0 failed; 0 ignored"))||row["cases"].as_array().is_none_or(|a|a.len()!=count as usize){return Err("Actual nonzero check result differs".into())}
 }
 Ok(())
}
