#!/usr/bin/env python3
"""Offline shipping authority proof: vibe-bin consumer-bin-dir scratch-parent typescript-dir.
Uses an existing TypeScript installation; never installs packages or calls a provider.
"""
import pathlib, tempfile, subprocess, os, json, hashlib, sys
if len(sys.argv) != 5: raise SystemExit(__doc__)
P=pathlib.Path
base=P(tempfile.mkdtemp(prefix='up-authority-shipping-',dir=sys.argv[3]))
project=base/'project'; settings=base/'settings'; registry=base/'registry'; home=base/'home'
for p in (project,settings,registry,home): p.mkdir()
(settings/'registry.toml').write_text('')
env={'PATH':os.environ['PATH'],'HOME':str(home),'VIBE_SETTINGS':str(settings),'VIBE_REGISTRY_CACHE':str(base/'cache'),'VIBE_NO_DEFAULT_REGISTRY':'1','TMPDIR':str(base),'LANG':'C.UTF-8'}
vibe=P(sys.argv[1]).resolve(strict=True); bins=P(sys.argv[2]).resolve(strict=True); rows=[]
def run(args,expect=True):
 p=subprocess.run(list(map(str,args)),cwd=project,env=env,text=True,capture_output=True,timeout=120)
 rows.append({'argv':list(map(str,args)),'exit':p.returncode,'stdout':p.stdout,'stderr':p.stderr})
 (base/'RESULTS.json').write_text(json.dumps({'host_binary_sha256':hashlib.sha256(vibe.read_bytes()).hexdigest(),'scenario_sha256':hashlib.sha256(P(__file__).read_bytes()).hexdigest(),'steps':rows},indent=2))
 if expect and p.returncode: raise AssertionError(rows[-1])
 return p
run([vibe,'init','--path',project])
pkg=registry/'org.authority/example/v1.0.0'; (pkg/'custom-specs').mkdir(parents=True)
(pkg/'vibe.toml').write_text("[package]\ngroup='org.authority'\nname='example'\nversion='1.0.0'\nkind='flow'\n")
(pkg/'specmap.toml').write_text("namespace='dependency-alias'\nspec_roots=['custom-specs']\nscan_roots=[]\n")
(pkg/'custom-specs/RULE.md').write_text('# Rule {#rule}\nDeclared dependency rule.\n')
# Use an already installed compiler, isolated fixture resolution only; no install.
(project/'node_modules').mkdir(); (project/'node_modules/typescript').symlink_to(P(sys.argv[4]).resolve(strict=True),target_is_directory=True)
run([vibe,'install','org.authority/example','--path',project,'--registry',registry,'--offline','--json'])
a=json.loads((project/'.vibe/specmap-authority.json').read_text()); dep=P(a['dependencies'][0]['spec_roots'][0])
(project/'specmap.toml').write_text(f"namespace='own-alias'\nscan_roots=[]\nspec_roots=[]\n[[external_specs]]\nnamespace='declared-alias'\nroot='{dep}'\n")
run([vibe,'install','--path',project,'--registry',registry,'--offline','--json'])
def consumer(name,good):
 exe=bins/name
 if name.endswith('-mcp'):
  req=json.dumps({'jsonrpc':'2.0','id':1,'method':'tools/call','params':{'name':'specmap_write','arguments':{}}})+'\n'
  p=subprocess.run([str(exe),'--path',str(project)],input=req,cwd=project,env=env,text=True,capture_output=True,timeout=60)
  response=next(json.loads(line) for line in p.stdout.splitlines() if json.loads(line).get('id')==1)
  ok=not response.get('result',{}).get('isError',False) and 'error' not in response
 else:
  p=run([exe,'--path',project],expect=False); ok=p.returncode==0
 rows.append({'consumer':name,'expected_success':good,'success':ok,'stdout':p.stdout,'stderr':p.stderr,'binary_sha256':hashlib.sha256(exe.read_bytes()).hexdigest()})
 (base/'RESULTS.json').write_text(json.dumps({'host_binary_sha256':hashlib.sha256(vibe.read_bytes()).hexdigest(),'scenario_sha256':hashlib.sha256(P(__file__).read_bytes()).hexdigest(),'steps':rows},indent=2))
 assert ok==good,rows[-1]
for name in ['rust-ai-native-specmap','typescript-ai-native-specmap','rust-ai-native-mcp','typescript-ai-native-mcp']: consumer(name,True)
# Direct normalized annotations must fail, not merely warn or count coverage.
(project/'code/src').mkdir(parents=True)
config=project/'specmap.toml'; config.write_text(config.read_text().replace('scan_roots=[]',"scan_roots=['code']"))
run([vibe,'install','--path',project,'--registry',registry,'--offline','--json'])
for namespace,good in [('declared-alias',True),('org.private/rules',False)]:
 (project/'code/src/lib.rs').write_text(f'#[spec(implements = "spec://{namespace}/RULE#rule")]\npub fn demo() {{}}\n')
 (project/'code/demo.ts').write_text(f'/** @implements spec://{namespace}/RULE#rule */\nexport function demo() {{}}\n')
 for name in ['rust-ai-native-specmap','typescript-ai-native-specmap','rust-ai-native-mcp','typescript-ai-native-mcp']: consumer(name,good)
(project/'code/src/lib.rs').unlink(); (project/'code/demo.ts').unlink()
# Configured undeclared root must refuse before publishing a replacement index.
unknown=base/'undeclared'; unknown.mkdir(); (unknown/'RULE.md').write_text('# Unknown {#rule}\n')
config=project/'specmap.toml'; config.write_text(config.read_text().replace(str(dep),str(unknown)))
run([vibe,'install','--path',project,'--registry',registry,'--offline','--json'])
for name in ['rust-ai-native-specmap','typescript-ai-native-specmap','rust-ai-native-mcp','typescript-ai-native-mcp']: consumer(name,False)
print(base/'RESULTS.json')
