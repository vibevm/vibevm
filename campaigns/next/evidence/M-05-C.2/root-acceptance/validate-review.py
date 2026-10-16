#!/usr/bin/env python3
"""Read-only retained-evidence validation. No tests, formatter, compiler, DB or runtime controls."""
import pathlib,json,hashlib
A=pathlib.Path(__file__).parent;D=A.parent;B=D.parent;P=pathlib.Path('/fast/git/v/vibevm-next')
def read(p):return json.loads(pathlib.Path(p).read_text())
def h(p):return hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()
def validate(review):
 for row in review['evidence_files'].values():assert h(row['path'])==row['sha256'],row['path']
 for path,d in read(D/'FINAL-SOURCE-HASHES.json').items():assert h(P/path)==d,path
 v=read(D/'V1-result.json');assert v['passed'] is True and v['fresh_tests']==7
 assert read(D/'V1-subjects-before.json')==read(D/'V1-subjects-after.json')
 for c in v['commands']:
  assert c['exit']==0 and c['counts']==[['7','0','0']] and c['source_drift']==[] and h(c['log'])==c['log_sha256']
  for p,d in c['source_hashes'].items():assert h(P/p)==d,p
 reds=read(D/'RED-results.json');assert len(reds)==2
 for c in reds:assert c['exit']==101 and h(c['log'])==c['sha256'] and c['source_before']==c['source_after']
 reused=0
 for g,count in [('V2',3),('V3',2),('V4',3)]:
  v=read(D/(g+'-result.json'));assert v['executed'] is False and v['fresh_tests']==0 and v['passed'] is None
  assert len(v['reused_evidence'])==count
  for item in v['reused_evidence']:
   assert h(item['path'])==item['sha256'] and h(item['prior_receipt'])==item['prior_receipt_sha256']
   prior=read(item['prior_receipt']);matches=[x for x in prior['commands'] if x['argv']==item['argv'] and x['log']==item['path']];assert len(matches)==1
   assert matches[0]['exit']==0 and matches[0]['counts']==item['counts']==[['1','0','0']]
   for p,d in item['source_hashes'].items():assert h(P/p)==d,p
   reused+=1
 for g in ['V5','V6']:
  v=read(D/(g+'-result.json'));assert v['passed'] is True
  for c in v['commands']:
   assert c['exit']==0 and c['source_drift']==[] and h(c['log'])==c['log_sha256']
   if g=='V5':assert c['argv'][0]=='rustfmt' and '--check' in c['argv']
 assert read(D/'V7-result.json')['extra']['structural_only'] is True
 assert len(review['criterion_assessments'])==2 and len(review['observations'])==7
 return {'valid':True,'fresh_final_green_cases':7,'historical_red_failures':2,'qualified_reused_cases':reused,'new_tests_run':0,'commands_submitted':0,'global_fmt_pass_claim':False}
