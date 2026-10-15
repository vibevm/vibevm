#!/usr/bin/env python3
"""Owned fixture and Linux query observation; no product admission logic."""
import hashlib,json,os,re,select,subprocess,time
from pathlib import Path
R=Path('/fast/zap-managed/2026-10-15-m08c1')
B=Path(json.loads((R/'V7-binary-before-V3.json').read_text())['binary'])
base=json.loads((R/'V7-binary-before-V3.json').read_text())['sha256']
assert hashlib.sha256(B.read_bytes()).hexdigest()==base
owned=R/'same-binary-fixture3';owned.mkdir(exist_ok=False)
root=owned/'project';root.mkdir()
for name in ['home','settings','config','cache','cwd','tmp']:(owned/name).mkdir()
host="[project]\nname='fixture'\nversion='1.0.0'\n"
(root/'vibe.toml').write_text(host)
descriptor=Path('fixtures/next/adapters/m-08-c1/client.toml').read_text()
env=os.environ.copy();env.update(HOME=str(owned/'home'),USERPROFILE=str(owned/'home'),VIBE_SETTINGS=str(owned/'settings'),XDG_CONFIG_HOME=str(owned/'config'),XDG_CACHE_HOME=str(owned/'cache'),TMPDIR=str(owned/'tmp'))
def census():
 return {str(p.relative_to(owned)):('directory' if p.is_dir() else hashlib.sha256(p.read_bytes()).hexdigest()) for p in sorted(owned.rglob('*'))}
def line(child):
 ready,_,_=select.select([child.stdout],[],[],30)
 if not ready:raise RuntimeError('bounded reply timeout')
 data=child.stdout.readline()
 if not data:raise RuntimeError('unexpected EOF')
 return json.loads(data)
