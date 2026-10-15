from pathlib import Path
import json,hashlib,re
r=Path(__file__).parent;old=Path('/fast/zap-managed/2026-10-15-m08a2-correction');repo=Path('/fast/git/v/vibevm-next');review=Path('/fast/git/v/research/2026-10-15-m08b1-correction-review')
def h(p):
 with Path(p).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()
def j(p,x):p.write_text(json.dumps(x,indent=2)+'\n')
candidate=json.loads((r/'candidate-result.json').read_text());claim=json.loads((r/'ready-capture.json').read_text());assert len(claim['work']['checks'])==8;assert all(c['disposition']=='satisfied' for c in candidate['criteria']);assert candidate['work_id']==claim['work']['work_id'] and candidate['producer']['job_id']==claim['job_id']
for n,kind in [('worker-report.md','report'),('worker.patch','patch')]:
 a=next(a for a in candidate['artifacts'] if a['kind']==kind);assert h(r/n)==a['digest'] and (r/n).stat().st_size==a['byte_len']
subjects=json.loads((r/'worker-subjects.json').read_text());assert all(h(repo/p)==d for p,d in subjects.items())
results=json.loads((r/'worker-command-results.json').read_text());assert all(h(x['log'])==x['log_sha256'] for x in results)
reuse=json.loads((r/'worker-reused-evidence.json').read_text());assert all(h(x['log'])==x['log_sha256'] and all(h(repo/p)==d for p,d in x['consumed_subjects'].items()) for x in reuse['claims'])
logs=['worker-lifecycle-final-bound.log','worker-preserve-0-bound.log','worker-preserve-1-bound.log','worker-preserve-2-bound.log','worker-route.log','worker-mcp.log','worker-cli-final.log','worker-wire.log'];observations=[]
for plan,log in zip(claim['work']['checks'],logs):
 x=next(x for x in results if Path(x['log']).name==log);expected=[plan['program']]+plan['arguments'];assert x['argv']==expected or (plan['verification_id']=='verification.m08b1' and x['argv']==expected+['--','--nocapture']);assert x['exit']==0 and x['nonzero_case_count']>0
 text=Path(x['log']).read_text();cases=re.findall(r'^test (\S+) \.\.\. ok$',text,re.M);assert len(cases)==x['nonzero_case_count'],(log,len(cases));observations.append({'verification_id':plan['verification_id'],'captured_argv':expected,'actual_argv':x['argv'],'normalization':'Only trailing -- --nocapture adds measurement output; selection unchanged' if x['argv']!=expected else 'exact captured argv','exit':0,'count':len(cases),'cases':cases,'log':x['log'],'log_sha256':x['log_sha256'],'target':plan['target']})
evidence={}
for n in ['candidate-result.json','worker-report.md','worker.patch','worker-subjects.json','worker-command-results.json','worker-reused-evidence.json','worker-oracle-results.json','worker-patch-manifest.json','worker-cli-final-binding.json','worker-integrity.json','ready-capture.json','actual-candidate.json']:
 evidence[n]={'path':str(r/n),'sha256':h(r/n)}
