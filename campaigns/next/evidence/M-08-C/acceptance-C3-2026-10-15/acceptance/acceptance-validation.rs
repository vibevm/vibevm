const APPROVAL_TOKEN:&str="Root approves FULL M08C3 corrected candidate with seven proofs and separate integrated codegen gate";
fn require_criterion_decisions(review:&serde_json::Value,candidate:&CandidateResult)->Result<()> {
 let decisions=review["criterion_assessments"].as_array().ok_or("Root criterion assessments missing")?;
 if decisions.len()!=2||decisions.len()!=candidate.criteria.len(){return Err("Both original criteria require Root judgment".into())}
 for c in &candidate.criteria {let d=decisions.iter().find(|d|d["requirement"].as_str()==Some(c.requirement.as_str())).ok_or("criterion missing")?;
  if d["producer_disposition"]!=serde_json::to_value(c.disposition)?||d["review_disposition"]!="satisfied"||d["rationale"].as_str().is_none_or(|s|s.trim().is_empty()){return Err("Root criterion rationale/disposition missing".into())}}
 Ok(())
}
fn checked_json(path:&str,expected:&str)->Result<serde_json::Value>{
 let bytes=std::fs::read(path)?;if PayloadDigest::hash(&bytes).to_string()!=expected{return Err(format!("evidence changed: {path}").into())}Ok(serde_json::from_slice(&bytes)?)
}
fn require_retirement(root:&std::path::Path)->Result<()> {
 let live:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("ROOT-ACTUAL-LAUNCH.json"))?)?;
 let retired:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("cleanup-after-integration/ROOT-QUIESCENCE.json"))?)?;
 let rows=live["processes"].as_object().ok_or("actual launch identities missing")?;
 if rows.len()!=5||retired["all5_original_absent"]!=true||retired["native_collected"]!=true||retired["normal_exact_stop_delivery"]!="delivered"{return Err("Root exact retirement and normal collected stop required".into())}
 let complete=retired["complete_launch_identities"].as_object().ok_or("complete launch identity check missing")?;
 if complete.len()!=rows.len(){return Err("retirement set mismatch".into())}
 for(pid,row)in rows{if complete[pid]["expected_birth"]!=row["birth"]||complete[pid]["original_absent"]!=true||std::path::Path::new(&format!("/proc/{pid}")).exists(){return Err("exact launch identity/current absence mismatch".into())}}
 let commit:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("ROOT-SOURCE-INTEGRATION.json"))?)?;
 if commit["source_commit"]!="9635e2f80cd1599b408fd0dcdf29f175941be0c1"||commit["all_hashes_stable"]!=true||commit["frozen_subjects"]!=317||commit["files"]!=41||commit["Root_check_codegen_exit"]!=0{return Err("Root exact integrated source commit missing".into())}Ok(())
}
fn checked_commands(result:&serde_json::Value,held:&serde_json::Value,expected_cases:u64)->Result<()> {
 let commands=result["commands"].as_array().ok_or("actual commands missing")?;let rows=held.as_array().ok_or("held commands missing")?;
 if result["executed"]!=true||result["passed"]!=true||commands.len()!=rows.len(){return Err("actual passed execution missing".into())}
 let mut cases=0;
 for (cmd,row) in commands.iter().zip(rows){
  if cmd["exit"]!=0||cmd["passed"]!=true||cmd["argv"]!=row["argv"]||cmd["exit"]!=row["exit"]||cmd["log"]!=row["log"]{return Err("actual command/exit differs".into())}
  let log=std::fs::read(row["log"].as_str().ok_or("log")?)?;if PayloadDigest::hash(&log).to_string()!=row["log_sha256"].as_str().ok_or("log hash")?{return Err("raw log changed".into())}
  if let Some(counts)=cmd["counts"].as_array(){for c in counts{let count=c[0].as_str().ok_or("count")?.parse::<u64>()?;cases+=count;if c[1]!="0"||c[2]!="0"||!String::from_utf8_lossy(&log).contains(&format!("test result: ok. {count} passed; 0 failed; 0 ignored")){return Err("raw nonignored test denominator differs".into())}}}
 }
 if cases!=expected_cases{return Err("actual case denominator differs".into())}Ok(())
}
fn require_observations(review:&serde_json::Value,claim:&RuntimeJobClaimRecord)->Result<()> {
 let root=std::path::Path::new("/fast/zap-managed/2026-10-15-m08c3-correction");let repo=std::path::Path::new("/fast/git/v/vibevm-next");
 let final_map:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("final-source-hashes.json"))?)?;
 let sources=final_map.as_object().ok_or("final source map")?;if sources.len()!=317{return Err("exact317 source closure required".into())}
 for (p,d) in sources{if PayloadDigest::hash(&std::fs::read(repo.join(p))?).to_string()!=d.as_str().ok_or("source hash")?{return Err("final source changed".into())}}
 let staged:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("ROOT-STAGED-SOURCE.json"))?)?;if staged["source_hashes_before_gate"]!=final_map{return Err("Root gate source closure differs".into())}
 let producer:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("REPORT.json"))?)?;
 let rows=review["observations"].as_array().ok_or("seven observations required")?;if rows.len()!=7||claim.work.checks.len()!=7{return Err("original seven methods required".into())}
 for(index,plan)in claim.work.checks.iter().enumerate(){
  let row=rows.iter().find(|r|r["verification_id"].as_str()==Some(plan.verification_id.as_str())).ok_or("method missing")?;let group=format!("V{}",index+1);
  let captured:Vec<String>=std::iter::once(plan.program.as_str().to_owned()).chain(plan.arguments.iter().map(|a|a.as_str().to_owned())).collect();
  if row["group"]!=group||row["captured_argv"]!=serde_json::to_value(captured)?||row["selected_for_acceptance"]!=true||row["observed_result"]!="observed_pass"{return Err("method identity or selected current pass missing".into())}
  let result=checked_json(row["result_path"].as_str().ok_or("result path")?,row["result_sha256"].as_str().ok_or("result hash")?)?;
  if result!=producer["verification"]["latest"][&group]{return Err("producer result evidence differs".into())}
  if index==4 {
   let cmds=result["commands"].as_array().ok_or("V5 commands")?;if result["passed"]!=false||cmds.len()!=3||cmds[0]["exit"]!=0||cmds[1]["exit"]!=1||cmds[2]["exit"]!=0||row["producer_failed_check_retained"]!=true{return Err("producer V5 failed history must remain".into())}
   for(cmd,held)in cmds.iter().zip(row["commands"].as_array().ok_or("held V5")?){if cmd["argv"]!=held["argv"]||cmd["exit"]!=held["exit"]||PayloadDigest::hash(&std::fs::read(held["log"].as_str().ok_or("log")?)?).to_string()!=held["log_sha256"].as_str().ok_or("hash")?{return Err("V5 history mismatch".into())}}
   let g=&row["integrated_gate"];let actual=checked_json(g["path"].as_str().ok_or("gate path")?,g["sha256"].as_str().ok_or("gate hash")?)?;
   if actual["exit"]!=0||actual["source_staged"]!=true||actual["command"]!=g["command"]||row["actual_argv"]!=actual["command"]||actual["log"]!=g["log"]||PayloadDigest::hash(&std::fs::read(g["log"].as_str().ok_or("gate log")?)?).to_string()!=g["log_sha256"].as_str().ok_or("gate log hash")?{return Err("separate actual Root integrated gate missing".into())}
  }else if index==6 {
   if result["executed"]!=true||result["passed"]!=true||result["xml_parsed"]!=true||result["prior_spec_lines_preserved"]!=true||result["old_schema_drift"]!=serde_json::json!([])||result["unqualified_generated_drift"]!=serde_json::json!([]){return Err("current old-only preservation proof missing".into())}
  }else{
   if row["actual_argv"]!=row["captured_argv"]{return Err("fresh method differs".into())}
   checked_commands(&result,&row["commands"],[22,6,6,20,0,0,0][index])?;
   if index==3 {let reuse=&row["reuse"];if result["conditional_selection"]["reused"]!=serde_json::json!([reuse])||row["reused_case_count"]!=1{return Err("exact old reuse must remain distinct".into())}
    for(p,d)in reuse["source_hashes"].as_object().ok_or("reuse hashes")?{if PayloadDigest::hash(&std::fs::read(repo.join(p))?).to_string()!=d.as_str().ok_or("reuse hash")?{return Err("reused input changed".into())}}
    let old=&reuse["original_command_result"];if old["exit"]!=0||old["counts"]!=serde_json::json!([["1","0","0"]])||PayloadDigest::hash(&std::fs::read(old["archived_log"].as_str().ok_or("reused log")?)?).to_string()!=old["log_sha256"].as_str().ok_or("reused log hash")?{return Err("reused exact command proof missing".into())}
   }
  }
 }
 Ok(())
}