def phase(label,want):
 before=census();trace=R/f'same-binary-{label}.trace';err=R/f'same-binary-{label}.stderr'
 argv=['/usr/bin/strace','-f','-s','65536','-o',str(trace),'-e','trace=%file,%process,%network,%desc',str(B),'--offline','adapter-plan','--root',str(root),'--kind','clients','--prepare']
 with err.open('wb') as stderr:
  child=subprocess.Popen(argv,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=stderr,env=env,cwd=owned/'cwd',text=True,bufsize=1)
  first=line(child);payload=json.loads(first['body']['payload_json']);kind=first['body']['kind'] if 'kind' in first['body'] else first['body'].get('stage')
  request={'schema':1,'operation_id':'rust:vibe-orchestrator:client-capabilities','domain_owner':'rust:vibe-orchestrator','evidence_id':None,'source_id':'captured-client-capabilities/1','body':{'stage':'input','format_id':'client-capability-request','payload_json':json.dumps({'schema':1,'context_id':payload['context_id'],'target_id':payload['target_id'],'action':'read'})}}
  answers=[first]
  for _ in range(2):
   child.stdin.write(json.dumps(request)+'\n');child.stdin.flush();answers.append(line(child))
  child.stdin.close();code=child.wait(timeout=30)
 assert code==0,(label,code,err.read_text())
 assert census()==before,(label,'census drift')
 assert hashlib.sha256(B.read_bytes()).hexdigest()==base
 for answer in answers:
  body=answer['body'];assert body['effect_state']=='no_effects',body
  if want=='refusal':assert body.get('stage')=='error',answer
  else:
   assert body.get('stage')=='result',answer
   value=json.loads(body['payload_json']);caps=value['capabilities']
   assert bool(caps)==(want=='selected'),caps
   if want=='selected':
    c=caps['org.example/new-client'];assert c['bindings'][0]['destination']=='.new-client/settings.json' and c['bindings'][0]['key']==['mcpServers','new.key'];assert c['authorization']=='not-granted'
 lines=trace.read_text().splitlines();out=[i for i,s in enumerate(lines) if re.search(r'\bwrite\(1,',s) and '\\n"' in s]
 assert len(out)==3,(label,'three protocol writes',len(out))
 intervals=[]
 for a,b in zip(out,out[1:]):
  selected=lines[a+1:b]
  effects={k:[] for k in ['process','network','mutation','lock']}
  for s in selected:
   if re.search(r'\b(clone|clone3|fork|vfork|execve|execveat)\(',s):effects['process'].append(s)
   if re.search(r'\b(socket|socketpair|connect|bind|listen|accept|accept4|sendto|sendmsg|sendmmsg|recvfrom|recvmsg|recvmmsg|shutdown)\(',s):effects['network'].append(s)
   if re.search(r'\b(mkdir|mkdirat|rmdir|unlink|unlinkat|rename|renameat|renameat2|link|linkat|symlink|symlinkat|chmod|fchmod|fchmodat|chown|fchown|truncate|ftruncate|fallocate|utime|utimes|utimensat|futimesat|mknod|mknodat|fsync|fdatasync)\(',s) or re.search(r'\bopen(at|at2)?\(.*\bO_(WRONLY|RDWR|CREAT|TRUNC|APPEND|TMPFILE)\b',s) or re.search(r'\b(write|writev|pwrite64|pwritev|pwritev2)\((?![12],)\d+,',s):effects['mutation'].append(s)
   if re.search(r'\b(sendfile|copy_file_range|splice|tee)\(',s):effects['mutation'].append(s)
   if re.search(r'\bflock\(|\bfcntl\(.*F_(OFD_)?SETLK',s):effects['lock'].append(s)
  assert not any(effects.values()),(label,effects)
  intervals.append({'trace_lines':len(selected),'counters':{k:len(v) for k,v in effects.items()},'grant_issuance':'no grant-issuer call in the source-bound query path; not a syscall-observable counter'})
 result={'phase':label,'argv':argv,'exit':code,'binary_sha256':base,'fixture_census':before,'census_unchanged':True,'answers':answers,'query_intervals':intervals,'trace_sha256':hashlib.sha256(trace.read_bytes()).hexdigest()}
 results.append(result);(R/'same-binary-proof.json').write_text(json.dumps({'binary_sha256':base,'phases':results,'effects':'owned fixture setup completed; measured queries separate'},indent=2)+'\n')
results=[]
phase('before','empty')
(root/'unselected').mkdir();(root/'unselected/vibe.toml').write_text(descriptor)
phase('unselected','empty')
slot=root/'vibevm/vibedeps/org.example.clients';slot.mkdir(parents=True);(slot/'vibe.toml').write_text(descriptor)
(root/'vibe.toml').write_text(host+"\n[requires]\npackages={ 'org.example/clients' = '=1.0.0' }\n")
(root/'vibe.lock').write_text("[meta]\ngenerated_by='m08c1 fixture'\ngenerated_at='2026-10-15T00:00:00Z'\nschema_version=7\n[[package]]\nkind='tool'\nname='clients'\ngroup='org.example'\nversion='1.0.0'\nsource_url='fixture'\ncontent_hash='sha256:"+'a'*64+"'\nmaterialization='in-place'\n")
(root/'.new-client').mkdir();(root/'.new-client/present').write_text('inert presence marker; no authority')
phase('selected','selected')
(slot/'vibe.toml').write_text(descriptor+'\n[[mechanism]]'+descriptor.split('[[mechanism]]')[1].replace("id = 'client'","id = 'duplicate'"))
phase('duplicate','refusal')
(slot/'vibe.toml').write_text(descriptor.replace('reconcile-json-member','launch-arbitrary'))
phase('unknown-operation','refusal')
(slot/'vibe.toml').write_text(descriptor.replace('schema = 1','schema = 2'))
phase('unknown-epoch','refusal')
print(json.dumps({'passed':True,'phases':len(results),'query_intervals':sum(len(p['query_intervals']) for p in results),'binary_sha256':base}))
