#!/usr/bin/env python3
"""Finite C3 verification command wrapper, not product logic or acceptance."""
from pathlib import Path
import os
os.umask(0o077)
import difflib
import hashlib
import json
import subprocess
import re
import sys
import time
import xml.etree.ElementTree as ET

ROOT=Path(__file__).resolve().parent
REPO=Path('/fast/git/v/vibevm-next')
group_id=sys.argv[1]
group=next(x for x in json.loads((ROOT/'SELECTED-CHECKS.json').read_text()) if x['id']==group_id)
base=json.loads((ROOT/'source-baseline.json').read_text())
hash_file=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def save(value):
    (ROOT/f'{group_id}-result.json').write_text(json.dumps(value,indent=2)+'\n')
    print(json.dumps(value))
if group_id=='V7':
    spec=REPO/'vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml'
    old=(ROOT/'PROP054-baseline.xml').read_bytes();new=spec.read_bytes();ET.fromstring(new)
    spec_ok=all(tag in ['equal','insert'] for tag,*_ in difflib.SequenceMatcher(None,old.splitlines(keepends=True),new.splitlines(keepends=True),autojunk=False).get_opcodes())
    schemas={p:d for p,d in base['sha256'].items() if p.startswith('schemas/') and p != 'schemas/deploy_generic_state.jtd.json'}
    schema_bad=[p for p,d in schemas.items() if hash_file(REPO/p)!=d]
    gen=json.loads((ROOT/'GENERATED-BASELINE.json').read_text());exceptions={'crates/vibe-wire/src/generated/mod.rs','crates/vibe-wire/src/generated/format_id/mod.rs','crates/vibe-wire/src/generated/format_id/properties.rs'}
    records=[];bad=[]
    def body(data):
        lines=data.splitlines(keepends=True);i=0
        while i<len(lines) and (not lines[i].strip() or lines[i].lstrip().startswith(b'//')):i+=1
        return b''.join(lines[i:])
    for p,d in gen['files'].items():
        current=(REPO/p).read_bytes()
        if hashlib.sha256(current).hexdigest()==d:continue
        if p in exceptions:records.append({'path':p,'kind':'declared new-family registration'});continue
        prior=subprocess.run(['git','show',gen['head']+':'+p],cwd=REPO,capture_output=True,check=True).stdout
        assert hashlib.sha256(prior).hexdigest()==d
        equal=p.endswith('.rs') and body(prior)==body(current)
        diff=list(difflib.unified_diff(prior.decode().splitlines(),current.decode().splitlines(),n=0))
        edited=[line[1:] for line in diff if line.startswith(('+','-')) and not line.startswith(('+++','---'))]
        header_only=all(not line.strip() or (line.lstrip().startswith('//') and ('schema' in line.lower() or '.jtd.json' in line)) for line in edited)
        okay=equal and header_only
        records.append({'path':p,'old_body_equal':equal,'source_list_header_only':header_only,'passed':okay})
        if not okay:bad.append(p)
    result={'group':group_id,'executed':True,'xml_parsed':True,'prior_spec_lines_preserved':spec_ok,'old_schema_drift':schema_bad,'generated_changes':records,'unqualified_generated_drift':bad,'passed':spec_ok and not schema_bad and not bad}
    save(result);raise SystemExit(0 if result['passed'] else 1)
commands=list(group['commands']) if isinstance(group['commands'],list) else []
selection=None
if group_id=='V4':
    selection=json.loads((ROOT/'V4-SELECTION.json').read_text());allowed={tuple(x):x for x in group['existing_conditional_commands']}
    run={tuple(x) for x in selection['run']};reused={tuple(x['argv']) for x in selection['reused']}
    assert run.isdisjoint(reused) and run|reused==set(allowed)
    for row in selection['reused']:
        assert row['rationale'] and row['source_hashes']
        for p,d in row['source_hashes'].items():assert base['sha256'][p]==d and hash_file(REPO/p)==d
    commands += [allowed[x] for x in sorted(run)]
if group_id=='V6':
    paths=json.loads((ROOT/'V6-PATHS.json').read_text());allowed=json.loads((ROOT/'CANONICAL-TASK.json').read_text())['tasks'][2]['write_paths']; allowed += [p for p in paths if p.startswith(('crates/vibe-lifecycle/src/mechanism/deploy/','crates/vibe-agent-projection/src/pkgskill/')) or p=='crates/vibe-agent-projection/src/pkgskill.rs']
    assert paths and all(p in allowed and p.endswith('.rs') and '/generated/' not in p for p in paths)
    commands=[['rustfmt','--edition','2024','--config','skip_children=true','--check',*paths],['git','diff','--check']]
binding=None
if group.get('new_family'):
    binding=json.loads((ROOT/'CASE-BINDINGS.json').read_text())[group_id]
    assert binding['names'] and binding['expected']==len(binding['names'])
subject_paths=json.loads((ROOT/'SOURCE-SUBJECTS.json').read_text()) if (ROOT/'SOURCE-SUBJECTS.json').exists() else list(base['sha256'])
results=[]
for index,argv in enumerate(commands,1):
    before={p:hash_file(REPO/p) for p in subject_paths};(ROOT/f'{group_id}-{index}-subjects-before.json').write_text(json.dumps(before,indent=2)+'\n')
    log=ROOT/f'{group_id}-{index}.log';start=time.monotonic()
    with log.open('wb') as stream:done=subprocess.run(argv,cwd=REPO,stdout=stream,stderr=subprocess.STDOUT)
    after={p:hash_file(REPO/p) for p in subject_paths};(ROOT/f'{group_id}-{index}-subjects-after.json').write_text(json.dumps(after,indent=2)+'\n')
    output=log.read_text(errors='replace');counts=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored',output)
    count_ok=bool(counts) and sum(int(x[0]) for x in counts)>0 and all(x[1]=='0' and x[2]=='0' for x in counts)
    if '--exact' in argv:count_ok=count_ok and sum(int(x[0]) for x in counts)==1
    elif binding:count_ok=count_ok and sum(int(x[0]) for x in counts)==binding['expected'] and all('test '+n+' ... ok' in output for n in binding['names'])
    elif group_id=='V4' and index==1:count_ok=count_ok and sum(int(x[0]) for x in counts)==13
    passed=done.returncode==0 and (count_ok if argv[:2]==['cargo','test'] else True)
    results.append({'argv':argv,'exit':done.returncode,'counts':counts,'log':str(log),'seconds':time.monotonic()-start,'passed':passed,'source_drift':[p for p in before if before[p]!=after[p]]})
result={'group':group_id,'executed':True,'case_binding':binding,'conditional_selection':selection,'commands':results,'passed':bool(results) and all(x['passed'] for x in results)}
save(result);raise SystemExit(0 if result['passed'] else 1)
