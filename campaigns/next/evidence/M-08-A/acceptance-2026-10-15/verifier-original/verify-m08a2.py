#!/usr/bin/python3
"""Finite M08A2 metadata checks; not vendor/runtime qualification."""
from pathlib import Path
import sys,json,hashlib,xml.etree.ElementTree as ET,urllib.parse,subprocess
ROOT=Path(__file__).resolve().parent
REPO=Path('/fast/git/v/vibevm-next')
CLIENTS={'claude_code','codex','opencode','qwen_code'}
RESOURCES={'skill','mcp','config','plugin'}
SCENARIOS={'prepare_plan','permission_trust','apply_install','repeat_noop','owned_update','verify','interrupted_recovery','later_failure_inverse','uninstall_revocation','foreign_ownership','unsupported_version_platform','client_discovery'}
def require(ok,msg):
 if not ok:raise ValueError(msg)
def nonempty(v):return isinstance(v,str) and bool(v.strip())
def unique(rows,label):
 require(isinstance(rows,list) and rows,label+' missing');out={}
 for row in rows:
  require(isinstance(row,dict) and nonempty(row.get('id')),label+' ID');require(row['id'] not in out,label+' duplicate ID');out[row['id']]=row
 return out
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def xml_value(e):return (e.tag,sorted(e.attrib.items()),e.text or '',[xml_value(c) for c in e])
def main():
 path=Path(sys.argv[1]) if len(sys.argv)>1 else REPO/'fixtures/next/adapters/four-client-contracts.json'
 data=json.loads(path.read_text());require(data.get('schema')==1,'schema1 required')
 clients=unique(data.get('clients'),'clients');require(set(clients)==CLIENTS,'all four mandatory clients required')
 sources=unique(data.get('sources'),'sources');rows=unique(data.get('rows'),'rows');proofs=unique(data.get('proofs'),'proofs')
 for sid,s in sources.items():
  for k in ['url','title','fetched_at','applicability','revision_status']:require(nonempty(s.get(k)),sid+' missing '+k)
  u=urllib.parse.urlparse(s['url']);require(u.scheme=='https' and u.hostname,sid+' direct HTTPS URL required')
  require(s.get('official_owner_verified') is True,sid+' official ownership review required')
  if s.get('client_id')=='codex':require(u.hostname in {'developers.openai.com','platform.openai.com','learn.chatgpt.com'},sid+' OpenAI source domain')
 for cid,c in clients.items():
  require(nonempty(c.get('platform')),cid+' platform required');v=c.get('version',{});require(v.get('status') in ['observed','missing','skipped','failed'],cid+' version status')
  if v['status']=='observed':
   require(nonempty(v.get('exact')) and nonempty(v.get('executable_path')) and nonempty(v.get('executable_sha256')),cid+' exact observation identity')
   require(v.get('argv')==['--version'] and v.get('exit')==0,cid+' bounded version-only observation')
   require('HOME' not in v.get('override_names',[]),cid+' HOME must not be repurposed')
  else:require(nonempty(v.get('debt')),cid+' explicit missing/skipped/failed debt')
  require(c.get('runtime_support_proved') is False,cid+' A2 cannot claim runtime support')
 coverage=set();validated_proofs=0
 for rid,row in rows.items():
  cid=row.get('client_id');res=row.get('resource');scope=row.get('scope');require(cid in CLIENTS and res in RESOURCES and scope in ['project','user'],rid+' tuple')
  coverage.add((cid,res,scope))
  for k in ['operation','shape','target_root','target_path','value_grammar','version_applicability','host_authority','receipt_owner','physical_lock','positive_case','negative_case']:require(nonempty(row.get(k)),rid+' missing '+k)
  require(row.get('status') in ['source_characterized','proposed','unsupported'],rid+' status must not promote docs to runtime')
  refs=row.get('source_ids');require(isinstance(refs,list) and refs and all(x in sources for x in refs),rid+' source reference')
  ids=row.get('proof_ids');require(isinstance(ids,list) and ids and all(x in proofs for x in ids),rid+' proof references')
  found=set()
  for pid in ids:
   p=proofs[pid];require(rid in p.get('row_ids',[]),pid+' row binding');require(p.get('status')=='unexecuted',pid+' runtime proof must remain unexecuted')
   scenario=p.get('scenario');require(scenario in SCENARIOS,pid+' scenario');found.add(scenario)
   for k in ['fixture_prestate','operation_sequence','expected_resources','expected_receipt_or_refusal','oracle','proposed_test_owner','environment_binding']:require(nonempty(p.get(k)),pid+' concrete '+k)
   validated_proofs+=1
  require(found==SCENARIOS,rid+' missing lifecycle proof groups: '+str(SCENARIOS-found))
 require(coverage=={(c,r,s) for c in CLIENTS for r in RESOURCES for s in ['project','user']},'all client/resource/project-user tuples including unsupported rows required')
 before=ET.parse(ROOT/'before/PROP-054.xml').getroot();after=ET.parse(REPO/'vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml').getroot()
 require(before.tag==after.tag and before.attrib==after.attrib and before.text==after.text,'PROP054 root changed')
 require(len(after)>=len(before) and all(xml_value(a)==xml_value(b) for a,b in zip(before,after)), 'old PROP054 clauses/attributes/text changed')
 legacy=json.loads((ROOT/'legacy-fixtures-before.json').read_text());changed=[p for p,d in legacy.items() if not (REPO/p).exists() or sha(REPO/p)!=d]
 declared={x['path']:x for x in data.get('legacy_fixture_changes',[])}
 for p in changed:
  require(p in declared and nonempty(declared[p].get('reason')),p+' unexpected legacy change')
  if p.endswith('matrix.json'):continue
  checks=declared[p].get('affected_tests',[]);require(checks,p+' actual consumed fixture regression required')
  for c in checks:
   require(c.get('exit')==0 and c.get('passed',0)>0 and sha(c['log'])==c['log_sha256'],p+' actual regression receipt/log invalid')
   require('test result: ok.' in Path(c['log']).read_text(),p+' actual regression log lacks pass')
 for p in sorted((REPO/'campaigns/next/evidence/M-08-A').rglob('*.json')):json.loads(p.read_text())
 diff=subprocess.run(['git','diff','--check','--','fixtures/next/adapters','vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml','campaigns/next/evidence/M-08-A'],cwd=REPO,text=True,capture_output=True);require(diff.returncode==0,'scoped whitespace check failed: '+diff.stdout+diff.stderr)
 print(json.dumps({'kind':'M08A2_metadata_validation','clients':len(clients),'rows':len(rows),'sources':len(sources),'proof_bindings':validated_proofs,'scenario_groups':len(SCENARIOS),'old_PROP054_children_preserved':len(before),'legacy_changes':changed,'runtime_support_qualified':False,'checks':'JSON/XML/coverage/references/provenance separation/preservation/scoped whitespace'}))
if __name__=='__main__':
 try:main()
 except Exception as e:print(json.dumps({'kind':'M08A2_metadata_validation_failed','reason':str(e)}));raise SystemExit(1)
