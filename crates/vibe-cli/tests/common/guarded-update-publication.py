"""Mandatory finite public rollback gate, outside and around one sandbox/run.
Run with python3 -I SCRIPT --cli EXACT_BINARY --root NEW_DISPOSABLE_DIRECTORY.
No host home/auth/config or root filesystem mount is admitted.
"""
import argparse
import hashlib
import json
import pathlib
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument('--cli', required=True)
parser.add_argument('--root', required=True)
parser.add_argument('--fault', choices=['publication','staging','boot'], default='publication')
args = parser.parse_args()
root = pathlib.Path(args.root).absolute()
root.mkdir(exist_ok=False)
project = root / 'project'
registry = root / 'registry'
settings = root / 'settings'
for path in [project, registry, settings, root / 'home', root / 'registry-cache', root / 'search-cache']:
    path.mkdir()
def write(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
def package(name, body):
    path = registry / 'org.fixture' / name / 'v1.0.0'
    write(path / 'vibe.toml', "[package]\ngroup='org.fixture'\nname='" + name + "'\nversion='1.0.0'\nkind='flow'\n[boot_snippet]\nsource='boot/rules.md'\ncategory='flow'\n" + body)
    write(path / 'boot/rules.md', '# ' + name + '\n' + name.upper() + '_MARKER\n')
package('leaf', '')
package('root', """
[requires.packages]
'org.fixture/leaf'={version='=1.0.0',optional=true}
[[configuration.features]]
id='on'
name='On'
[[configuration.features]]
id='off'
name='Off'
[[configuration.features]]
id='selected'
name='Selected'
effects=[{kind='dependency',target='org.fixture/leaf'}]
[[configuration.groups]]
mode='choice'
id='switch'
name='Switch'
cardinality='exactly-one'
members=['on','off']
[[configuration.groups]]
mode='choice'
id='detail'
name='Detail'
cardinality='exactly-one'
members=['selected']
guards=[{member='on'}]
""")
def consumer(mode):
    write(project / 'vibe.toml', "[project]\nname='rollback'\nversion='0.1.0'\n[requires.packages.'org.fixture/root']\nversion='=1.0.0'\nlink='static-transitive'\nconfiguration={policy='require-selection',default_features=false,selections={switch={intent='exact',members=['" + mode + "']},detail={intent='exact',members=['selected']}}}\n[[registry]]\nname='fixture'\nurl='file://" + str(registry) + "'\n")
for name in ['AGENTS.md', 'CLAUDE.md', 'GEMINI.md']:
    write(project / name, '# Synthetic rollback fixture\n')
def sandbox(command, readonly_lock=False):
    cmd = ['/usr/bin/bwrap', '--unshare-all', '--die-with-parent', '--new-session', '--clearenv']
    for path in ['/usr', '/bin', '/lib', '/lib64']:
        cmd += ['--ro-bind', path, path]
    cmd += ['--dir', '/etc', '--ro-bind', '/etc/ld.so.cache', '/etc/ld.so.cache', '--proc', '/proc', '--dev', '/dev', '--tmpfs', '/tmp', '--bind', str(root), str(root), '--ro-bind', str(pathlib.Path(args.cli).absolute()), '/vibe']
    if readonly_lock:
        target = {'publication': project / 'vibe.lock', 'staging': project / 'vibevm/vibedeps/org.fixture.root/1.0.0', 'boot': project / 'vibevm/vibespecs/boot/STATIC.md'}[args.fault]
        cmd += ['--ro-bind', str(target), str(target)]
    for key, value in {'HOME': str(root / 'home'), 'PATH': '/usr/bin:/bin', 'VIBE_SETTINGS': str(settings), 'VIBE_REGISTRY_CACHE': str(root / 'registry-cache'), 'VIBEVM_SEARCH_CACHE_DIR': str(root / 'search-cache'), 'VIBE_NO_DEFAULT_REGISTRY': '1'}.items():
        cmd += ['--setenv', key, value]
    cmd += ['--chdir', str(project), '/vibe'] + command
    return subprocess.run(cmd, capture_output=True, timeout=90)
def snapshot():
    result = {}
    for path in sorted(project.rglob('*')):
        rel = str(path.relative_to(project))
        if rel == '.vibe' or rel.startswith('.vibe/'):
            continue  # coordination is reported separately, never package evidence
        result[rel] = ('directory', None) if path.is_dir() else ('file', hashlib.sha256(path.read_bytes()).hexdigest())
    return result
consumer('on')
setup = sandbox(['install', '--offline', '--assume-yes', '--json'])
assert setup.returncode == 0, setup.stderr.decode()
consumer('off')
before = snapshot()
lock_before = (project / 'vibe.lock').read_bytes()
failed = sandbox(['update', '--all', '--offline', '--assume-yes', '--json'], True)
(root / 'failed.stdout').write_bytes(failed.stdout)
(root / 'failed.stderr').write_bytes(failed.stderr)
assert failed.returncode != 0
assert (b'busy' in failed.stderr.lower() if args.fault == 'publication' else b'read-only' in failed.stderr.lower() or b'busy' in failed.stderr.lower()), failed.stderr.decode()
decoder = json.JSONDecoder()
text = failed.stdout.decode()
reports = []
while text.strip():
    value, end = decoder.raw_decode(text.lstrip())
    reports.append(value)
    text = text.lstrip()[end:]
report = next(value for value in reports if value.get('command') == 'update')
assert report['ok'] is False
row = report['retained_backups'][0]
assert row['state'] == 'restored_container_retained'
container = pathlib.Path(row['path'])
assert container.is_dir() and not (container / 'slot').exists()
assert (project / 'vibe.lock').read_bytes() == lock_before
assert (project / 'vibevm/vibedeps/org.fixture.leaf/1.0.0/vibe.toml').is_file()
after = snapshot()
relative = str(container.relative_to(project))
assert after.pop(relative) == ('directory', None)
legacy_backups = sorted(project.glob('vibevm/.vibe-install-rollback-*'))
if args.fault == 'staging':
    assert len(legacy_backups) == 1, 'mixed rollback must preserve the separately failed root backup'
    assert b'also failed' in failed.stderr
    legacy = legacy_backups[0]
    assert any(path.name == 'vibe.toml' for path in legacy.rglob('vibe.toml'))
    prefix = str(legacy.relative_to(project))
    after = {key: value for key, value in after.items() if key != prefix and not key.startswith(prefix + '/')}
else:
    assert not legacy_backups, 'clean rollback must not claim a failed legacy inverse'
assert row['slot'] not in report['pruned'], 'confirmed restored leaf cannot still be reported as removed'
assert after == before, 'rollback changed package/boot/lock bytes outside explicitly retained containers'
print(json.dumps({'gate': 'guarded-update-' + args.fault + '-rollback', 'status': 'PASS', 'exit': failed.returncode, 'primary_error': failed.stderr.decode(), 'report': report, 'cli_sha256': hashlib.sha256(pathlib.Path(args.cli).read_bytes()).hexdigest(), 'coordination_paths': sorted(str(path.relative_to(project)) for path in (project / '.vibe').rglob('*')), 'root': str(root)}, indent=2))
