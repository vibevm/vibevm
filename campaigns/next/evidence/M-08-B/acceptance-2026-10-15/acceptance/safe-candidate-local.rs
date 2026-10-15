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
use zap_app::*;use zap_core::*;use zap_wire::*;use zap_runtime::*;use std::path::Path;
type Result<T>=std::result::Result<T,Box<dyn std::error::Error>>;
fn frame<P:CommandPayload+serde::Serialize>(app:&ApplicationService,p:&P,name:&str)->Result<CanonicalCommandFrame>{Ok(CanonicalCommandFrame::new(CommandHeader::new(CommandHeaderInput{protocol:ProtocolEpoch::new(2)?,store_id:app.identity().store_id.clone(),campaign_id:app.identity().campaign_id.clone(),base_id:app.identity().base_id.clone(),command_id:CommandId::parse(name)?,event_id:EventId::parse(&format!("event:{name}"))?,expected_revision:app.store().head()?,kind:EventKind::parse(P::KIND)?,causes:vec![],basis:BasisBinding::NotApplicable})?,CommandReason::new(CommandReasonInput{summary:BoundedText::parse("Root independently attests full M08B1 pure-query and explicit-preparation safe boundary with eight actual checks, complete review and actual Completed local fixture effects; original process tree retired; vendor/native/platform limits retained")?,evidence:vec![],decision:None,change:None})?,CanonicalPayload::encode_json(CodecEpoch::CURRENT,p)?)?)}
fn main()->Result<()> {
 let root=Path::new("/fast/zap-managed/2026-10-15-m08b1-review-correction");let directory=root.join("service");let review_path=root.join("ROOT-REVIEW.json");let review:serde_json::Value=serde_json::from_slice(&std::fs::read(&review_path)?)?;
 if review["approved"]!=true{return Err("Independent Root approval required".into())}
 for entry in review["evidence_files"].as_object().ok_or("Root evidence hashes required")?.values(){let path=entry["path"].as_str().ok_or("evidence path")?;if PayloadDigest::hash(&std::fs::read(path)?).to_string()!=entry["sha256"].as_str().ok_or("evidence digest")?{return Err("Root evidence bytes changed".into())}}
 require_retirement(root)?;
 let retirement=root.join("ROOT-RETIREMENT.json");if !review["evidence_files"].as_object().ok_or("evidence")?.values().any(|v|v["path"].as_str()==retirement.to_str()){return Err("Root exact retirement must be hash-bound".into())}
 let captured:RuntimeJobClaimRecord=serde_json::from_slice(&std::fs::read(root.join("ready-capture.json"))?)?;require_observations(&review,&captured)?;
 let original:CandidateResult=serde_json::from_slice(&std::fs::read(root.join("candidate-result.json"))?)?;require_criterion_decisions(&review,&original)?;
 let producer_bytes=std::fs::read(root.join("worker-report.md"))?;
 let artifact=original.artifacts.iter().find(|a|a.kind==ArtifactKind::Report).ok_or("producer report missing")?;if ArtifactDigest::hash(&producer_bytes)!=artifact.digest||original.effect_state!=CandidateEffectState::Completed{return Err("exact original ReportedCompleted report required".into())}
 let pids=std::env::args().skip(1).map(|a|a.parse::<u32>()).collect::<std::result::Result<Vec<_>,_>>()?;if pids.len()!=5||pids.iter().any(|p|*p==0){return Err("usage: safe-candidate-local ACTUAL_SERVICE_PID ACTUAL_MANAGER_PID ACTUAL_POD_PID ACTUAL_CODEX_PID ACTUAL_CODE_MODE_PID (Root-confirmed original processes after stop)".into())}for pid in pids{if Path::new(&format!("/proc/{pid}")).exists(){return Err(format!("Original physical PID {pid} exists; require fresh physical assessment").into())}}
 let mut config:ManagedRuntimeConfig=serde_json::from_slice(&std::fs::read(directory.join("managed.json"))?)?;config.application.service.store_mode=ApplicationStoreMode::Open;
 let trusted_id=config.application.service.trust.trusted.channel.credential_id.clone();let observation=config.application.service.trust.trusted.observation.clone();let coordinator_id=config.application.service.trust.coordinator.credential.credential_id.clone();
 let app=ApplicationService::open_filesystem(&directory,config.application.service)?;let mut coordinator=std::fs::read(directory.join("coordinator.secret"))?;let mut trusted=std::fs::read(directory.join("trusted.secret"))?;
 let claim:RuntimeJobClaimRecord=serde_json::from_slice(&std::fs::read(root.join("ready-capture.json"))?)?;let candidate:CandidateResult=serde_json::from_slice(&std::fs::read(root.join("candidate-result.json"))?)?;let job_id=claim.job_id.clone();let mut job=app.store().read(ReadAt::Current)?.get_typed::<RuntimeJobRecord>(&job_id)?.ok_or("job missing")?;
 if !matches!(job.execution,ExecutionState::Succeeded)||job.candidate_id.as_ref()!=Some(&candidate.candidate_id)||job.producer!=candidate.producer||job.contract_digest!=candidate.contract_digest||!matches!(&job.candidate_result_contract.template.effect,CandidateEffectPolicy::Reported{allowed} if allowed.contains(&CandidateEffectState::Completed)){return Err("Actual successful Reported Completed fixture candidate is required".into())}
 let page=app.store().read(ReadAt::Current)?.scan_typed::<ManagedProgressRecord>(KeyRange{start:std::ops::Bound::Unbounded,end:std::ops::Bound::Unbounded},PageLimit::within(32,32)?)?;if page.completeness!=RecordCompleteness::Complete{return Err("Managed marker lookup requires complete bounded state".into())}let rows=page.items;
 if !rows.iter().any(|r|matches!(&r.target,zap_api::ManagedExecutionTarget::Job{job_id:jid,dispatch_id} if jid==&job.job_id&&dispatch_id==&job.dispatch_id)&&r.collected&&r.native_turn.is_some()&&!r.unknown_tail&&!r.failed){return Err("Exact native turn has no canonical successful collected/completed marker".into())}
 let verifier=job.intent.safe_stop.verifier.clone().ok_or("Declared safe verifier absent")?;
 let expected_scope=VerificationScope::SafeBoundary{attempt_id:job.attempt_id.clone(),effect_id:job.effect_id.clone(),boundary:job.intent.safe_stop.boundary.clone()};
 let capabilities=app.store().read(ReadAt::Current)?.get_typed::<CapabilityObservationRecord>(&job.expected_producer.capability_observation)?.ok_or("Actual capability record absent")?;
 let provenance=DriverProvenance::bind(&capabilities.capabilities,observation.clone())?;
 let mut record=app.store().read(ReadAt::Current)?.get_typed::<VerificationRecord>(&verifier)?;
 if let Some(r)=&record {if r.job_id!=job_id||r.scope!=expected_scope{return Err("Existing verifier belongs to a different exact scope".into())}}
 if record.is_none(){let p=VerificationClaimedPayload{record:VerificationRecord{verification_id:verifier.clone(),job_id:job_id.clone(),scope:expected_scope.clone(),state:VerificationState::Claimed,observation:None,artifacts:vec![],revision:Revision::GENESIS}};let receipt=app.submit_credential(&coordinator_id,&coordinator,frame(&app,&p,"root-safe.m08b1.claim")?)?;println!("{}",serde_json::to_string(&receipt)?);record=app.store().read(ReadAt::Current)?.get_typed::<VerificationRecord>(&verifier)?;}
 let record=record.ok_or("Verifier claim not persisted")?;
 if matches!(record.state,VerificationState::Claimed){
 let artifacts=zap_store::ArtifactStore::create(directory.join("artifacts"))?;let retained_review=artifacts.prepare_file(&review_path)?.publish()?;
 let candidate:CandidateResult=serde_json::from_slice(&std::fs::read(root.join("candidate-result.json"))?)?;let report=candidate.artifacts.iter().find(|a|a.kind==ArtifactKind::Report).ok_or("actual candidate report absent")?;
 let p=VerificationResultPayload{verification_id:verifier.clone(),expected_record_revision:record.revision,state:VerificationState::Passed,observation:observation.clone(),artifacts:vec![report.digest,retained_review.digest()],provenance:provenance.clone()};let receipt=app.submit_observation(&trusted_id,&trusted,frame(&app,&p,"root-safe.m08b1.result")?)?;println!("{}",serde_json::to_string(&receipt)?);
 }else if !matches!(record.state,VerificationState::Passed){return Err("Retained verification failed; no positive safe state permitted".into())}
 job=app.store().read(ReadAt::Current)?.get_typed::<RuntimeJobRecord>(&job_id)?.ok_or("job missing")?;
 if !matches!(job.safe,SafeState::Safe|SafeState::Completed){let p=SafeStateRecordedPayload{job_id:job_id.clone(),expected_job_revision:job.revision,safe_state:SafeState::Safe,observation,evidence:vec![verifier],provenance};let receipt=app.submit_observation(&trusted_id,&trusted,frame(&app,&p,"root-safe.m08b1.record")?)?;println!("{}",serde_json::to_string(&receipt)?);}
 coordinator.fill(0);trusted.fill(0);let job=app.store().read(ReadAt::Current)?.get_typed::<RuntimeJobRecord>(&job_id)?.ok_or("job missing")?;let shared=app.store().read(ReadAt::Current)?.get_typed::<WorkExecutionObservationRecord>(&job_id)?.ok_or("shared execution missing")?;
 println!("head={} job_safe={:?} shared_safe={:?} acceptance={:?}",app.store().head()?.get(),job.safe,shared.safe_state,job.acceptance);Ok(())
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
