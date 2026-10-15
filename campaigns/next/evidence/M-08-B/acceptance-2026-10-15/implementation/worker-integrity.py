from pathlib import Path
import hashlib,json,subprocess
r=Path(__file__).parent
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
base=json.loads((r/'source-baseline.json').read_text()); roster=json.loads((r/'READ-ROSTER.json').read_text())
changed=subprocess.check_output(['git','diff','--name-only','HEAD']).decode().splitlines()
new=subprocess.check_output(['git','ls-files','--others','--exclude-standard']).decode().splitlines()
paths=sorted(set(changed+new));allowed=json.loads((r/'CANONICAL-TASK.json').read_text())['tasks'][0]['write_paths']+['crates/progress-core/src/generated/','vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v1.1.0/crates/core-ai-native-specmap/src/generated/','vibevm/vibepacks/org.vibevm.doc/web/v1.1.0/site/src/generated/doc-manifest.ts']
assert all(any(f==a or (a.endswith('/') and f.startswith(a)) for a in allowed) for f in paths)
assert set(base['permitted_preexisting_delta']).issubset(paths)
assert all(sha(f)==roster['sha256'][f] for f in roster['read_only_additional_safefs'])
old=base['permitted_preexisting_delta'];delta={f:{'baseline':old.get(f,base['sha256'].get(f)),'current':sha(f)} for f in paths if sha(f)!=old.get(f,base['sha256'].get(f))}
subjects={f:sha(f) for f in sorted(set(paths)|set(base['sha256'])|{'Cargo.toml','Cargo.lock','crates/vibe-lifecycle/src/mechanism/deploy/transaction.rs'}) if Path(f).is_file()}
(r/'worker-subjects.json').write_text(json.dumps(subjects,indent=2))
spec={}
for f,block in json.loads((r/'worker-spec-blocks.json').read_text()).items():
 now=Path(f).read_text(); original=(r/'worker-baseline'/f).read_bytes(); assert now.count(block)==1
 restored=now.replace(block,'').encode(); assert restored==original
 spec[f]={'all_previous_bytes_preserved':True,'original_sha256':hashlib.sha256(original).hexdigest(),'stripped_sha256':hashlib.sha256(restored).hexdigest(),'current_sha256':sha(f)}
(r/'worker-spec-preservation.json').write_text(json.dumps(spec,indent=2))
(r/'worker-integrity.json').write_text(json.dumps({'head':subprocess.check_output(['git','rev-parse','HEAD']).decode().strip(),'retained_count':len(old),'candidate_files':len(paths),'correction_delta_files':delta,'retained_unchanged':[f for f,h in old.items() if sha(f)==h],'safefs_readonly_equal':True,'outside_write_paths':[],'missing_retained_files':[]},indent=2))
prior=Path('/fast/zap-managed/2026-10-15-m08b1-continuation')
claims=[('projection',[1,9,10,11,12],lambda f:f.startswith('crates/vibe-agent-projection/') or f in ['.cargo/config.toml','Cargo.toml','Cargo.lock']),('world',[4],lambda f:f.startswith(('crates/vibe-orchestrator/src/world/','crates/vibe-orchestrator/src/phase/','crates/vibe-orchestrator/src/install/','crates/vibe-agent-projection/','crates/vibe-workspace/')) or f in ['crates/vibe-orchestrator/src/world/mod.rs','crates/vibe-orchestrator/src/callback.rs','crates/vibe-orchestrator/src/phase.rs','.cargo/config.toml','Cargo.toml','Cargo.lock']),('command_progress',[13,14],lambda f:f in ['crates/vibe-cli/src/cli.rs','crates/vibe-cli/src/main.rs','crates/vibe-cli/src/commands/mod.rs','crates/vibe-cli/src/command_progress.rs','crates/vibe-cli/src/command_progress/tests.rs','.cargo/config.toml','Cargo.toml','Cargo.lock'])]
reused=[]
for claim,indices,select in claims:
 for i in indices:
  snap=json.loads((prior/f'worker-final2-subjects-{i}.json').read_text()); checked={f:h for f,h in snap.items() if select(f)}
  mismatch=[f for f,h in checked.items() if sha(f)!=h]; assert not mismatch,(claim,mismatch)
  log=prior/f'worker-final2-check-{i}.log'; assert 'test result: ok.' in log.read_text()
  reused.append({'claim':claim,'snapshot':str(prior/f'worker-final2-subjects-{i}.json'),'log':str(log),'log_sha256':sha(log),'consumed_subjects':checked,'mismatches':mismatch})
(r/'worker-reused-evidence.json').write_text(json.dumps({'claims':reused,'rationale':'Reuse is limited to unchanged source closure/world admission and CLI command registration claims. F1/F2 change client executable/list observation, deploy preparation-result carriage and new refusal vocabulary, none of which these selected source-world/registration cases execute. Source capture/projection owners, world callbacks/phase admission helpers, fixtures and registration inputs match the recorded checked subjects. This is not reuse of the whole old binaries or old client/CLI purity claims; all affected client/query/wire/CLI consumers were rebuilt and executed.'},indent=2))
print(json.dumps({'candidate_files':len(paths),'retained':len(old),'delta_files':len(delta),'reused_commands':len(reused),'spec_preserved':True,'safefs_unchanged':True}))
