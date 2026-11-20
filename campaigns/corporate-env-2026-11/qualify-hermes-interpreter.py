#!/usr/bin/env python3
"""Finite public prerequisite qualification; no supplier code or Python is executed."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import tarfile

p = argparse.ArgumentParser()
p.add_argument('--cli', required=True)
p.add_argument('--proxy', required=True)
p.add_argument('--hermes-package', required=True)
p.add_argument('--output', required=True)
a = p.parse_args()
root = Path(a.output)
root.mkdir(mode=0o700)  # Fresh evidence only; never overwrite a prior run.
cli, proxy, hermes = map(lambda s: Path(s).absolute(), (a.cli, a.proxy, a.hermes_package))
assert hashlib.sha256(cli.read_bytes()).hexdigest() == '2acac8641b59a4a811e583df9c029171c2be680d92397fb39ac66400b16be69e'
source_hashes = {}
for name, expected in [('vibe.toml', '69aa3de07b9c58d5014368976bf045e7b6438500a4605835a6fa145b247c127c'),
                       ('place-payload.sh', 'afdba183ce5a2bbe0ec845c91a15519d0719067849621ee5b2333930b11fb559')]:
    actual = hashlib.sha256((hermes / name).read_bytes()).hexdigest()
    assert actual == expected, f'accepted Hermes source drift: {name}'
    source_hashes[name] = actual
steps = []


def run(world, args, missing):
    cmd = ['/usr/bin/bwrap', '--unshare-all', '--die-with-parent', '--new-session', '--clearenv']
    for source in ['/usr', '/bin', '/lib', '/lib64']:
        cmd += ['--ro-bind', source, source]
    if missing:
        # Closed system search cannot find python3; no fake interpreter is supplied.
        cmd += ['--tmpfs', '/usr/bin', '--ro-bind', str(Path('/usr/bin/sh').resolve()), '/usr/bin/sh']
    cmd += ['--dir', '/etc', '--ro-bind', '/etc/ld.so.cache', '/etc/ld.so.cache',
            '--proc', '/proc', '--dev', '/dev', '--tmpfs', '/tmp',
            '--dir', '/home/probe', '--bind', str(world), '/world',
            '--ro-bind', str(cli), '/tools/vibe', '--ro-bind', str(proxy), '/tools/vibe-app-proxy',
            '--setenv', 'HOME', '/home/probe', '--setenv', 'PATH', '/usr/bin:/bin',
            '--setenv', 'VIBE_SETTINGS', '/world/settings with spaces',
            '--setenv', 'VIBE_REGISTRY_CACHE', '/world/cache',
            '--setenv', 'VIBEVM_SEARCH_CACHE_DIR', '/world/search-cache',
            '--setenv', 'VIBE_NO_DEFAULT_REGISTRY', '1', '--chdir', '/tmp', '/tools/vibe'] + args
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=60)
    steps.append(dict(world=world.name, args=args, exit=result.returncode, stdout=result.stdout, stderr=result.stderr))
    (root / 'steps.json').write_text(json.dumps(steps, indent=2) + '\n')
    return result


def absent_effects(world):
    settings = world / 'settings with spaces'
    for rel in ['applications/index.json', 'cache/app-work', 'opt/bin', 'opt/apps']:
        assert not (settings / rel).exists(), rel


for mode in ['missing-real-hermes', 'mapped-ownership-real-hermes', 'matched-synthetic-no-requirement']:
    world = root / mode
    package = world / 'registry/org.vibevm.agents/hermes/v0.23.2'
    package.mkdir(parents=True)
    (world / 'settings with spaces').mkdir()
    synthetic = mode == 'matched-synthetic-no-requirement'
    if not synthetic:
        for name in ['vibe.toml', 'place-payload.sh']:
            (package / name).write_bytes((hermes / name).read_bytes())
    else:
        payload = b'#!/bin/sh\nprintf "SYNTHETIC_ARCHIVE_CONTROL\\n"\n'
        archive = package / 'control.tar.gz'
        with tarfile.open(archive, 'w:gz') as out:
            info = tarfile.TarInfo('bin/control')
            info.mode, info.size, info.mtime = 0o755, len(payload), 0
            out.addfile(info, io.BytesIO(payload))
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        (package / 'vibe.toml').write_text(f'''[package]
group='org.vibevm.agents'
name='hermes'
kind='app'
version='0.23.2'
[application]
id='hermes-control'
[[installer_options]]
id='control'
name='Synthetic no-Python archive control'
platform='linux'
platform_name='Linux'
arch='x86_64'
[installer_options.installation]
type='tar_gz'
source='control.tar.gz'
sha256='{digest}'
[[installer_options.launchers]]
id='main'
name='Control'
kind='command'
executable='$app_bin/control'
default=true
[[installer_options.public_commands]]
name='hermes-control'
launcher='main'
''')
    args = ['install', '-g', 'org.vibevm.agents/hermes', '--registry', '/world/registry', '--offline', '--unattended', '--json']
    result = run(world, args, mode != 'mapped-ownership-real-hermes')
    if synthetic:
        assert result.returncode == 0, result.stderr
        receipt = json.loads((world / 'settings with spaces/applications/index.json').read_text())
        assert 'org.vibevm.agents/hermes@0.23.2' in receipt['installations']
        launched = run(world, ['app', 'run', 'org.vibevm.agents/hermes', 'main'], True)
        assert launched.returncode == 0 and 'SYNTHETIC_ARCHIVE_CONTROL' in launched.stdout, launched.stderr
        removed = run(world, ['uninstall', '-g', 'org.vibevm.agents/hermes', '--json'], True)
        assert removed.returncode == 0, removed.stderr
        assert json.loads((world / 'settings with spaces/applications/index.json').read_text())['installations'] == {}
        assert not (world / 'settings with spaces/opt/bin/hermes-control').exists()
    else:
        assert result.returncode == 2, result.stderr
        assert 'runtime_remediation_refused' in result.stderr, result.stderr
        expected = 'system Python missing at /usr/bin/python3' if mode.startswith('missing') else 'protected system-owned executable'
        assert expected in result.stderr, result.stderr
        absent_effects(world)
summary = dict(protocol='hermes-public-prerequisite-qualification/1', cli_sha256=hashlib.sha256(cli.read_bytes()).hexdigest(), source_sha256=source_hashes, proxy_sha256=hashlib.sha256(proxy.read_bytes()).hexdigest(),
               worlds=3, missing='PASS public refusal', ownership='PASS provenance refusal; NOT ABI evidence',
               matched_control='PASS synthetic archive install/run/uninstall without Python',
               incompatible_abi='UNRUN: namespace ownership refuses before capability probe',
               interpreter_drift='UNRUN: no admitted positive CLI interpreter witness',
               real_hermes_positive='UNRUN; no supplier executable or archive used', network='unshare-all; no provider/auth mounts')
(root / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
print(json.dumps(summary, indent=2))
