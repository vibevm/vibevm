import pathlib,json,subprocess,os,hashlib,re,sys
home=pathlib.Path('/fast/git/v/research/2026-10-12-vibevm-first-agent-start/M02-D1')
build=home/sys.argv[1];test=home/sys.argv[2]
artifacts=[]
for line in build.read_text().splitlines():
 try:d=json.loads(line)
 except json.JSONDecodeError:continue
 if d.get('reason')=='compiler-artifact' and d.get('target',{}).get('name')=='vibe' and d.get('executable'):artifacts.append(d['executable'])
assert len(artifacts)==1
binary=pathlib.Path(artifacts[0]);digest=hashlib.sha256(binary.read_bytes()).hexdigest()
root=pathlib.Path(re.search(r'^CLI_SMOKE_ROOT (.+)$',test.read_text(),re.M).group(1));host=root/'host'
hermetic=home/'cli-smoke-homes';hermetic.mkdir(exist_ok=True)
env={'PATH':os.environ['PATH'],'HOME':str(hermetic/'home'),'XDG_CONFIG_HOME':str(hermetic/'config'),'XDG_CACHE_HOME':str(hermetic/'cache'),'XDG_STATE_HOME':str(hermetic/'state'),'VIBE_SETTINGS':str(hermetic/'settings'),'CARGO_NET_OFFLINE':'true','RUSTUP_AUTO_INSTALL':'0','LANG':'C.UTF-8'}
for key in ['HOME','XDG_CONFIG_HOME','XDG_CACHE_HOME','XDG_STATE_HOME','VIBE_SETTINGS']:pathlib.Path(env[key]).mkdir(parents=True,exist_ok=True)
def tree(base):return {str(p.relative_to(base)):None if p.is_dir() else {'bytes':p.stat().st_size,'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in sorted(base.rglob('*'))}
base=[str(binary),'--json','change','new','--from','flow:org.demo/dep@=1.0.0','--id','chg-0123456789abcdef0123456789abcdef','--spec-root','docs/custom','--path',str(host)]
runs=[]
def call(label,apply=False):
 cmd=base+(['--apply'] if apply else []);p=subprocess.run(cmd,cwd='/tmp',env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE)
 (home/(label+'.stdout.json')).write_bytes(p.stdout);(home/(label+'.stderr.log')).write_bytes(p.stderr)
 row={'label':label,'command':cmd,'cwd':'/tmp','exit':p.returncode,'binary':str(binary),'binary_sha256':digest};runs.append(row)
 return p,json.loads(p.stdout)
before=tree(root);homes_before=tree(hermetic)
p,a=call('cli-process-prepare');assert p.returncode==0 and a['status']=='prepared';assert tree(root)==before and tree(hermetic)==homes_before
prepare_after=tree(root)
p,b=call('cli-process-apply',True);assert p.returncode==0 and b['status']=='completed';assert a['prepared']==b['prepared']
for name,body in b['prepared']['files'].items():assert (host/b['prepared']['directory']/name).read_bytes()==body.encode()
after=tree(root)
for path,state in before.items():assert after[path]==state
p,c=call('cli-process-occupied',True);assert p.returncode==1 and c['status']=='refused' and c['code']=='occupied';assert tree(root)==after
assert tree(hermetic)==homes_before
record={'binary':str(binary),'binary_sha256':digest,'fixture':str(root),'environment':env,'runs':runs,'before':before,'prepare_after':prepare_after,'apply_after':after,'occupied_after':tree(root),'hermetic_before':homes_before,'hermetic_after':tree(hermetic),'prepared_equal_apply':a['prepared']==b['prepared'],'created_bytes_exact':True,'existing_bytes_and_directories_unchanged':True}
(home/'CLI-PROCESS-EVIDENCE.json').write_text(json.dumps(record,indent=2)+'\n');print(json.dumps({'binary_sha256':digest,'exits':[r['exit']for r in runs],'created':len(b['outcome']['verified']),'no_overwrite':True,'prepared_equal':True,'hermetic_unchanged':True}))
