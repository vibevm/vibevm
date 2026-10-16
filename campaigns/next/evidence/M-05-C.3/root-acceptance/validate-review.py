#!/usr/bin/env python3
"""Read-only checks of retained C3 evidence; no tests, DB access or runtime controls."""
import pathlib,json,hashlib
A=pathlib.Path(__file__).parent;D=A.parent;P=pathlib.Path('/fast/git/v/vibevm-next')
def read(p):return json.loads(pathlib.Path(p).read_text())
def h(p):return hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()
def validate(review):
 for row in review['evidence_files'].values():assert h(row['path'])==row['sha256'],row['path']
 for path,d in read(A/'FINAL-SOURCE-HASHES.json').items():assert h(P/path)==d,path
 counts=[4,4,2,13,0,0,0]
 for i,count in enumerate(counts,1):
  v=read(D/f'V{i}-result.json');assert v['passed'] is True and v['fresh_tests']==count
  assert read(D/f'V{i}-subjects-before.json')==read(D/f'V{i}-subjects-after.json')
  actual=0
  for c in v['commands']:
   assert c['exit']==0 and c['source_drift']==[] and h(c['log'])==c['log_sha256']
   for p,d in c['source_hashes'].items():assert h(P/p)==d,p
   for row in c['counts']:assert int(row[1])==0;actual+=int(row[0])
   if i==5:assert c['argv'][0]=='rustfmt' and '--check' in c['argv']
  assert actual==count
 assert read(D/'V7-result.json')['extra']['structural_only'] is True
 report=read(D/'REPORT.json');assert report['final_source_tests']==23
 for attempt in report['fresh_failed_attempts']:
  assert h(attempt['result']['path'])==attempt['result']['sha256']
  for c in attempt['commands']:assert c['exit']==101 and h(c['log']['path'])==c['log']['sha256']
 assert len(review['criterion_assessments'])==2 and len(review['observations'])==7
 if review.get('independent_review'):
  row=review['independent_review'];assert h(row['path'])==row['sha256']
 return {'valid':True,'fresh_final_cases':23,'fresh_failed_attempts_retained':len(report['fresh_failed_attempts']),'historical_attempt_retained':True,'qualified_reused_cases':0,'new_tests_run':0,'commands_submitted':0,'global_fmt_pass_claim':False}
