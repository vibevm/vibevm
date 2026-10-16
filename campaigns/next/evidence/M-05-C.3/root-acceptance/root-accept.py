#!/usr/bin/env python3
"""Root-only ordinary HTTP acceptance phases. validate/inspect never submit commands; no database access."""
import pathlib,json,os,sys,hashlib,subprocess,importlib.util
os.umask(0o077);A=pathlib.Path(__file__).parent;R=A.parent;B=R;P=pathlib.Path('/fast/git/v/vibevm-next')
J='job.runtime.packet.vibevm-m05c3r';CID='managed-result.dc00d52268990eccb9c404a55b2279883918b76777b9e144a4f991ce4fe4a0ea';OP='managed.ecf57b8cdf291f352cbb547b9ce0e67938e9f38840f7ec200996c9c4c59633fa'
TOKEN='Root approves full M05C3 both criteria and 23 fresh cases, with reviewed safe boundary and no physical stop claim'
REVIEW_DIGEST=''
def read(p):return json.loads(pathlib.Path(p).read_text())
def digest(p):return hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()
def save(p,v):pathlib.Path(p).write_text(json.dumps(v,indent=2)+'\n')
pins=read(A/'OPERATOR-PINS.json')
for x in pins.values():assert digest(x['path'])==x['sha256'],x['path']
spec=importlib.util.spec_from_file_location('normal_http','/fast/zap-managed/r20/online.py');http=importlib.util.module_from_spec(spec);spec.loader.exec_module(http);http.R=B
def wire(mode,v):
 out=json.loads(subprocess.check_output([pins['wire']['path'],mode],input=http.dump(v)))
 # These are ordinary public DTO fields, not edits to canonical payload bytes or stored records.
 if mode=='prepare' and out['request'].get('actor'):out['request']['actor']['principal_id']='coordinator.vibevm-m05c3r'
 if mode in ['command','observation']:out['command']['frame']['reason']['summary']='Root approved full C3 and exact safe boundary; review SHA256='+REVIEW_DIGEST
 return out
http.wire=wire
def job():
 v=http.call('/v1/runtime/inspect',{'kind':'runtime_inspect','request':{'job_id':J,'limit':1}});rows=http.detail(v);assert len(rows)==1;return v,rows[0]
def collected(j):
 captured=read(A/'CURRENT-JOB.json')
 for k in ['job_id','work_id','attempt_id','dispatch_id','effect_id','candidate_id','producer','contract_id','contract_digest','relevant_basis','packet_id']:assert j[k]==captured[k],k
 n=http.detail(http.call('/v1/native',{'kind':'native_driver','request':{'kind':'managed_inspect','target':{'kind':'job','job_id':J,'dispatch_id':j['dispatch_id']}}},'coordinator'))
 assert j['candidate_id']==CID and j['execution']=='succeeded' and j['effect']=='completed'
 assert n['operation']==OP and n['collected'] and not n['unknown_tail'] and not n['failed'] and not n['accounting_pending']
 assert n['result']['status']['kind']=='bound' and n['result']['status']['candidate_id']==CID
 assert n['result']['status']==read(A/'COLLECTED-NATIVE.json')['result']['status']
 assert n['result']['native_turn']==read(A/'COLLECTED-NATIVE.json')['result']['native_turn']
 return n
def prepare_request(phase,kind,payload,identity,revision):
 return wire('prepare',{'id':'root-review.m05c3r.'+phase,'kind':kind,'payload':payload,'identity':identity,'revision':revision})
def submit(phase,kind,payload,role='coordinator',basis_required=False):
 receipt=A/'receipts'/(phase+'.json');request=A/'requests'/(phase+'.json')
 if receipt.exists():return read(receipt)
 if request.exists():req=read(request)
 else:
  v,_=job();basis=None;revision=v['revision'];identity=v['store']
  if basis_required:
   draft=prepare_request(phase,kind,payload,identity,revision)
   b=http.call('/v1/prepare/bundle',draft,'reader');save(A/'requests'/(phase+'.prepared.json'),b);revision=b['observed_revision']
   if b['request']['effects']:assert len(b['request']['effects'])==1;basis=b['request']['effects'][0]['relevant_before']
   else:assert b['request']['no_op_basis'] is not None;basis=b['request']['initial_basis']
  req=wire('observation' if role=='trusted' else 'command',{'identity':identity,'id':'root-review.m05c3r.'+phase,'kind':kind,'payload':payload,'revision':revision,'basis':basis});save(request,req)
 response=http.call('/v1/observation' if role=='trusted' else '/v1/command',req,role)
 assert response.get('status')=='committed','Uncertain command; retain exact request and retry same phase, never invent a replacement ID'
 save(receipt,response['receipt']);print(json.dumps({'phase':phase,'status':'committed','revision':response['receipt']['revision']}),flush=True);return response['receipt']
