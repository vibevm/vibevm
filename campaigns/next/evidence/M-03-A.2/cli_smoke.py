from pathlib import Path
import os,json,subprocess,hashlib,tempfile
out=Path('/fast/git/v/research/2026-10-12-vibevm-first-agent-start/M03-A2')
binary=Path('/fast/git/v/.build/c4-first-agent-target/debug/vibe')
assert binary.is_file()
sha=lambda b:hashlib.sha256(b).hexdigest()
def inventory(root):
 return {str(p.relative_to(root)):{'kind':'directory'} if p.is_dir() else {'kind':'file','size':p.stat().st_size,'sha256':sha(p.read_bytes())} for p in sorted(root.rglob('*'))}
with tempfile.TemporaryDirectory(prefix='vibe-a2-cli-') as temp:
 base=Path(temp);root=base/'project';root.mkdir();id='chg-0123456789abcdef0123456789abcdef';d=root/'docs/changes'/id;d.mkdir(parents=True)
 (root/'vibe.toml').write_text("[project]\ngroup='org.demo'\nname='consumer'\nversion='1.0.0'\n")
 text=f"schema=1\n[change]\nid='{id}'\nselected_node='.'\nslug='fixture'\ntitle='Selected fixture'\n[[origins]]\npackage='feat:org.demo/origin@=1.0.0'\nmanifest_sha256='sha256:{'a'*64}'\n[[origins.documents]]\naddress='spec://org.demo/origin/FEAT#ONE'\nsha256='sha256:{'b'*64}'\n"
 for role,path,normative in [('intent','intent.md',True),('acceptance','acceptance.md',True),('work','work.toml',False)]:text+=f"[[documents]]\nrole='{role}'\npath='{path}'\nnormative={str(normative).lower()}\ninput=true\n"
 (d/'change.toml').write_text(text);(d/'intent.md').write_text('Selected fixture intent');(d/'acceptance.md').write_text('Independent acceptance');(d/'work.toml').write_text("schema=1\n[[items]]\nid='work-a'\n[[items]]\nid='work-b'\n")
 env=os.environ.copy()
 for key,name in [('HOME','home'),('XDG_CONFIG_HOME','config'),('XDG_CACHE_HOME','cache'),('XDG_DATA_HOME','data'),('XDG_STATE_HOME','state')]:
  path=base/name;path.mkdir();env[key]=str(path)
 env.update(RUSTUP_AUTO_INSTALL='0',CARGO_NET_OFFLINE='true')
 before=inventory(base);(out/'CLI-SMOKE-BEFORE.json').write_text(json.dumps(before,indent=2)+'\n')
 cases=[('default-preparation',[str(binary),'--json','change','apply',id,'--work','work-a','--spec-root','docs'],0),('missing-work',[str(binary),'--json','change','apply',id,'--work','work-missing','--spec-root','docs'],1),('no-execution-flag',[str(binary),'--json','change','apply',id,'--work','work-a','--execute'],2)]
 records=[]
 for name,argv,expected in cases:
  r=subprocess.run(argv,cwd=root,env=env,capture_output=True)
  (out/(name+'.stdout')).write_bytes(r.stdout);(out/(name+'.stderr')).write_bytes(r.stderr)
  after=inventory(base);(out/(name+'.after.json')).write_text(json.dumps(after,indent=2)+'\n')
  records.append({'name':name,'argv':argv,'cwd':str(root),'exit':r.returncode,'expected_exit':expected,'inventory_equal':before==after,'stdout_sha256':sha(r.stdout),'stderr_sha256':sha(r.stderr)})
  assert r.returncode==expected,(name,r.returncode,r.stderr.decode())
  assert before==after,name
  if name=='default-preparation':
   value=json.loads(r.stdout);assert value['operations']==['production','current-source-verification'];assert value['selected_work']=='work-a';assert value['production_request'] is None;assert value['execution_carriage']=='unavailable:m16-change-full-pass'
 (out/'CLI-SMOKE.json').write_text(json.dumps({'binary':str(binary),'binary_sha256':sha(binary.read_bytes()),'cases':records,'all_snapshots_equal':True},indent=2)+'\n')
 print('CLI smoke:3 processes, exits0/1/2; byte+directory inventories equal')
