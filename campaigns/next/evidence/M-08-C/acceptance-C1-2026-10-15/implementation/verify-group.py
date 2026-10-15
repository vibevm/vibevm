#!/usr/bin/env python3
"""Finite operator wrapper for the seven packet-bound C1 verification groups."""
from pathlib import Path
import hashlib
import json
import re
import shlex
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parent
REPO = Path('/fast/git/v/vibevm-next')
group_id = sys.argv[1]
group = next(x for x in json.loads((ROOT / 'SELECTED-CHECKS.json').read_text()) if x['id'] == group_id)
prior = [ROOT / f"{group_id}-result.json"] + [ROOT / f"{group_id}-{i}.log" for i in range(1,5)]
if any(p.exists() for p in prior):
    attempt=1
    while any((ROOT / f"attempt{attempt}-{p.name}").exists() for p in prior): attempt+=1
    for p in prior:
        if p.exists(): p.rename(ROOT / f"attempt{attempt}-{p.name}")
commands = [shlex.split(x) for x in group.get('commands', [])]
if group_id == 'V7':
    decision = json.loads((ROOT / 'V7-SELECTION.json').read_text())
    assert decision['rationale']
    if decision['mode'] == 'reuse':
        binary = Path(decision['binary'])
        assert hashlib.sha256(binary.read_bytes()).hexdigest() == decision['binary_sha256']
        assert decision['source_hashes'] and decision['evidence_hashes']
        for name, digest in decision['source_hashes'].items():
            path = (REPO / name).resolve()
            assert path.is_relative_to(REPO)
            assert hashlib.sha256(path.read_bytes()).hexdigest() == digest
        for name, digest in decision['evidence_hashes'].items():
            assert hashlib.sha256(Path(name).read_bytes()).hexdigest() == digest
        result = {'group': group_id, 'executed': False, 'disposition': 'same-subject executable reused; not a new build', 'decision': decision}
        (ROOT / 'V7-result.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result))
        raise SystemExit(0)
    assert decision['mode'] == 'run'
if group_id == 'V6':
    paths = json.loads((ROOT / 'V6-PATHS.json').read_text())
    task = json.loads((ROOT / 'CANONICAL-TASK.json').read_text())['tasks'][0]
    assert paths and all(isinstance(p, str) and p.endswith('.rs') for p in paths)
    for name in paths:
        path = (REPO / name).resolve()
        assert path.is_relative_to(REPO) and '/generated/' not in name
        assert any(name == allowed or (allowed.endswith('/') and name.startswith(allowed)) for allowed in task['write_paths'])
    commands = [['rustfmt', '--edition', '2024', '--config', 'skip_children=true', '--check', *paths], ['git', 'diff', '--check']]
results = []
for i, argv in enumerate(commands, 1):
    log = ROOT / f'{group_id}-{i}.log'
    started = time.monotonic()
    with log.open('wb') as stream:
        done = subprocess.run(argv, cwd=REPO, stdout=stream, stderr=subprocess.STDOUT)
    output = log.read_text(errors='replace')
    counts = re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored', output)
    count_ok = bool(counts) and sum(int(x[0]) for x in counts) > 0 and all(x[1] == '0' and x[2] == '0' for x in counts)
    if group.get('expected_counts'):
        count_ok = count_ok and (int(counts[-1][0]) if '--exact' in argv and counts else sum(int(x[0]) for x in counts)) == group['expected_counts'][i-1]
    if '--exact' in argv:
        count_ok = count_ok and int(counts[-1][0]) == 1
    passed = done.returncode == 0 and (count_ok if argv[:2] == ['cargo', 'test'] else True)
    results.append({'argv': argv, 'exit': done.returncode, 'counts': counts, 'nonzero_nonignored': count_ok if argv[:2] == ['cargo', 'test'] else None, 'log': str(log), 'seconds': time.monotonic()-started, 'passed': passed})
preservation = None
if group_id in ['V4', 'V5']:
    expected = json.loads((ROOT / 'OLD-E1-PRESERVATION.json').read_text())['files']
    exceptions = json.loads((ROOT / 'ROOT-ANSWER.json').read_text())['allowed_comment_only_paths']
    drift = []
    headers = []
    def strip_header(data):
        return re.sub(rb'\A(?://[^\n]*\n)*', b'', data)
    for name, digest in expected.items():
        data = (REPO / name).read_bytes()
        actual = hashlib.sha256(data).hexdigest()
        if name in exceptions:
            old = subprocess.check_output(['git','show','HEAD:'+name],cwd=REPO)
            equal = strip_header(old) == strip_header(data)
            headers.append({'path':name,'before_sha256':digest,'after_sha256':actual,'comment_stripped_equal':equal})
            if not equal: drift.append(name)
        elif actual != digest: drift.append(name)
    preservation = {'exact_files':11,'header_only_files':headers,'drift':drift,'passed':not drift}

result = {'group': group_id, 'executed': True, 'commands': results, 'old_e1_preservation': preservation, 'passed': bool(results) and all(x['passed'] for x in results) and (preservation is None or preservation['passed'])}
(ROOT / f'{group_id}-result.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result))
raise SystemExit(0 if result['passed'] else 1)