def validation(review):
 s=importlib.util.spec_from_file_location('review_validation',A/'validate-review.py');v=importlib.util.module_from_spec(s);s.loader.exec_module(v);return v.validate(review)
def encode_only(review):
 v,j=job();n=collected(j);rows=list(read(A/'STATIC-PAYLOADS.json').items())+[(x['phase'],x) for x in read(A/'EVIDENCE-PAYLOADS.json')]
 for phase,row in rows:
  cmd=wire('command',{'identity':v['store'],'id':'root-review.m05c3r.'+phase,'kind':row['kind'],'payload':row['payload'],'revision':v['revision'],'basis':None})
  prep=prepare_request(phase,row['kind'],row['payload'],v['store'],v['revision']);save(A/'requests'/('DRAFT-'+phase+'.json'),{'not_submitted':True,'command_DTO_without_prepared_basis':cmd,'normal_prepare_request':prep})
 provenance=wire('provenance',read(R/'service/managed.json'));assert provenance['capability_digest']==j['intent']['capability_digest']
 vid='verification.m05c3r.root-safe';report_digest=next(x['digest'] for x in read(R/'content-result.json')['artifacts'] if x['kind']=='report')
 safe_rows=[('safe.claim','runtime.verification-claimed',{'record':{'verification_id':vid,'job_id':J,'scope':{'kind':'safe_boundary_occurrence','declared_verification_id':j['intent']['safe_stop']['verifier'],'attempt_id':j['attempt_id'],'effect_id':j['effect_id'],'boundary':j['intent']['safe_stop']['boundary']},'state':'claimed','observation':None,'artifacts':[],'revision':v['revision']+1}},'command'),('safe.result','runtime.verification-result-recorded',{'verification_id':vid,'expected_record_revision':v['revision']+1,'state':'passed','observation':provenance['observation'],'artifacts':[report_digest],'provenance':provenance},'observation'),('safe.record','runtime.safe-state-recorded',{'job_id':J,'expected_job_revision':j['revision'],'safe_state':'safe','observation':provenance['observation'],'evidence':[vid],'provenance':provenance},'observation')]
 for phase,kind,payload,mode in safe_rows:
  cmd=wire(mode,{'identity':v['store'],'id':'root-review.m05c3r.'+phase,'kind':kind,'payload':payload,'revision':v['revision'],'basis':None});save(A/'requests'/('DRAFT-'+phase+'.json'),{'not_submitted':True,'root_approval_required':True,'approved_Root_review_sha_bound_in_protected_reason':True,'revisions_bound_from_actual_prior_phase_receipts_at_execution':True,'command_DTO':cmd})
 return {'status':'READY_NOT_APPLIED','typed_payloads_encoded':len(rows)+len(safe_rows),'commands_submitted':0,'canonical_revision':v['revision'],'job_id':J,'candidate_id':CID,'safe':j['safe'],'runtime_acceptance_projection':j['acceptance'],'native_stopped':n['process_stopped'],'canonical_work_state':http.record('work',j['work_id'])['state']}
