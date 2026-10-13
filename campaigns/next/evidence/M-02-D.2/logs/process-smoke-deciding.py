import pathlib,json,subprocess,hashlib,copy
home=pathlib.Path('/fast/git/v/research/2026-10-12-vibevm-first-agent-start/M02-D2')
context=json.loads((home/'PROCESS-CONTEXT.json').read_text());home=home/'deciding-process';home.mkdir(exist_ok=True);root=pathlib.Path(context['root']);host=root/'host';binary=context['binary'];env=context['env'];env['VIBE_OFFLINE']='true'
ids=['chg-0123456789abcdef0123456789abcdef','chg-fedcba9876543210fedcba9876543210']
def snapshot():
 return {str(p.relative_to(root)):({'kind':'dir'} if p.is_dir() else {'kind':'file','bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}) for p in sorted(root.rglob('*'))}
def execute(name,args,cwd,stdin=None):
 cmd=[binary,*args];r=subprocess.run(cmd,cwd=cwd,env=env,input=stdin,capture_output=True,timeout=60)
 (home/(name+'.stdout')).write_bytes(r.stdout);(home/(name+'.stderr')).write_bytes(r.stderr)
 record={'argv':cmd,'cwd':str(cwd),'env':env,'exit':r.returncode,'binary_sha256':hashlib.sha256(pathlib.Path(binary).read_bytes()).hexdigest(),'stdout_sha256':hashlib.sha256(r.stdout).hexdigest(),'stderr_sha256':hashlib.sha256(r.stderr).hexdigest()}
 (home/(name+'.command.json')).write_text(json.dumps(record,indent=2)+'\n');return r
# Explicit second-change setup, before the commissioned read-only interval.
beta=host/'vibevm/vibespecs/changes'/ids[1]
if not beta.exists():
 r=execute('process-beta-setup',['--json','change','new','--from','flow:org.demo/dep@=1.0.0','--id',ids[1],'--slug','beta','--title','Independent beta','--path',str(host),'--apply'],host)
 assert r.returncode==0,r.stderr;assert json.loads(r.stdout)['status']=='completed'
 (beta/'refinement.md').write_text('Independent beta refinement bytes\n')
def semantic(value):
 value=copy.deepcopy(value)
 if value['status']=='success':
  value.pop('capture')
  for fact in value['facts']:
   if fact['scope']=='local':fact.pop('capture')
 return value
before=snapshot();(home/'PROCESS-BEFORE.json').write_text(json.dumps(before,indent=2)+'\n')
requests=[];cli=[];pairs=[]
for ident in ids:
 for mode in ['check','show','coverage']:
  request={'schema':1,'mode':mode,'id':ident};requests.append(request)
  # Default cwd, no --path. Relative-path counterpart separately below.
  r=execute('process-cli-'+ident[-4:]+'-'+mode,['--json','change',mode,ident],host)
  assert r.returncode==0,(r.stdout,r.stderr);value=json.loads(r.stdout);assert value['status']=='success';cli.append(value)
  after=snapshot();assert before==after; pairs.append({'subject':ident+'/'+mode,'before':before,'after':after})
r=execute('process-cli-relative',['--json','change','show',ids[0],'--path','host'],root)
assert r.returncode==0;assert semantic(json.loads(r.stdout))==semantic(cli[1]);assert before==snapshot()
requests.extend([{'schema':99,'mode':'check','id':ids[0]},{'schema':1,'mode':'check','id':ids[0],'root':'/elsewhere'},{'schema':1,'mode':'check','id':'../escape'},{'schema':1,'mode':'check','id':ids[0],'roles':['closure']}])
for n,request in enumerate(requests[6:]):
 r=execute('process-cli-refusal-'+str(n),['--json','change','check','--path','missing-dir' if n<3 else str(host),'--request-json',json.dumps(request)],host)
 assert r.returncode==1;cli.append(json.loads(r.stdout));assert cli[-1]['status']=='refused';assert before==snapshot()
messages=[{'jsonrpc':'2.0','id':1,'method':'initialize','params':{'protocolVersion':'2024-11-05','capabilities':{},'clientInfo':{'name':'owned-d2-conformance','version':'1'}}}]
messages += [{'jsonrpc':'2.0','id':n+2,'method':'tools/call','params':{'name':'change_query','arguments':req}} for n,req in enumerate(requests)]
stdin=('\n'.join(json.dumps(x) for x in messages)+'\n').encode();(home/'process-mcp.stdin').write_bytes(stdin)
r=execute('process-mcp',['--offline','mcp','serve','--path',str(host)],host,stdin)
assert r.returncode==0,(r.stdout,r.stderr)
replies=[json.loads(x) for x in r.stdout.splitlines()];assert len(replies)==len(messages),(len(replies),len(messages))
for n,(request,wanted,reply) in enumerate(zip(requests,cli,replies[1:])):
 result=reply['result'];actual=result['structuredContent'];assert semantic(wanted)==semantic(actual),(n,wanted,actual)
 assert result['isError']==(actual['status']=='refused')
 if actual['status']=='success':assert json.loads(result['content'][0]['text'])==actual
 assert before==snapshot();pairs.append({'subject':'mcp/'+str(n),'before':before,'after':snapshot()})
assert cli[0]['origins']==cli[3]['origins'];assert cli[0]['intent_hash']!=cli[3]['intent_hash'];assert cli[0]['scope_complete'] is True;assert cli[3]['scope_complete'] is False
for p in ['home','config','cache','state','settings']:assert list((root/'hermetic'/p).iterdir())==[],p
(home/'PROCESS-AFTER.json').write_text(json.dumps(snapshot(),indent=2)+'\n')
(home/'PROCESS-SNAPSHOT-PAIRS.json').write_text(json.dumps(pairs,indent=2)+'\n')
(home/'PROCESS-PARITY.json').write_text(json.dumps({'binary':binary,'binary_sha256':hashlib.sha256(pathlib.Path(binary).read_bytes()).hexdigest(),'requests':requests,'cli':cli,'mcp':replies,'semantic_pairs':len(requests),'snapshot_pairs':len(pairs),'default_cwd':True,'relative_path':True,'empty_hermetic_dirs':True,'no_effects_claim':'complete before/after owned byte+directory inventory; no universal syscall/all-platform claim'},indent=2)+'\n')
print(json.dumps({'pairs':len(requests),'snapshots':len(pairs),'binary_sha256':hashlib.sha256(pathlib.Path(binary).read_bytes()).hexdigest()}))
