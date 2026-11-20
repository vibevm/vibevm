#!/usr/bin/env python3
"""Fresh isolated synthetic kit, real bootstrap/CLI, metadata-only PTY setup."""
import argparse
import fcntl
import hashlib
import io
import json
import os
from pathlib import Path
import pty
import select
import subprocess
import termios
import time
import zipfile


def digest(data):
    return 'sha256:' + hashlib.sha256(data).hexdigest()


def inside():
    root = Path('/world')
    kit = root / 'kit with spaces'
    kit.mkdir()
    home = root / 'home'
    home.mkdir()
    settings = root / 'settings'
    settings.mkdir(mode=0o700)
    destination = root / 'installed'
    env = {'HOME': str(home), 'PATH': '/usr/bin:/bin', 'SHELL': '/bin/sh',
           'VIBE_SETTINGS': str(settings), 'VIBEVM_INSTALL_ROOT': str(destination),
           'VIBE_OFFLINE': '1', 'VIBE_NO_DEFAULT_REGISTRY': '1', 'TERM': 'xterm'}
    source = io.BytesIO()
    with zipfile.ZipFile(source, 'w', zipfile.ZIP_DEFLATED) as z:
        z.writestr('Cargo.toml', '[workspace]\n')
        z.writestr('crates/vibe-cli/src/main.rs', 'fn main() {}\n')
    source = source.getvalue()
    components = [(name, Path('/input/' + filename).read_bytes()) for name, filename in
                  [('vibe', 'vibe'), ('vibe-app-proxy', 'vibe-app-proxy'), ('vibe-index', 'vibe-app-proxy')]]
    manifest = {'schema_version': 1, 'product': 'vibevm', 'repository': 'vibevm/vibevm',
                'version': '1.0.12', 'tag': 'v1.0.12', 'source_commit': '1' * 40,
                'target': 'x86_64-unknown-linux-gnu',
                'components': [dict(name=n, path=n, size=len(b), digest=digest(b)) for n, b in components],
                'source_archive': dict(path='vibevm-source.zip', size=len(source), digest=digest(source), tree_oid='2' * 40)}
    # Component placeholders are declared honestly as synthetic fixture bytes;
    # only the actual CLI and proxy are executed, never the index placeholder.
    bundle = kit / 'vibevm-1.0.12-x86_64-unknown-linux-gnu.zip'
    with zipfile.ZipFile(bundle, 'w', zipfile.ZIP_DEFLATED, compresslevel=1) as z:
        for name, data in components:
            item = zipfile.ZipInfo(name)
            item.external_attr = 0o100755 << 16
            item.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(item, data)
        z.writestr('DISTRIBUTION.json', json.dumps(manifest))
        z.writestr('vibevm-source.zip', source)
        z.writestr('LICENSE.md', 'Synthetic fixture; not a release\n')
        z.writestr('README.md', 'Synthetic setup adapter qualification\n')
    (kit / 'SHA256SUMS.txt').write_text(hashlib.sha256(bundle.read_bytes()).hexdigest() + '  ' + bundle.name + '\n')
    profile = kit / 'profile-linux-auto.toml'
    profile.write_text(f'schema=1\nname="synthetic adapter fixture"\n[self]\ninstall_root="{destination}"\nsettings_dir="{settings}"\n[path]\nfix="never"\n[proxy]\nmode="none"\n')
    steps = []

    def run(label, args, executable='/input/installer', expect=0, data=None):
        result = subprocess.run([executable, *args], input=data, capture_output=True, env=env, cwd='/tmp', timeout=90)
        row = dict(label=label, args=args, exit=result.returncode,
                   stdout=result.stdout.decode(errors='replace'), stderr=result.stderr.decode(errors='replace'))
        steps.append(row)
        (root / 'steps.json').write_text(json.dumps(steps, indent=2) + '\n')
        assert result.returncode == expect, row
        return result

    common = ['--kit', str(kit), '--no-path']
    run('dry-opt-in-no-setup', common + ['--configure-ai', '--unattended', '--dry-run'])
    assert not destination.exists()
    first = run('fresh-opt-in-unattended', common + ['--configure-ai', '--unattended'])
    assert json.loads(first.stdout)['optional_setup']['status'] == 'input_required'
    old_route = run('repeat-no-flag', common + ['--unattended'])
    assert b'vibe-installer-optional-setup' not in old_route.stdout
    current = Path((destination / 'opt/vibevm/current').read_text().strip())
    installed = current / 'bin/vibe'
    assert installed.is_file() and digest(installed.read_bytes()) == manifest['components'][0]['digest']
    baseline = {n: hashlib.sha256((current / 'bin' / n).read_bytes()).hexdigest() for n, _ in components}
    home_before = {p.name: p.read_bytes() for p in home.iterdir() if p.is_file()}
    assert all(b'PATH=' not in b and b'/opt/bin' not in b for b in home_before.values()), 'no-path changed PATH'
    foreign = settings / 'foreign'
    foreign.write_text('KEEP')
    repeat = run('repeat-opt-in-unattended', common + ['--configure-ai', '--unattended'])
    report = json.loads(repeat.stdout)
    assert report['installation'] == 'completed' and report['optional_setup']['status'] == 'input_required'
    assert not (settings / 'secrets/index.toml').exists() and not (settings / 'secrets/ai/index.toml').exists()
    # Actual source-shaped executable has a different running identity from active.
    source_root = root / 'source-launch'
    source_executable = source_root / 'target/debug/vibe'
    source_executable.parent.mkdir(parents=True)
    (source_root / 'Cargo.toml').write_text('[workspace]\n')
    (source_root / 'crates/vibe-cli').mkdir(parents=True)
    source_executable.write_bytes(Path('/input/vibe').read_bytes())
    source_executable.chmod(0o755)
    ordinary = run('source-launched-default-query', ['self', 'which', 'vibe', '--json'], str(source_executable))
    assert json.loads(ordinary.stdout)['path'] == str(source_executable)
    assert json.loads(ordinary.stdout)['selector'] is None
    active = run('source-launched-active-query', ['self', 'which', 'vibe', '--active', '--json'], str(source_executable))
    assert json.loads(active.stdout)['path'] == str(installed)
    assert json.loads(active.stdout)['selector'] is not None
    no_active_env = env.copy()
    no_active_env['VIBEVM_INSTALL_ROOT'] = str(root / 'empty-active')
    empty = subprocess.run(['/input/vibe', 'self', 'which', 'vibe', '--active', '--json'], capture_output=True, env=no_active_env)
    assert empty.returncode != 0 and b'"ok": true' not in empty.stdout
    steps.append(dict(label='empty-active-refuses', exit=empty.returncode, stdout=empty.stdout.decode(), stderr=empty.stderr.decode()))

    # Synthetic secret creation is test setup only. Setup itself never resolves
    # the value; values are not printed, logged or copied into the kit.
    secret = run('seed-synthetic-secret', ['secrets', 'create', '--kind', 'api-key', '--label', 'Fixture', '--value-stdin', '--json'], str(installed), data=b'SYNTHETIC_ADAPTER_ONLY')
    sid = json.loads(secret.stdout)['items'][0]['id']
    connection = run('seed-synthetic-reference', ['ai', 'connections', 'add', '--label', 'Fixture', '--endpoint', 'http://127.0.0.1:32123/v1', '--api-kind', 'openai-chat-completions', '--default-model', 'fixture', '--secret-ref', sid, '--json'], str(installed))
    result = json.loads(connection.stdout)
    cid = next(iter(result['items']))
    run('seed-default-reference', ['ai', 'connections', 'default', 'set', cid, '--expect-store', str(result['store_revision']), '--json'], str(installed))
    ai = settings / 'secrets/ai/index.toml'
    secret_index = settings / 'secrets/index.toml'
    secret_before = secret_index.read_bytes()

    def journey(label):
        before = ai.read_bytes()
        master, slave = pty.openpty()
        original = termios.tcgetattr(slave)

        def session():
            os.setsid()
            fcntl.ioctl(0, termios.TIOCSCTTY, 0)

        child = subprocess.Popen(['/input/installer', *common, '--configure-ai'], stdin=slave, stdout=slave, stderr=slave,
                                 env=env, cwd='/tmp', preexec_fn=session)
        data = b''
        stage = 0
        answered = False
        last = time.monotonic()
        deadline = last + 75
        while time.monotonic() < deadline:
            if select.select([master], [], [], 0.03)[0]:
                try:
                    data += os.read(master, 65536)
                except OSError:
                    break
            now = time.monotonic()
            if stage == 0 and b'Configure existing connection metadata only' in data and not termios.tcgetattr(slave)[3] & termios.ECHO:
                os.write(master, b'\x1b' if label == 'cancel' else b' ')
                stage = 1
                last = now
            elif label != 'cancel' and stage == 1 and now - last > 0.2:
                os.write(master, b'\r')
                stage = 2
            if label != 'cancel' and not answered and b'Apply exactly this metadata selection' in data:
                os.write(master, b'y\r' if label == 'apply' else b'\r')
                answered = True
            if child.poll() is not None:
                break
        if child.poll() is None:
            child.kill()
            child.wait()
            (root / f'{label}.terminal').write_bytes(data)
            raise AssertionError('test-owned setup child timed out')
        restored = termios.tcgetattr(slave) == original
        os.close(master)
        os.close(slave)
        (root / f'{label}.terminal').write_bytes(data)
        assert child.returncode == 0 and restored and stage > 0
        expected_status = b'failed' if label == 'failed' else b'completed_unconfirmed'
        assert expected_status in data and b'"installation":"completed"' in data
        assert secret_index.read_bytes() == secret_before
        assert (ai.read_bytes() != before) == (label == 'apply')
        assert all(hashlib.sha256((current / 'bin' / n).read_bytes()).hexdigest() == h for n, h in baseline.items())
        assert foreign.read_text() == 'KEEP'
        return dict(label=label, exit=child.returncode, terminal_restored=restored, actual_menu=True,
                    confirmation_seen=answered, ai_changed=ai.read_bytes() != before)

    journeys = [journey(label) for label in ['cancel', 'decline', 'apply']]
    binding = run('fresh-applied-binding-read', ['ai', 'bindings', 'show', 'pi', '--json'], str(installed))
    assert 'inherit-default' in binding.stdout.decode()
    selection = run('current-default-before-clear', ['ai', 'connections', 'default', 'show', '--json'], str(installed))
    revision = json.loads(selection.stdout)['store_revision']
    run('clear-default-for-optional-failure', ['ai', 'connections', 'default', 'clear', '--expect-store', str(revision), '--yes', '--json'], str(installed))
    journeys.append(journey('failed'))
    saved_profile = profile.read_bytes()
    mandatory_before = {n: hashlib.sha256((current / 'bin' / n).read_bytes()).hexdigest() for n, _ in components}
    ai_before = ai.read_bytes()
    profile.write_text('schema=1\nunknown_field=true\n')
    refused = run('mandatory-refusal-no-optional-child', common + ['--configure-ai', '--unattended'], expect=1)
    assert b'vibe-installer-optional-setup' not in refused.stdout
    assert ai.read_bytes() == ai_before
    assert all(hashlib.sha256((current / 'bin' / n).read_bytes()).hexdigest() == h for n, h in mandatory_before.items())
    profile.write_bytes(saved_profile)
    assert b'SYNTHETIC_ADAPTER_ONLY' not in (root / 'steps.json').read_bytes()
    assert {p.name: p.read_bytes() for p in home.iterdir() if p.is_file()} == home_before
    summary = dict(protocol='installer-optional-metadata-setup-qualification/1', journeys=journeys,
                   mandatory_bytes_preserved=True, secret_index_unchanged=True,
                   scope='synthetic kit with actual CLI; metadata-only; not real release or full onboarding')
    (root / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(json.dumps(summary, indent=2))


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--inside', action='store_true')
    p.add_argument('--installer')
    p.add_argument('--cli')
    p.add_argument('--proxy')
    p.add_argument('--output')
    a = p.parse_args()
    if a.inside:
        return inside()
    output = Path(a.output)
    output.mkdir(mode=0o700)
    command = ['/usr/bin/bwrap', '--unshare-all', '--die-with-parent', '--new-session', '--clearenv']
    for path in ['/usr', '/bin', '/lib', '/lib64']:
        command += ['--ro-bind', path, path]
    command += ['--dir', '/etc', '--ro-bind', '/etc/ld.so.cache', '/etc/ld.so.cache', '--proc', '/proc', '--dev', '/dev',
                '--tmpfs', '/tmp', '--bind', str(output.absolute()), '/world',
                '--ro-bind', str(Path(a.installer).absolute()), '/input/installer',
                '--ro-bind', str(Path(a.cli).absolute()), '/input/vibe',
                '--ro-bind', str(Path(a.proxy).absolute()), '/input/vibe-app-proxy',
                '--ro-bind', str(Path(__file__).absolute()), '/control.py', '--chdir', '/tmp',
                '/usr/bin/python3', '-I', '/control.py', '--inside']
    result = subprocess.run(command, capture_output=True, text=True, timeout=360)
    (output / 'outer.json').write_text(json.dumps(dict(exit=result.returncode, stdout=result.stdout, stderr=result.stderr), indent=2) + '\n')
    print(result.stdout)
    if result.returncode:
        print(result.stderr)
    raise SystemExit(result.returncode)


if __name__ == '__main__':
    main()