def main():
 global REVIEW_DIGEST
 phase=sys.argv[1]
 if phase=='inspect':
  v,j=job();w=http.record('work',j['work_id']);print(json.dumps({'revision':v['revision'],'job_id':J,'candidate_id':j['candidate_id'],'safe':j['safe'],'runtime_acceptance_projection':j['acceptance'],'canonical_work_state':w['state']}));return
 if phase=='stop-observe':
  v,j=job();native=collected(j)
  value={'revision':v['revision'],'job':{k:j.get(k) for k in ['job_id','candidate_id','revision','execution','effect','safe','acceptance','stop_receipt']},'native':{k:native.get(k) for k in ['operation','state','source_sequence','collected','unknown_tail','failed','process_stopped','accounting_pending','result']}}
  save(A/'FINAL-STOP-OBSERVATION.json',value);print(json.dumps({'job_revision':j['revision'],'delivery':(j.get('stop_receipt') or {}).get('delivery'),'process_stopped':native['process_stopped'],'unknown_tail':native['unknown_tail'],'failed':native['failed']}));return
 review_path=pathlib.Path(sys.argv[2]);review=read(review_path)
 result=validation(review)
 if phase=='validate':
  assert review['approved'] is False;out=encode_only(review);save(A/'VALIDATION.json',{'evidence_validation':result,'public_API_and_typed_encoding':out});print(json.dumps(out));return
 assert review['approved'] is True and review['approval_token']==TOKEN,'Explicit final Root approval required'
 assert review['candidate_id']==CID and review['job_id']==J
 assert review.get('independent_review') and review.get('independent_review_reviewed_by_root') is True,'Root final independent review must be bound before application'
 assert review['safe_boundary_attestation']['approved'] is True,'Explicit Root safe-boundary approval required'
 REVIEW_DIGEST=digest(review_path)
 approval_path=A/'ROOT-APPROVAL-BINDING.json'
 if approval_path.exists():assert read(approval_path)['sha256']==REVIEW_DIGEST,'Do not alter the signed review between phases'
 else:save(approval_path,{'path':str(review_path),'sha256':REVIEW_DIGEST,'physical_stop_claim':False})
 v,j=job();collected(j)
 if phase=='stop':
  assert http.record('work',j['work_id'])['state']=='accepted','Canonical WorkAccepted must precede this retirement request'
  path=A/'requests/managed-stop.json'
  if path.exists():request=read(path)
  else:
   _,current=job()
   request={'kind':'native_driver','request':{'kind':'managed_stop','request':{'request_id':'root.m05c3r.stop.after-accepted','job_id':J,'dispatch_id':current['dispatch_id'],'effect_id':current['effect_id'],'expected_revision':current['revision'],'reason':'Root normal retirement after canonical C3 WorkAccepted; preserve old stopped no-candidate attempt and all evidence.'}}}
   save(path,request)
  response=http.call('/v1/native',request,'coordinator');save(A/'receipts/managed-stop.json',response)
  native=http.detail(response);print(json.dumps({k:native.get(k) for k in ['state','source_sequence','collected','failed','unknown_tail','process_stopped','accounting_pending']}));return
 if phase=='safe':
  cfg=read(R/'service/managed.json');provenance=wire('provenance',cfg);assert provenance['capability_digest']==j['intent']['capability_digest']
  vid='verification.m05c3r.root-safe';claim={'record':{'verification_id':vid,'job_id':J,'scope':{'kind':'safe_boundary_occurrence','declared_verification_id':j['intent']['safe_stop']['verifier'],'attempt_id':j['attempt_id'],'effect_id':j['effect_id'],'boundary':j['intent']['safe_stop']['boundary']},'state':'claimed','observation':None,'artifacts':[],'revision':v['revision']+1}}
  receipt=submit('safe.claim','runtime.verification-claimed',claim)
  report_digest=next(x['digest'] for x in read(R/'content-result.json')['artifacts'] if x['kind']=='report')
  payload={'verification_id':vid,'expected_record_revision':receipt['revision'],'state':'passed','observation':provenance['observation'],'artifacts':[report_digest],'provenance':provenance}
  submit('safe.result','runtime.verification-result-recorded',payload,'trusted')
  _,j=job()
  if j['safe'] not in ['safe','completed']:submit('safe.record','runtime.safe-state-recorded',{'job_id':J,'expected_job_revision':j['revision'],'safe_state':'safe','observation':provenance['observation'],'evidence':[vid],'provenance':provenance},'trusted')
  return
 assert j['safe'] in ['safe','completed'] and read(A/'ROOT-APPROVAL-BINDING.json')['sha256']==digest(review_path)
 assert (A/'receipts/safe.result.json').exists(),'Normal safe verification receipt required first'
 if phase=='evidence':
  for row in read(A/'EVIDENCE-PAYLOADS.json'):submit(row['phase'],row['kind'],row['payload'],basis_required=True)
 elif phase in ['open-review','stage','work']:
  row=read(A/'STATIC-PAYLOADS.json')[phase];submit(phase,row['kind'],row['payload'],basis_required=True)
 else:raise SystemExit('validate/inspect/safe/open-review/evidence/stage/work/stop/stop-observe')
if __name__=='__main__':main()
