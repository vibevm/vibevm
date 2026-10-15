"""Narrow structural guards for reviewed M08A2 F1/F2/F3; no runtime proof."""
from pathlib import Path
import json,hashlib

def semantic_validate(data,root,repo):
 def need(ok,msg):
  if not ok:raise ValueError(msg)
 def text(x):return isinstance(x,str) and bool(x.strip())
 def digest(b):return hashlib.sha256(b).hexdigest()
 clients={c['id']:c for c in data['clients']};known=json.loads((root/'OPERATOR-PROBE-BINDINGS.json').read_text())['known']
 for cid,c in clients.items():
  v=c['version'];need(text(v.get('discovery_scope')),cid+' discovery_scope required');need(v.get('machine_wide_absence_claimed') is False,cid+' no machine-wide absence inference')
  if cid not in known:continue
  k=known[cid];path=Path(v.get('probe_record',''));need(path.is_file(),cid+' actual bounded probe record required for supplied ELF')
  need(path.resolve().is_relative_to(root.resolve()) or path.resolve().is_relative_to((repo/'campaigns/next/evidence/M-08-A').resolve()),cid+' probe record outside authorized evidence roots');raw=path.read_bytes();need(digest(raw)==v.get('probe_record_sha256'),cid+' probe record digest');p=json.loads(raw)
  need(p.get('executable_path')==k['executable_path'] and p.get('executable_sha256')==k['executable_sha256'],cid+' exact known ELF binding')
  need(p.get('argv')==['--version'] and p.get('probe_executed') is True,cid+' actual version-only attempt')
  need(0<p.get('timeout_seconds',0)<=15 and p.get('stdin_closed') is True,cid+' bounded closed-stdin probe')
  need('HOME' not in p.get('override_names',[]) and k['private_override'] in p.get('override_names',[]),cid+' documented private root/no HOME repurpose')
  need(text(p.get('cwd')) and text(p.get('platform')),cid+' probe environment binding')
  need(len(p.get('stdout','').encode())<=65536 and len(p.get('stderr','').encode())<=65536,cid+' output bound')
  if p.get('exit')==0:
   need(v.get('executable_path')==k['executable_path'] and v.get('executable_sha256')==k['executable_sha256'],cid+' version observation exact binary');need(v.get('status')=='observed' and text(v.get('exact')) and v['exact'] in p.get('stdout',''),cid+' observed reply, not filename version')
  else:need(v.get('status') in ['failed','skipped'] and text(v.get('debt')),cid+' truthful failed probe debt')
 proofs={p['id']:p for p in data['proofs']}
 def receipt(x,label):
  need(isinstance(x,dict) and x.get('state') in ['none','active','retired','foreign','unknown'],label+' receipt state')
  need((x.get('generation') is None) if x['state'] in ['none','unknown'] else isinstance(x.get('generation'),int) and x['generation']>0,label+' generation')
 def same(a,b):return a==b
 def active(x,n):return x.get('state')=='active' and x.get('generation')==n
 for row in data['rows']:
  rid=row['id'];cid=row['client_id'];vb=row.get('version_binding',{});need('observed_exact' in vb and 'proposed_exact' in vb and text(vb.get('documentation_applicability')),rid+' separate version binding')
  v=clients[cid]['version'];need(vb['observed_exact']==(v.get('exact') if v['status']=='observed' else None),rid+' observed version mismatch')
  if cid in known and v['status']=='observed':need(vb['proposed_exact']==v['exact'] and v['exact'] in row['version_applicability'],rid+' proposed exact observed target required')
  if vb['proposed_exact'] is None:need(text(vb.get('target_version_debt')),rid+' exact target version debt')
  if cid=='qwen_code' and row['resource']=='plugin' and row['scope']=='user':
   need(row.get('activation_scope')=='user' and text(row.get('lock_binding_debt') or row.get('scope_specific_lock')),rid+' user lock/store binding or precise debt')
   need('project activation store' not in row['physical_lock'].lower(),rid+' copied project lock is invalid')
  own=row.get('ownership');required=row['resource']=='skill' or rid=='opencode.plugin.user.projected-resources'
  if required:
   need(isinstance(own,dict) and own.get('mode') in ['enumerated','tree_root'],rid+' explicit ownership footprint')
   members=own.get('members',[]);need(len(members)>=2 and len({m['path'] for m in members})==len(members),rid+' complete distinct member footprint')
   need(own.get('physical_locks') and all(text(x) for x in own['physical_locks']),rid+' explicit physical locks')
   for m in members:
    need(text(m.get('id')) and text(m.get('oracle')) and text(m.get('physical_lock')),rid+' member identity/oracle/lock')
    for version in ['v1','v2']:
     value=m.get(version)
     if value is not None:need(isinstance(value.get('bytes'),str) and digest(value['bytes'].encode())==value.get('sha256'),rid+' exact member bytes/hash')
   for version in ['v1','v2']:
    expected=sorted([{'path':m['path'],'sha256':m[version]['sha256']} for m in members if m.get(version) is not None],key=lambda x:x['path'])
    need(own.get('manifests',{}).get(version)==expected,rid+' exact '+version+' manifest')
    ids=sorted([m['id'] for m in members if m.get(version) is not None]) if own['mode']=='enumerated' else [own.get('root_id')]
    need(own.get('receipt_resources',{}).get(version)==ids,rid+' full '+version+' receipt footprint')
   if own['mode']=='tree_root':need(text(own.get('root_id')) and text(own.get('root_path')) and row.get('logical_resource')==own['root_id'],rid+' entry is not tree ownership root')
   need(own.get('foreign_siblings') and all(text(x) for x in own['foreign_siblings']),rid+' preserve foreign siblings')
   if row['resource']=='skill':
    by={Path(m['path']).name:m for m in members};need('SKILL.md' in by and 'reference.txt' in by,rid+' entry+reference required')
    need(by['reference.txt'].get('v1') is not None and by['reference.txt'].get('v2') is None,rid+' v2 removes only prior-owned reference')
    need(by['SKILL.md']['v1']['bytes']==row['example_bytes_v1'] and by['SKILL.md']['v2']['bytes']==row['example_bytes_v2'],rid+' entry examples bind footprint')
   else:need(isinstance(row['target_path'],list) and sorted(row['target_path'])==sorted(m['path'] for m in members) and len(members)==2,rid+' two actual resources, not concatenated pseudo-path')
  for pid in row['proof_ids']:
   p=proofs[pid];scenario=p['scenario'];cases=p.get('receipt_cases',[]);need(cases,pid+' phase-specific receipt cases required');outcomes=set();uninstall_versions=set()
   for c in cases:
    before=c.get('initial_receipt');after=c.get('expected_receipt');receipt(before,pid);receipt(after,pid);outcome=c.get('outcome');need(outcome in ['success','refused','recovery_required'],pid+' outcome');outcomes.add(outcome)
    for key in ['id','fixture_prestate','initial_target','expected_target','receipt_oracle','resource_oracle']:need(text(c.get(key)),pid+' case '+key)
    if outcome!='success':need(same(before,after) and c.get('ownership_acquired') is False,pid+' refusal preserves prior receipt/no acquisition')
    if row['status']=='unsupported':need(outcome!='success' and c.get('ownership_acquired') is False,pid+' unsupported tuple cannot acquire')
    elif scenario=='prepare_plan':need(before['state']=='none' and same(before,after) and c.get('receipt_write') is False,pid+' pure plan NONE→NONE')
    elif scenario in ['permission_trust','foreign_ownership','unsupported_version_platform']:need(same(before,after) and c.get('ownership_acquired') is False,pid+' no acquisition')
    elif scenario=='apply_install' and outcome=='success':need(before['state']=='none' and active(after,1),pid+' install NONE→generation1')
    elif scenario=='repeat_noop' and outcome=='success':need(active(before,1) and same(before,after) and c.get('receipt_write') is False,pid+' repeat retained1')
    elif scenario=='owned_update' and outcome=='success':need(active(before,1) and active(after,2),pid+' update1→2')
    elif scenario=='verify':need(same(before,after) and c.get('receipt_write') is False,pid+' verify cannot mutate receipt')
    elif scenario=='interrupted_recovery':need(text(c.get('checkpoint')) and text(c.get('checkpoint_expected_transition')),pid+' checkpoint-specific transition')
    elif scenario=='later_failure_inverse' and outcome=='success':
     image=c.get('before_image',{});need(text(image.get('resources_digest')) and image.get('receipt')==after,pid+' actual before-image receipt restoration')
    elif scenario=='uninstall_revocation' and outcome=='success':
     need(before['state']=='active' and after['state'] in ['none','retired'] and c['expected_target']=='absent',pid+' explicit retired/removed terminal');uninstall_versions.add(c.get('initial_owned_version'))
     if after['state']=='retired':need(after['generation']==before['generation'],pid+' retirement retains exact last generation')
    elif scenario=='client_discovery' and outcome=='success':
     phases={z['phase']:z['receipt'] for z in c.get('phase_receipts',[])};need(active(phases.get('after_install',{}),1) and active(phases.get('after_update',{}),2) and phases.get('after_remove',{}).get('state') in ['none','retired'],pid+' discovery lifecycle phase receipts')
    if required and row['status']!='unsupported':
     if scenario=='prepare_plan':need(c.get('planned_resources')==own['receipt_resources']['v1'],pid+' plan full footprint')
     if scenario=='apply_install' and outcome=='success':need(c.get('expected_owned_version')=='v1' and c.get('expected_resources')==own['receipt_resources']['v1'],pid+' install full footprint')
     if scenario=='owned_update' and outcome=='success':
      removed=sorted(m['path'] for m in own['members'] if m.get('v1') is not None and m.get('v2') is None);need(c.get('expected_resources')==own['receipt_resources']['v2'] and c.get('removed_members')==removed,pid+' update only prior-owned removals')
     if scenario=='uninstall_revocation' and outcome=='success':
      v=c.get('initial_owned_version');need(v in ['v1','v2'] and c.get('removed_members')==[x['path'] for x in own['manifests'][v]],pid+' full corresponding uninstall footprint')
   if row['status']!='unsupported' and scenario=='owned_update':need({'success','refused'}<=outcomes,pid+' split update success/refused expansion')
   if required and row['resource']=='skill' and scenario=='uninstall_revocation':need(uninstall_versions=={'v1','v2'},pid+' uninstall after v1 AND v2')
   if any(c['initial_receipt']['state']=='active' for c in cases):need('absent; no receipt' not in p['fixture_prestate'],pid+' contradictory generic absent/no-receipt prestate')
   if any(c['expected_receipt'].get('generation')!=1 or c['expected_receipt']['state']!='active' for c in cases):need(not p['expected_receipt_or_refusal'].startswith('One deployment receipt'),pid+' contradictory unconditional generation1 prose')
