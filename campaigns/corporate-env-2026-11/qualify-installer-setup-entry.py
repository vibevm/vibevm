#!/usr/bin/env python3
"""Public no-publication probes for the corporate installer's optional setup seam."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

p = argparse.ArgumentParser()
p.add_argument('--installer', required=True)
p.add_argument('--cli', required=True)
p.add_argument('--output', required=True)
a = p.parse_args()
root = Path(a.output)
root.mkdir(mode=0o700)
installer, cli = Path(a.installer).absolute(), Path(a.cli).absolute()
steps = []
for label, binary, args in [
    ('installer-help', installer, ['--help']),
    ('rerun-configure-ai-absent', installer, ['--configure-ai', '--unattended']),
    ('initial-empty-kit', installer, ['--kit', '/world/kit', '--unattended', '--no-path']),
    ('standalone-setup-help', cli, ['ai', 'setup', '--help']),
    ('standalone-setup-no-selection', cli, ['ai', 'setup', '--json', '--unattended']),
]:
    world = root / label
    (world / 'kit').mkdir(parents=True)
    (world / 'settings').mkdir()
    cmd = ['/usr/bin/bwrap', '--unshare-all', '--die-with-parent', '--new-session', '--clearenv']
    for source in ['/usr', '/bin', '/lib', '/lib64']:
        cmd += ['--ro-bind', source, source]
    cmd += ['--dir', '/etc', '--ro-bind', '/etc/ld.so.cache', '/etc/ld.so.cache',
            '--proc', '/proc', '--dev', '/dev', '--tmpfs', '/tmp', '--dir', '/home/probe',
            '--bind', str(world), '/world', '--ro-bind', str(binary), '/tools/subject',
            '--setenv', 'HOME', '/home/probe', '--setenv', 'PATH', '/usr/bin:/bin',
            '--setenv', 'VIBE_SETTINGS', '/world/settings', '--setenv', 'VIBE_REGISTRY_CACHE', '/world/cache',
            '--setenv', 'VIBE_NO_DEFAULT_REGISTRY', '1', '--chdir', '/tmp', '/tools/subject'] + args
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=25)
    row = dict(label=label, args=args, exit=result.returncode, stdout=result.stdout, stderr=result.stderr)
    steps.append(row)
    (root / 'steps.json').write_text(json.dumps(steps, indent=2) + '\n')
    if label.endswith('help'):
        assert result.returncode == 0, row
        if label == 'installer-help':
            assert '--configure-ai' not in result.stdout
        else:
            assert '--apply-preview' in result.stdout and '--agent' in result.stdout
    elif label == 'rerun-configure-ai-absent':
        assert result.returncode == 2 and 'unexpected argument' in result.stderr and '--configure-ai' in result.stderr, row
    elif label == 'initial-empty-kit':
        assert result.returncode != 0 and 'site profile' in result.stderr, row
    else:
        assert result.returncode != 0 and 'explicit --agent selection' in result.stderr, row
    for name in ['applications/index.json', 'opt/bin', 'opt/apps', 'secrets/index.toml', 'secrets/ai/index.toml']:
        assert not (world / 'settings' / name).exists(), name
    row['settings_entries'] = sorted(str(path.relative_to(world / 'settings')) for path in (world / 'settings').rglob('*'))
(root / 'steps.json').write_text(json.dumps(steps, indent=2) + '\n')
summary = {'protocol': 'corporate-setup-entry-qualification/1', 'commands': len(steps),
           'installer_sha256': hashlib.sha256(installer.read_bytes()).hexdigest(),
           'cli_sha256': hashlib.sha256(cli.read_bytes()).hexdigest(),
           'scope': 'parser/empty-kit and standalone setup admission, NOT actual initial/rerun installation',
           'automatic_setup_adapter': 'missing in current parser/orchestration; source inspection required',
           'credential_or_provider_operation': False}
(root / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
print(json.dumps(summary, indent=2))