for n in ['REPORT.md','REPORT.json','sourcehashes.json']:evidence['independent-'+n]={'path':str(review/n),'sha256':h(review/n)}
for x in results:evidence[Path(x['log']).name]={'path':x['log'],'sha256':x['log_sha256']}
for p,d in subjects.items():evidence['source:'+p]={'path':str(repo/p),'sha256':d}
ret=r/'ROOT-RETIREMENT.json';evidence['ROOT-RETIREMENT.json']={'path':str(ret),'sha256':h(ret) if ret.exists() else None}
proposal={'approved':False,'summary':'Root review proposal for complete M08B1 prepared adapter planning; both producer criteria satisfied, F1/F2 closed; ordinary functional acceptance only after exact retirement and final approval.','scope':'Full M08B1 pure admitted query and explicit preparation across source, lifecycle, native refusal, route, CLI/MCP and generated contracts; safe stop needs no product apply.','limitations':['37 fresh distinct scoped cases and18 explicitly matching-subject reused cases; not55 fresh runs. Eight primary checks are freshly measured.','check-codegen exit1 intended uncommitted generated diff with identical reproduction; wire-diff exit1 unchanged repomd before epoch verdict; global tail unrun.','Existing generated long-arm formatting debt retained; scoped authored formatting/diff passed.','Production Claude/Codex currentness unavailable without a qualified state owner; no real vendor/native/cross-platform commissioning or hostile-ABA claim.','Completed effects are actual local source/build/fixture/probe writes; query intervals independently measured pure. RuntimeUnreviewed is separate from canonical WorkAccepted.'],'actual_environment':'Linux; repository nightly rustc1.101.0-nightly(c36f14571 2026-10-01); CARGO_TARGET_DIR=/fast/zap-managed/2026-10-14-m10b1-correction/cargo-target; TMPDIR='+str(r/'tmp')+'; offline locked scoped commands; no tests rerun by operator.','evidence_files':evidence,'criterion_assessments':[{'requirement':c['requirement'],'producer_disposition':c['disposition'],'review_disposition':'satisfied','rationale':reason} for c,reason in zip(candidate['criteria'],['Actual query attempt counters and finite root byte/membership snapshots establish empty-cache query purity; admitted source/executable/state captures retain exact identity and refuse missing or drift without rehydration/reprobe. Independent review closes F1/F2, preserving finite-continuity limits.','Configured CLI and registered MCP share the same admitted route, validate generated identical success/refusal semantics, and carry missing/stale/unavailable distinctions; real child protocol and exact registration checks are bound. Full explicit-preparation safe stop is met without product apply.'])],'observations':observations,'fresh_distinct_cases':37,'reused_distinct_cases':18}
j(r/'ROOT-REVIEW-PROPOSAL.json',proposal);j(r/'ACCEPTANCE-BINDINGS.json',{'candidate_sha256':h(r/'candidate-result.json'),'producer_report_sha256':h(r/'worker-report.md'),'patch_sha256':h(r/'worker.patch'),'subjects':len(subjects),'primary_checks':observations,'retirement_available':ret.exists(),'no_phases_executed':True})
common='''
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
'''
def cv(s):return s.replace(str(old),str(r)).replace('vibevm-m08a2-correction','vibevm-m08b1-review-correction').replace('m08a2','m08b1').replace('M08A2','M08B1')
s=cv((old/'safe-candidate-local.rs').read_text());a=s.index(' if review["approved"]');b=s.index(' let original:CandidateResult=',a)
s=s[:a]+''' if review["approved"]!=true{return Err("Independent Root approval required".into())}
 for entry in review["evidence_files"].as_object().ok_or("Root evidence hashes required")?.values(){let path=entry["path"].as_str().ok_or("evidence path")?;if PayloadDigest::hash(&std::fs::read(path)?).to_string()!=entry["sha256"].as_str().ok_or("evidence digest")?{return Err("Root evidence bytes changed".into())}}
 require_retirement(root)?;
 let retirement=root.join("ROOT-RETIREMENT.json");if !review["evidence_files"].as_object().ok_or("evidence")?.values().any(|v|v["path"].as_str()==retirement.to_str()){return Err("Root exact retirement must be hash-bound".into())}
 let captured:RuntimeJobClaimRecord=serde_json::from_slice(&std::fs::read(root.join("ready-capture.json"))?)?;require_observations(&review,&captured)?;
'''+s[b:];s=s.replace('std::fs::read("/fast/git/v/vibevm-next/campaigns/next/evidence/M-08-A/report-correction.json")?','std::fs::read(root.join("worker-report.md"))?');s=s.replace('Root independently attests full M08B1 docs/contracts safe-stop with actual Completed document/probe effects;one metadataCase and independent F1-F3/criterion2 rationale;runtimecommissioning remainsdebt; original tree absent; no live activation','Root independently attests full M08B1 pure-query and explicit-preparation safe boundary with eight actual checks, complete review and actual Completed local fixture effects; original process tree retired; vendor/native/platform limits retained')
s+=common;(r/'safe-candidate-local.rs').write_text(s)
s=cv((old/'accept-candidate-local.rs').read_text()).replace('observed_metadata_case_count:u32,metadata_exit:i32,','observations:serde_json::Value,fresh_distinct_cases:u32,reused_distinct_cases:u32,');s=s.replace('if !review.approved||review.observed_metadata_case_count<1||review.metadata_exit!=0','if !review.approved||review.fresh_distinct_cases!=37||review.reused_distinct_cases!=18');s=s.replace('if evidence_ids.len()!=1{return Err("M08B1 requires its one exact captured Python metadata verification plan".into())}','if evidence_ids.len()!=8{return Err("All eight captured checks required".into())}');a=s.index(' if plan.verification_id.as_str()!=');b=s.index('\n let evidence=',a);s=s[:a]+''' let row=review.observations.as_array().ok_or("actual observations")?.iter().find(|r|r["verification_id"].as_str()==Some(plan.verification_id.as_str())).ok_or("actual method absent")?;
 let actual_argv:Vec<String>=serde_json::from_value(row["actual_argv"].clone())?;
 let actual_cases:Vec<String>=serde_json::from_value(row["cases"].clone())?;
'''+s[b:];s=s.replace('argv:std::iter::once(BoundedText::parse(plan.program.as_str())?).chain(plan.arguments.clone()).collect()','argv:actual_argv.iter().map(|a|BoundedText::parse(a)).collect::<std::result::Result<Vec<_>,_>>()?').replace('cases:vec![BoundedText::parse("four-clients-lifecycle-metadata-preservation")?]','cases:actual_cases.iter().map(|a|BoundedText::parse(a)).collect::<std::result::Result<Vec<_>,_>>()?');needle=' let mut evidence_ids=claim.work.checks.iter()';s=s.replace(needle,' require_observations(&serde_json::json!({"observations":&review.observations}),&claim)?;\n'+needle);s+=common;(r/'accept-candidate-local.rs').write_text(s)
for n in ['safe-candidate-local','accept-candidate-local']:
 args=json.loads((old/(n+'-build-args.json')).read_text());j(r/(n+'-build-args.json'),[a.replace(str(old),str(r)) for a in args])
print('proposal false; subjects',len(subjects),'primary8 counts',[x['count'] for x in observations],'actual logs',len(results),'no phases')
