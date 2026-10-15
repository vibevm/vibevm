#!/usr/bin/env python3
"""Finite operator verification wrapper; decisions and new case bindings are explicit."""
from pathlib import Path
import difflib
import hashlib
import json
import re
import subprocess
import sys
import time
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parent
REPO = Path('/fast/git/v/vibevm-next')
group_id = sys.argv[1]
group = next(x for x in json.loads((ROOT / 'SELECTED-CHECKS.json').read_text()) if x['id'] == group_id)
baseline = json.loads((ROOT / 'source-baseline.json').read_text())['sha256']
def save(value):
    (ROOT / f'{group_id}-result.json').write_text(json.dumps(value, indent=2) + '\n')
    print(json.dumps(value))
if group_id in ['V2', 'V3', 'V4', 'V5']:
    choice = json.loads((ROOT / 'CHECK-SELECTION.json').read_text())[group_id]
    assert choice['rationale']
    if choice['mode'] == 'not_applicable':
        assert group_id == 'V2' and not (REPO / 'crates/vibe-agent-projection/src/client_projection.rs').exists()
        save({'group': group_id, 'executed': False, 'disposition': 'module not introduced; not an executed pass', 'choice': choice})
        raise SystemExit(0)
    if choice['mode'] == 'unchanged_source':
        assert group_id in ['V3', 'V4', 'V5']
        assert set(group['reuse_required']) <= set(choice['source_hashes'])
        for name, digest in choice['source_hashes'].items():
            assert baseline[name] == digest
            assert hashlib.sha256((REPO / name).read_bytes()).hexdigest() == digest
        save({'group': group_id, 'executed': False, 'disposition': 'unchanged prior candidate source/evidence reused; candidate remains unaccepted, no new test pass claimed', 'choice': choice})
        raise SystemExit(0)
    assert choice['mode'] == 'run'
if group_id == 'V7':
    path = REPO / 'vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml'
    before = (ROOT / 'PROP054-baseline.xml').read_bytes()
    after = path.read_bytes()
    ET.fromstring(after)
    changes = difflib.SequenceMatcher(None, before.splitlines(keepends=True), after.splitlines(keepends=True), autojunk=False).get_opcodes()
    preserved = after == before
    old = b''.join(line for line in after.splitlines(keepends=True) if b'<GENERIC-HOST-EXACT-GRANTS ' not in line)
    old_digest = hashlib.sha256(old).hexdigest()
    assert old_digest == 'cd594468e7b3cad5a5edf92855e01fcabe613426182f6fe7a60ecc03f47db2e5'
    (ROOT / 'spec-old-clause-reconstruction.json').write_text(json.dumps({'current_candidate_bytes_exact': preserved, 'old_sha256': old_digest, 'removed_lines': len(after.splitlines())-len(old.splitlines())}, indent=2)+'\n')
    save({'group': group_id, 'executed': True, 'xml_parsed': True, 'old_bytes_preserved_as_unchanged_lines': preserved, 'old_sha256': hashlib.sha256(before).hexdigest(), 'new_sha256': hashlib.sha256(after).hexdigest(), 'passed': preserved})
    raise SystemExit(0 if preserved else 1)
commands = group['commands']
if group_id == 'V6':
    paths = json.loads((ROOT / 'V6-PATHS.json').read_text())
    allowed = json.loads((ROOT / 'CANONICAL-TASK.json').read_text())['tasks'][1]['write_paths']
    for filename, question in [('approved-error-ROOT-ANSWER.json', 'm08c2-generic-provider-error-1'), ('approved-v4-ROOT-ANSWER.json', 'm08c2-v4-native-path-fixture-2')]:
        amendment = json.loads((ROOT / filename).read_text())
        assert amendment['question_id'] == question and amendment['approved'] is True
        allowed = allowed + amendment['additional_write_paths']
    assert paths and all(p in allowed and p.endswith('.rs') and '/generated/' not in p for p in paths)
    commands = [['rustfmt', '--edition', '2024', '--config', 'skip_children=true', '--check', *paths], ['git', 'diff', '--check']]
new_cases = None
if group_id in ['V1', 'V2']:
    new_cases = json.loads((ROOT / 'CASE-BINDINGS.json').read_text())[group_id]
    assert new_cases['names'] and new_cases['expected'] == len(new_cases['names'])
results = []
for index, argv in enumerate(commands, 1):
    log = ROOT / f'{group_id}-{index}.log'
    start = time.monotonic()
    subjects = {name: hashlib.sha256((REPO / name).read_bytes()).hexdigest() for name in baseline}
    (ROOT / f'{group_id}-{index}-subjects-before.json').write_text(json.dumps(subjects, indent=2) + '\n')
    with log.open('wb') as stream:
        process = subprocess.run(argv, cwd=REPO, stdout=stream, stderr=subprocess.STDOUT)
    after_subjects = {name: hashlib.sha256((REPO / name).read_bytes()).hexdigest() for name in baseline}
    (ROOT / f'{group_id}-{index}-subjects-after.json').write_text(json.dumps(after_subjects, indent=2) + '\n')
    output = log.read_text(errors='replace')
    counts = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored', output)
    count_ok = bool(counts) and sum(int(x[0]) for x in counts) > 0 and all(x[1] == '0' and x[2] == '0' for x in counts)
    if '--exact' in argv:
        count_ok = count_ok and sum(int(x[0]) for x in counts) == 1
    if new_cases:
        count_ok = count_ok and sum(int(x[0]) for x in counts) == new_cases['expected'] and all(('test ' + name + ' ... ok') in output for name in new_cases['names'])
    passed = process.returncode == 0 and (count_ok if argv[:2] == ['cargo', 'test'] else True)
    results.append({'argv': argv, 'exit': process.returncode, 'counts': counts, 'count_matches': count_ok if argv[:2] == ['cargo', 'test'] else None, 'log': str(log), 'seconds': time.monotonic()-start, 'passed': passed, 'consumed_sources_stable': subjects == after_subjects})
result = {'group': group_id, 'executed': True, 'case_binding': new_cases, 'commands': results, 'passed': bool(results) and all(x['passed'] for x in results)}
save(result)
raise SystemExit(0 if result['passed'] else 1)
