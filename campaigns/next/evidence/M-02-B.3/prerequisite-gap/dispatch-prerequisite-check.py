from pathlib import Path
import json,sys,tomllib,hashlib
root=Path('/fast/git/v/vibevm-next')
home=Path(__file__).parent
manifest=json.loads((root/'campaigns/next/manifest.json').read_text())
accepted=set(json.loads((home/'CAMPAIGN-COUNTS.json').read_text())['accepted_ids'])
seed_path=Path('/home/olegchir/.vibe/steward/contexts/d1d2418b-4581-400b-8288-169707f1b954/plan.toml')
seed=tomllib.loads(seed_path.read_text())
gates={}
def scan(v):
    if isinstance(v,dict):
        if v.get('id') in ['NEXT-P0-GATE','NEXT-PREVIEW-BASE.3']: gates[v['id']]=v.get('state')
        for item in v.values(): scan(item)
    elif isinstance(v,list):
        for item in v: scan(item)
scan(seed)
def tasks(parent):
    p=root/'campaigns/next/tasks'/f'{parent}.json'
    return [v['id'] for v in json.loads(p.read_text())['tasks']] if p.is_file() else []
missing=[];seen=set();checked={}
def parent_done(parent):
    if parent in seen:return
    seen.add(parent)
    ids=tasks(parent)
    if ids:
        absent=[v for v in ids if v not in accepted];checked[parent]=absent;missing.extend(absent)
    elif parent not in accepted and gates.get(parent)!='accepted': missing.append(parent)
    for dep in manifest['baseline_graph'].get(parent,[]):parent_done(dep)
task=sys.argv[1];parent=task.split('.')[0];siblings=tasks(parent);assert task in siblings,task
missing.extend(v for v in siblings[:siblings.index(task)] if v not in accepted)
for dep in manifest['baseline_graph'].get(parent,[]):parent_done(dep)
for dep in manifest.get('extra_dependencies',{}).get(task,[]):
    if dep not in accepted and gates.get(dep)!='accepted':parent_done(dep)
result={'task':task,'eligible':not missing,'missing':sorted(set(missing)),'checked_parents':checked,'extra_dependencies':manifest.get('extra_dependencies',{}).get(task,[]),'accepted_gates':gates,'seed_sha256':hashlib.sha256(seed_path.read_bytes()).hexdigest()}
print(json.dumps(result,indent=2));sys.exit(0 if not missing else 1)
