#!/usr/bin/env python3
"""Finite operator wrapper: execute only the packet's selected verification argv."""
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
commands = [shlex.split(x) for x in group.get('commands', [])]
if group_id == 'V5':
    decision = json.loads((ROOT / 'V5-SELECTION.json').read_text())
    if decision['mode'] == 'reuse':
        assert decision['rationale'] and decision['source_hashes'] and decision['evidence_hashes']
        for name, digest in decision['source_hashes'].items():
            path = (REPO / name).resolve()
            assert path.is_relative_to(REPO)
            assert hashlib.sha256(path.read_bytes()).hexdigest() == digest
        for name, digest in decision['evidence_hashes'].items():
            assert hashlib.sha256(Path(name).read_bytes()).hexdigest() == digest
        result = {'group': group_id, 'executed': False, 'disposition': 'matching evidence reused; not a new pass', 'decision': decision}
        (ROOT / 'V5-result.json').write_text(json.dumps(result, indent=2) + '\n')
        print(json.dumps(result))
        raise SystemExit(0)
    assert decision['mode'] == 'run' and decision['rationale']
if group_id == 'V6':
    paths = json.loads((ROOT / 'V6-PATHS.json').read_text())
    assert paths and all(isinstance(p, str) and p.endswith('.rs') for p in paths)
    allowed = {
        'crates/vibe-agent-projection/src/pkgskill/projection.rs',
        'crates/vibe-agent-projection/src/pkgskill/projection/m_08_b_2_cases.rs',
        'crates/vibe-agent-projection/src/pkgskill/projection/capture.rs',
        'crates/vibe-agent-projection/src/pkgskill/projection/binding.rs',
        'crates/vibe-orchestrator/src/machine_operation/adapter_plan/tests.rs',
        'crates/vibe-orchestrator/src/machine_operation/adapter_plan/m_08_b_2_fixture.rs',
    }
    assert set(paths) <= allowed
    commands = [['rustfmt', '--edition', '2024', '--config', 'skip_children=true', '--check', *paths]]
results = []
for i, argv in enumerate(commands, 1):
    log = ROOT / f'{group_id}-{i}.log'
    started = time.monotonic()
    with log.open('wb') as stream:
        done = subprocess.run(argv, cwd=REPO, stdout=stream, stderr=subprocess.STDOUT)
    output = log.read_text(errors='replace')
    counts = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored', output)
    minimum = group.get('expected_min_nonzero', group.get('expected_nonzero_each', 1))
    count_ok = bool(counts) and sum(int(x[0]) for x in counts) >= minimum and all(x[1] == '0' and x[2] == '0' for x in counts)
    passed = done.returncode == 0 and (count_ok if argv[:2] == ['cargo', 'test'] else True)
    results.append({'argv': argv, 'exit': done.returncode, 'counts': counts, 'meets_expected_count': count_ok if argv[:2] == ['cargo', 'test'] else None, 'log': str(log), 'seconds': time.monotonic()-started, 'passed': passed})
result = {'group': group_id, 'executed': True, 'commands': results, 'passed': bool(results) and all(x['passed'] for x in results)}
(ROOT / f'{group_id}-result.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps(result))
raise SystemExit(0 if result['passed'] else 1)
