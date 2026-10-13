import pathlib,tempfile,subprocess,os,json,hashlib,sys
home=pathlib.Path('/fast/git/v/research/2026-10-12-vibevm-first-agent-start/M03-A1')
binary=pathlib.Path('/fast/git/v/.build/c4-first-agent-target/debug/vibe')
def sha(b):return hashlib.sha256(b).hexdigest()
def inventory(root):
 return {str(p.relative_to(root)):({'kind':'directory'} if p.is_dir() else {'kind':'file','bytes':p.stat().st_size,'sha256':sha(p.read_bytes())}) for p in sorted(root.rglob('*'))}
script=r'''#!/bin/sh
python3 - <<'FIXTURE'
import os,json,pathlib,subprocess
ctx=json.load(open(os.environ['VIBE_CONTEXT']));r=pathlib.Path(ctx['project']['root']);op=ctx['execution']['id']
with (r/'calls').open('a') as f:f.write(op+'\n')
if op=='produce':
 (r/'src').mkdir(exist_ok=True);p=r/'src/product.rs';p.write_text('pub fn answer()->u32{42}\n#[test] fn value(){assert_eq!(answer(),42);}\n');ident='source'
elif op=='compile':
 (r/'target').mkdir(exist_ok=True);p=r/'target/check';subprocess.run(['/home/olegchir/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/bin/rustc','--edition=2024','--test',str(r/'src/product.rs'),'-o',str(p)],check=True);ident='compiled'
elif op=='observe':
 result=subprocess.run([str(r/'target/check')],check=True,capture_output=True);(r/'evidence').mkdir(exist_ok=True);p=r/'evidence/stdout';p.write_bytes(result.stdout);ident='evidence'
else:raise RuntimeError(op)
json.dump({'envelope':1,'status':'ok','tasks':[],'artifacts':[{'id':ident,'kind':'file','path':str(p)}]},open(os.environ['VIBE_REPLY'],'w'))
FIXTURE
'''
def authored(root):
 text="[project]\ngroup='org.demo'\nname='consumer'\nversion='1.0.0'\n"
 for ident,phase,effect,inp in [('produce','create','product-source','intent.txt'),('compile','build','build-artifact','src/product.rs'),('observe','verify','evidence-observer','src/product.rs')]:
  text+=f"\n[[extension]]\nid='{ident}'\npoint='phase:{phase}'\neffects=['{effect}']\ninputs=['{inp}']\nhandler={{kind='script',base='scripts/operation'}}\n"
 (root/'vibe.toml').write_text(text);(root/'intent.txt').write_text('stable explicit intent\n');(root/'scripts').mkdir();(root/'scripts/operation.sh').write_text(script)
def env_for(root):
 dirs={key:root/key for key in ['home','config','cache','state','data','cargo']}
 for p in dirs.values():p.mkdir()
 env={'PATH':os.environ['PATH'],'LANG':'C.UTF-8','HOME':str(dirs['home']),'XDG_CONFIG_HOME':str(dirs['config']),'XDG_CACHE_HOME':str(dirs['cache']),'XDG_STATE_HOME':str(dirs['state']),'XDG_DATA_HOME':str(dirs['data']),'CARGO_HOME':str(dirs['cargo']),'RUSTUP_HOME':'/home/olegchir/.rustup','RUSTUP_TOOLCHAIN':'nightly','RUSTUP_AUTO_INSTALL':'0','CARGO_NET_OFFLINE':'true'}
 return env,dirs
results=[]
for mode in ['default','relative-member']:
 fixture=pathlib.Path(tempfile.mkdtemp(prefix='m03-a1-cli-'));selected=fixture/'project';selected.mkdir();env,dirs=env_for(fixture)
 if mode=='relative-member':
  (selected/'vibe.toml').write_text("[project]\nname='container'\nversion='1.0.0'\n[workspace]\nmembers=['member']\n");selected=selected/'member';selected.mkdir()
 authored(selected);cwd=selected if mode=='default' else selected.parent;path=[] if mode=='default' else ['--path','member']
 before_control={k:inventory(p) for k,p in dirs.items()};initial=inventory(selected)
 for phase in ['create','build','verify']:
  phase_control={k:inventory(p) for k,p in dirs.items()}
  before=inventory(selected);source_before=sha((selected/'src/product.rs').read_bytes()) if (selected/'src/product.rs').exists() else None
  argv=[str(binary),'--json','--offline','--agent-mode','agent','--no-progress',phase,*path,'--assume-yes']+(['--force'] if phase!='create' else [])
  run=subprocess.run(argv,cwd=cwd,env=env,capture_output=True);stem=f'CLI-{mode}-{phase}'
  (home/(stem+'.stdout.json')).write_bytes(run.stdout);(home/(stem+'.stderr.log')).write_bytes(run.stderr)
  record={'mode':mode,'phase':phase,'root':str(selected),'cwd':str(cwd),'argv':argv,'exit':run.returncode,'binary_sha256':sha(binary.read_bytes()),'stdout_sha256':sha(run.stdout),'stderr_sha256':sha(run.stderr),'before':before,'after':inventory(selected),'source_before':source_before,'source_after':sha((selected/'src/product.rs').read_bytes()) if (selected/'src/product.rs').exists() else None,'control_before':phase_control,'initial_control':before_control,'control_after':{k:inventory(p) for k,p in dirs.items()}}
  (home/(stem+'.evidence.json')).write_text(json.dumps(record,indent=2)+'\n');results.append(record)
  if run.returncode!=0:
   (home/'CLI-SMOKE-RESULTS.json').write_text(json.dumps(results,indent=2)+'\n');raise RuntimeError(f'{stem} exit{run.returncode}: '+run.stderr.decode(errors='replace')[-1200:])
  decoder=json.JSONDecoder(); raw=run.stdout.decode(); documents=[]; pos=0
  while pos<len(raw):
   while pos<len(raw) and raw[pos].isspace():pos+=1
   if pos==len(raw):break
   value,pos=decoder.raw_decode(raw,pos);documents.append(value)
  assert [v['command'] for v in documents]==['lifecycle:plan','lifecycle']
  value=documents[-1];assert value['ok'] is True
  record['decoded_documents']=documents
  (home/(stem+'.evidence.json')).write_text(json.dumps(record,indent=2)+'\n')
  print(json.dumps({'stem':stem,'exit':run.returncode,'keys':list(value),'calls':(selected/'calls').read_text(),'source_unchanged':record['source_before']==record['source_after'] if phase!='create' else None}),flush=True)
  if phase!='create':
   assert record['source_before']==record['source_after']
   assert record['control_before']==record['control_after']
 assert (selected/'calls').read_text()=='produce\ncompile\nobserve\n'
 assert '1 passed' in (selected/'evidence/stdout').read_text()
 assert results[-1]['control_before']==results[-1]['control_after']
 if mode=='relative-member':assert not (selected.parent/'src').exists() and not (selected.parent/'target').exists()
(home/'CLI-SMOKE-RESULTS.json').write_text(json.dumps(results,indent=2)+'\n')
