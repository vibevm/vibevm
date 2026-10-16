#!/usr/bin/env python3
"""Read-only D1 evidence validation; preserves failed producer V4 and separate Root gate."""
import pathlib,json,hashlib,subprocess
A=pathlib.Path(__file__).parent;D=A.parent;P=pathlib.Path('/fast/git/v/vibevm-next')
def read(p):return json.loads(pathlib.Path(p).read_text())
def h(p):return hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()
def validate(review):
 for row in review['evidence_files'].values():assert h(row['path'])==row['sha256'],row['path']
 commit=read(D/'ROOT-CHECKPOINT-COMMIT.json');assert commit['commit']=='cecd8552725a3fc17dd9139b1eabf0c5910db4ef'
 assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=P,text=True).strip()==commit['commit']
 for p,d in commit['source_files'].items():assert h(P/p)==d,p
 peer=read('/fast/git/v/research/2026-10-16-m05d1-review/REPORT.json')
 assert peer['blocking_semantic_findings']==[] and peer['source_manifest_mismatches']==[] and peer['log_hash_mismatches']==[]
 allowed=set(peer['earlier_snapshot_differences']);assert allowed=={'crates/vibe-install/src/apply.rs','crates/vibe-install/src/record.rs','crates/vibe-install/src/record/m_05_d_1_cases.rs'}
 source_scope={};total=0
 for i,count in enumerate([3,3,2,2,0,0,0],1):
  v=read(D/f'V{i}-result.json');assert v['fresh_tests']==count and v['passed'] is (False if i==4 else True)
  assert read(D/f'V{i}-subjects-before.json')==read(D/f'V{i}-subjects-after.json')
  mismatches=set();actual=0
  for c in v['commands']:
   assert c['source_drift']==[] and h(c['log'])==c['log_sha256']
   expected_exit=1 if c['argv']==['cargo','xtask','check-codegen'] else 0
   assert c['exit']==expected_exit
   for p,d in c['source_hashes'].items():
    if h(P/p)!=d:mismatches.add(p)
   for row in c['counts']:assert int(row[1])==0;actual+=int(row[0])
   if i==5:assert c['argv'][0]=='rustfmt' and '--check' in c['argv']
  assert actual==count;total+=actual
  assert mismatches==(allowed if i in [2,3,4] else set()),(i,mismatches)
  source_scope[f'V{i}']=sorted(mismatches)
 v=read(D/'V4-result.json');assert [c['exit'] for c in v['commands']]==[0,0,0,1]
 assert h(D/'V4-result.json')==h(A/'producer-V4-original.json')
 gate=read(D/'root-postcommit/RESULT.json');cmd=read(D/'root-postcommit/COMMAND.json')
 assert gate['exit_code']==0 and gate['source_drift']==[] and gate['git_status']=='' and gate['source_head']==commit['commit']==cmd['source_head']
 assert cmd['argv']==['/home/olegchir/.cargo/bin/cargo','xtask','check-codegen'] and h(D/'root-postcommit/check-codegen.log')==gate['log_sha256']
 assert read(D/'V7-result.json')['extra']['structural_only'] is True
 assert len(review['criterion_assessments'])==2 and len(review['observations'])==7
 return {'valid':True,'passing_test_cases':total,'producer_V4_passed':False,'root_postcommit_codegen_exit':0,'root_postcommit_seconds':gate['seconds'],'qualified_earlier_snapshot_differences':source_scope,'new_tests_builds':0,'commands_submitted':0,'global_fmt_pass_claim':False}
